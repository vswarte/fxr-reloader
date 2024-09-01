use pattern::{
    match_instruction_pattern, GET_ALLOCATOR_PATTERN, PATCH_OFFSETS_PATTERN, WTF_FXR_PATTERN,
};

use protocol::PatchFxrError;

use crate::{
    game::FxrPatcher,
    singleton::{self, DLRFLocatable},
};

use super::pattern;

type FxrAllocatorGetter = unsafe extern "system" fn() -> usize;
type AllocateFxr = unsafe extern "system" fn(usize, usize, usize) -> usize;
type PatchFxrOffsets = unsafe extern "system" fn(usize, usize, usize) -> *const std::ffi::c_void;
type PrepareFxr = unsafe extern "system" fn(usize) -> *const std::ffi::c_void;

#[derive(Debug)]
pub struct EldenRingFxrPatcher {
    patch_fxr_offset: PatchFxrOffsets,
    prepare_fxr: PrepareFxr,
    fxr_allocator_getter: FxrAllocatorGetter,
}

impl EldenRingFxrPatcher {
    pub fn new() -> Result<Self, PatchFxrError> {
        let get_allocator =
            {
                let matched = match_instruction_pattern(GET_ALLOCATOR_PATTERN).ok_or(
                    PatchFxrError::InstructionPattern("get_allocator_call".to_string()),
                )?;

                let capture = matched.captures.first().unwrap();
                let offset =
                    i32::from_le_bytes(capture.bytes.as_slice().try_into().map_err(|_| {
                        PatchFxrError::InstructionPattern("get_allocator".to_string())
                    })?);

                let rip = capture.location + 4;

                // End me
                if offset.is_positive() {
                    rip + offset as usize
                } else {
                    rip - offset.unsigned_abs() as usize
                }
            } as usize;

        unsafe {
            Ok(Self {
                patch_fxr_offset: std::mem::transmute(
                    match_instruction_pattern(PATCH_OFFSETS_PATTERN)
                        .ok_or(PatchFxrError::InstructionPattern("patch_fxr".to_string()))?
                        .location,
                ),
                prepare_fxr: std::mem::transmute(
                    match_instruction_pattern(WTF_FXR_PATTERN)
                        .ok_or(PatchFxrError::InstructionPattern("wtf_fxr".to_string()))?
                        .location,
                ),
                fxr_allocator_getter: std::mem::transmute(get_allocator),
            })
        }
    }
}

impl FxrPatcher for EldenRingFxrPatcher {
    fn patch(&self, fxr_bytes: Vec<u8>) -> Result<(), PatchFxrError> {
        // TODO: use zerocopy to parse fxr instead?
        if fxr_bytes.len() < 0x10 {
            return Err(PatchFxrError::InvalidFxr);
        }

        // Retrieve FXR ID from the input bytes
        let fxr_id = u32::from_le_bytes(
            fxr_bytes[0xc..0x10]
                .try_into()
                .map_err(|_| PatchFxrError::InvalidFxr)?,
        );

        let sfx_imp = unsafe {
            &mut *singleton::get_instance::<CSSfx>()?.ok_or(PatchFxrError::CSSfxInstanceMissing)?
        };

        let fxr = sfx_imp
            .fxr_definition_iter()
            .filter_map(|f| unsafe { f.as_mut() })
            .find(|f| f.id == fxr_id);

        if let Some(fxr) = fxr {
            let allocator = unsafe { (self.fxr_allocator_getter)() };

            let allocate: AllocateFxr = unsafe {
                std::mem::transmute(
                    *((*(allocator as *const usize) + 0x50) as *const usize)
                )
            };

            let allocation = unsafe {
                allocate(allocator, fxr_bytes.len(), 0x10)
            };

            unsafe {
                std::ptr::copy_nonoverlapping(
                    fxr_bytes.as_ptr(),
                    allocation as *mut u8,
                    fxr_bytes.len(),
                );
            }

            unsafe {
                (self.patch_fxr_offset)(allocation, allocation, allocation);
                (self.prepare_fxr)(allocation);
            }

            unsafe {
                if let Some(wrapper) = fxr.fxr_wrapper.as_mut() {
                    wrapper.fxr = allocation;
                }
            }
        }

        Ok(())
    }
}

struct FxrDefinitionIterator {
    current: *mut FxrListNode,
}

impl Iterator for FxrDefinitionIterator {
    type Item = *mut FxrListNode;

    fn next(&mut self) -> Option<Self::Item> {
        let previous = unsafe { self.current.as_ref() }?;
        self.current = previous.next;

        let current = unsafe { self.current.as_ref() }?;
        if current.id == 0 {
            None
        } else {
            Some(self.current)
        }
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct FxrWrapper {
    fxr: usize,
    unk: u64,
}

#[repr(C)]
#[derive(Debug)]
struct FxrListNode {
    pub next: *mut FxrListNode,
    pub prev: *mut FxrListNode,
    pub id: u32,
    _pad14: u32,
    pub fxr_wrapper: *mut FxrWrapper,
}

#[repr(C)]
#[derive(Debug)]
struct FxrResourceContainer {
    pub allocator1: u64,
    pub scene_ctrl: u64,
    pub unk10: u64,
    pub allocator2: u64,
    pub fxr_list_head: *mut FxrListNode,
    pub resource_count: u64,
}

#[repr(C)]
#[derive(Debug)]
struct GXFfxGraphicsResourceManager {
    pub vftable: u64,
    pub unk: [u8; 0x158],
    pub resource_container: &'static mut FxrResourceContainer,
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
struct CSSfx {
    pub vftable: u64,
    pub unk: [u8; 0x58],
    pub scene_ctrl: &'static mut GXFfxSceneCtrl,
}

impl CSSfx {
    pub fn fxr_definition_iter(&mut self) -> FxrDefinitionIterator {
        FxrDefinitionIterator {
            current: self
                .scene_ctrl
                .graphics_resource_manager
                .resource_container
                .fxr_list_head,
        }
    }
}

impl DLRFLocatable for CSSfx {
    fn name() -> &'static str {
        "CSSfx"
    }
}
