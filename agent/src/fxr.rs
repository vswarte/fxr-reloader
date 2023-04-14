use std::ptr;
use std::mem;
use crate::util::{SymbolLookupError, get_module_handle};

// TODO: gotta AOB these at some point, maybe use the broadsword crate I've been preparing?
// Holds the SFX repository which manages all the FXR definitions
const OFFSET_SFX_IMP: usize = 0x3cfa618;
// ??? but it doesn't work without this
const OFFSET_WTF_FXR: usize = 0x20dea50;
// Fills in offsets with their pointers
const OFFSET_PATCH_OFFSETS: usize = 0x20b5940;
// Retrieves some allocator which we'll use to allocate the required mem for the FXR defs
const OFFSET_GET_FXR_ALLOCATOR: usize = 0x20713b0;

#[repr(C)]
#[derive(Debug)]
struct FXRRoot {
    pub magic: [u8; 4],
    pub pad04: [u8; 2],
    pub version: u16,
    pub unk08: u32,
    pub ffx_id: u32,
}

#[repr(C)]
#[derive(Debug)]
struct FXRWrapper {
    fxr: &'static mut FXRRoot,
    unk: u64,
}

#[repr(C)]
#[derive(Debug)]
struct FXRListNode {
    pub next: &'static mut FXRListNode,
    pub prev: u64,
    pub hash: u64,
    pub fxr_wrapper: &'static mut FXRWrapper,
}

#[repr(C)]
#[derive(Debug)]
struct FXRList {
    pub head: &'static mut FXRListNode,
}

#[repr(C)]
#[derive(Debug)]
struct FXRResourceContainer {
    pub allocator1: u64,
    pub scene_ctrl: u64,
    pub unk10: u64,
    pub allocator2: u64,
    pub fxr_list: &'static mut FXRList,
    pub resource_count: u64,
}

#[repr(C)]
#[derive(Debug)]
struct GXFfxGraphicsResourceManager {
    pub vftable: u64,
    pub unk: [u8; 0x158],
    pub resource_container: &'static mut FXRResourceContainer,
}

#[repr(C)]
#[derive(Debug)]
struct GXFfxSceneCtrl {
    pub vftable: u64,
    pub sg_entity: u64,
    pub allocator: u64,
    pub ffx_manager: u64,
    pub unk: u64,
    pub graphics_resource_manager: &'static mut GXFfxGraphicsResourceManager,
}

#[repr(C)]
#[derive(Debug)]
struct SfxImp {
    pub vftable: u64,
    pub unk: [u8; 0x58],
    pub scene_ctrl: &'static mut GXFfxSceneCtrl,
}

fn get_game_base() -> Result<usize, SymbolLookupError> {
    get_module_handle("eldenring.exe".to_string())
        .or_else(|_| get_module_handle("start_protected_game.exe".to_string()))
}

/// This function takes in the FXR file as a byte array, prepares it for use in-game by calling some
/// routines that are supplied by the game, and swaps out the old pointer to the FXR definition in
/// `CSSfxImp` with one to the definition that we prepared. This effectively causes the game to use
/// our own definition when a given sfx is spawned again.
pub(crate) unsafe fn patch_fxr_definition(input_fxr: Vec<u8>) {
    // Grab the 4 bytes that represent the sfx ID from the byte array and cast them to a uint 32.
    let fxr_id_bytes: [u8; 4] = input_fxr[0xC..0x10].try_into().unwrap();
    let supplied_fxr_id = u32::from_le_bytes(fxr_id_bytes);

    // Get the game base and build pointers to the functions and statics
    let game_base = get_game_base().unwrap();

    // The CSSfxImp seems to be a repository holding all of the FXR definitions indirectly
    let sfx_imp_ptr = game_base + OFFSET_SFX_IMP;
    let sfx_imp: &mut SfxImp = unsafe { &mut **(sfx_imp_ptr as *const *mut SfxImp) };
    let fxr_list = &sfx_imp.scene_ctrl.graphics_resource_manager.resource_container.fxr_list;
    let fxr_list_ptr = *(*fxr_list as *const FXRList as *const usize);


    // Traverse the FXRList
    // TODO: this could be turned into an iterator abstraction as more repositories in the game use a similar layout.
    let mut current_node = &fxr_list.head;
    loop {
        let current_node_ptr = *(*current_node as *const FXRListNode as *const usize);

        // At the end of the list the next node points to the fucken list itself
        if current_node_ptr == fxr_list_ptr {
            break;
        }

        // Skip over entries that don't have a definition associated with them
        if current_node.fxr_wrapper as *const _ as usize == 0x0 {
            continue;
        }

        // Loop until we find the right definition
        // TODO: can probably deref fxr_allocator_fn, fxr_alloc, patch_fxr_offsets and wtf_fxr once instead of every swap
        if current_node.fxr_wrapper.fxr.ffx_id == supplied_fxr_id {
            // Grabs the FXR-specific allocator. Unsure if the FXR defs are freed at all so might not need an allocator object from the game itself
            let fxr_allocator = (mem::transmute::<usize, unsafe extern "system" fn() -> usize>(game_base + OFFSET_GET_FXR_ALLOCATOR))();

            // We need to dig into the vftable of the allocator to get to the actual alloc method.
            // The alloc function is the 11th (vftable[10]) entry in the vftable.
            // TODO: can probably represent the vftable using a struct
            let fxr_allocator_fn = *((*(fxr_allocator as *const usize) + 0x50) as *const usize);

            // RCX holds a pointer to the allocator itself
            // RDX holds the size of the allocation
            // R8 holds the alignment of the memory (fixed 0x10)
            // RAX holds a pointer to the allocated memory
            let fxr_alloc = mem::transmute::<usize, unsafe extern "system" fn(usize, usize, usize) -> usize>(fxr_allocator_fn);
            let alloc = fxr_alloc(fxr_allocator, input_fxr.len(), 0x10);

            // Copy the received FXR definition into the allocated space
            ptr::copy_nonoverlapping(input_fxr.as_ptr(), alloc as *mut u8, input_fxr.len());

            // This fn seemingly replaces the FXR def offsets for in-memory pointers. Oddly it takes
            // 3 identical copies of the FXR data.
            let patch_fxr_offsets = mem::transmute::<usize, unsafe extern "system" fn(usize, usize, usize) -> *const ()>(game_base + OFFSET_PATCH_OFFSETS);

            // RCX, RDX and R8 all hold the same pointer to the new FXR def for some reason
            patch_fxr_offsets(alloc, alloc, alloc);

            // Do something with the new FXR def? Crashes happen if I don't call this
            let wtf_fxr = mem::transmute::<usize, unsafe extern "system" fn(usize) -> *const ()>(game_base + OFFSET_WTF_FXR);
            wtf_fxr(alloc);

            // Do some rather questionable casting because nobody ever intended for this code to exist
            let current_fxr_ptr = current_node.fxr_wrapper as *const FXRWrapper as *mut FXRWrapper as *mut usize;
            let alloc_ptr_ptr = &alloc as *const usize;

            // Swap out the pointer definition
            ptr::copy_nonoverlapping(alloc_ptr_ptr, current_fxr_ptr, 1);

            break;
        }

        current_node = &current_node.next;
    }
}