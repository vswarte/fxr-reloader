mod fxr;
mod util;

dll_syringe::payload_procedure! {
    fn PatchFxr(process_name: String, fxr_bytes: Vec<u8>) {
        unsafe {
            fxr::patch_fxr_definition(process_name, fxr_bytes);
        }
    }
}