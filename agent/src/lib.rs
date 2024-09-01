use protocol::PatchFxrError;

mod game;
mod fxr;
mod singleton;

dll_syringe::payload_procedure! {
    fn PatchFxr(fxrs: Vec<Vec<u8>>) -> Result<(), PatchFxrError> {
        let game = game::detection::detect_running_game()?;
        let patcher = game::make_patcher(game)?;

        fxrs.into_iter().try_for_each(|f| patcher.patch(f))
    }
}
