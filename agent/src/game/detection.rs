use std::ops::Range;

use pelite::pe::Pe;
use pelite::pe::PeView;
use protocol::GameDetectionError;

pub const PRODUCT_NAME_ELDENRING: &str = "ELDEN RING™";
// pub const PRODUCT_NAME_ARMOREDCORE6: &str = "ARMORED CORE™ VI FIRES OF RUBICON™";

#[derive(Debug)]
pub(crate) enum RunningGame {
    EldenRing,
    // ArmoredCore6,
}

/// Figures out what game we're currently running inside of.
pub(crate) fn detect_running_game() -> Result<RunningGame, GameDetectionError> {
    let header = unsafe {
        let handle = windows::Win32::System::LibraryLoader::GetModuleHandleA(std::ptr::null().into())
            .map_err(|_| GameDetectionError::NoMainModuleHandle)?;

        PeView::module(handle.0 as *const u8)
    };

    let _ = find_text_section(&header)?;
    let product_name = select_product_name(&header)?;

    std::fs::write("product-name.txt", &product_name).unwrap();

    Ok(match product_name.as_str() {
        PRODUCT_NAME_ELDENRING => RunningGame::EldenRing,
        // PRODUCT_NAME_ARMOREDCORE6 => RunningGame::ArmoredCore6,
        _ => return Err(GameDetectionError::UnknownProductName(product_name)),
    })
}

/// Attempts to capture the product name from the PE header.
fn select_product_name(
    header: &PeView,
) -> Result<String, GameDetectionError> {
    let resources = header.resources()
        .map_err(|_| GameDetectionError::MissingPEResources)?;
    let version_info = resources.version_info()
        .map_err(|_| GameDetectionError::MissingPEVersionInfo)?;
    let language = version_info.translation().first()
        .ok_or(GameDetectionError::MissingPEStringsLanguage)?;

    let mut product_name: Option<String> = None;
    version_info.strings(*language, |k,v| if k == "ProductName" {
        product_name = Some(v.to_string())
    });

    product_name.ok_or(GameDetectionError::MissingProductName)
}

/// Attempts to find the .text section
fn find_text_section(header: &PeView) -> Result<Range<u32>, GameDetectionError> {
    header.section_headers().iter()
        .find(|s| s.name_bytes() == b".text")
        .map(|s| s.virtual_range())
        .ok_or(GameDetectionError::MissingTextSection)
}
