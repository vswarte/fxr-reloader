use protocol::PatchFxrError;
use detection::RunningGame;
use eldenring::EldenRingFxrPatcher;
use armoredcore6::ArmoredCore6FxrPatcher;

pub(crate) mod pattern;
pub(crate) mod detection;
pub(crate) mod eldenring;
pub(crate) mod armoredcore6;

pub(crate) fn make_patcher(game: RunningGame) -> Result<Box<dyn FxrPatcher>, PatchFxrError> {
    Ok(match game {
        RunningGame::EldenRing => Box::new(EldenRingFxrPatcher::new()?),
        // RunningGame::ArmoredCore6 => Box::new(ArmoredCore6FxrPatcher::new()?),
    })
}

pub(crate) trait FxrPatcher {
    fn patch(&self, fxr: Vec<u8>) -> Result<(), PatchFxrError>;
}
