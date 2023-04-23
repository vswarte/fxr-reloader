mod fxr;
mod util;

/// Routine that is called from another process to make the DLL swap out FXR definitions.
dll_syringe::payload_procedure! {
    fn PatchFxr(process_name: String, fxr_bytes: Vec<u8>) {
        unsafe {
            fxr::patch_fxr_definition(process_name, fxr_bytes);
        }
    }
}