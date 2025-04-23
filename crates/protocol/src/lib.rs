use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error, Serialize, Deserialize)]
pub enum PatchFxrError {
    #[error("Could not locate CSSfx singleton. {0}")]
    CSSfxSingletonMissing(#[from] LookupError),
    #[error("Could not locate CSSfx instance")]
    CSSfxInstanceMissing,
    #[error("Failed parsing the supplier FXR. Can't read FXR ID.")]
    InvalidFxr,
    #[error("Could not acquire game parameters. {0}")]
    GameDetectionError(#[from] GameDetectionError),
    #[error("Could not match pattern instructions. {0}")]
    InstructionPattern(String),
}

#[derive(Debug, Error, Serialize, Deserialize)]
pub enum LookupError {
    #[error("Singleton was not found.")]
    NotFound,
    #[error("Could init initialize the singleton map {0}.")]
    SingletonMapCreation(SingletonMapError),
}

#[derive(Debug, Error, Serialize, Deserialize)]
pub enum SingletonMapError {
    #[error("Error parsing pattern.")]
    Pattern,
    #[error("Failed to locate section {0} - {1}.")]
    Section(String, SectionLookupError),
    #[error("Failed to parse singleton name.")]
    MalformedName,
}

#[derive(Debug, Error, Serialize, Deserialize)]
pub enum SectionLookupError {
    #[error("Failed to locate game base.")]
    NoGameBase,
    #[error("Failed to locate game section.")]
    SectionNotFound,
}

#[derive(Debug, Error, Serialize, Deserialize)]
pub enum GameDetectionError {
    #[error("Failed acquiring the games module handle.")]
    NoMainModuleHandle,
    #[error("Failed acquiring PE resources.")]
    MissingPEResources,
    #[error("Failed acquiring PE version info.")]
    MissingPEVersionInfo,
    #[error("Failed acquiring PE language for strings.")]
    MissingPEStringsLanguage,
    #[error("Failed acquiring product name from PE header. Cannot determine what game we're running.")]
    MissingProductName,
    #[error("Did not recognize game for product name {0}.")]
    UnknownProductName(String),
    #[error("Failed to find the .text section.")]
    MissingTextSection,
}

pub const AGENT_DLL_NAME: &str = "fxr_reloader_agent";
