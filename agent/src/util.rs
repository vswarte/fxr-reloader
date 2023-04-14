use windows::core::PCWSTR;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;

// TODO: can most likely use a crate for this
fn string_to_pcwstr(input: String) -> PCWSTR {
    PCWSTR::from_raw([
        input.encode_utf16().collect::<Vec<u16>>(),
        vec![0x0 as u16]
    ].concat().as_ptr())
}

#[derive(Debug)]
pub enum SymbolLookupError {
    ModuleNotFound,
}

/// Finds the module's base address. Used to calculate the offsets to specific offsets in the game's
/// memory image.
pub(crate) fn get_module_handle(module: String) -> Result<usize, SymbolLookupError> {
    unsafe {
        GetModuleHandleW(string_to_pcwstr(module))
            .map_err(|_| SymbolLookupError::ModuleNotFound)
            .map(|x| x.0 as usize)
    }
}
