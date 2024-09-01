use std::fs;
use std::path::Path;
use dll_syringe::error::EjectError;
use dll_syringe::error::LoadProcedureError;
use dll_syringe::rpc::PayloadRpcError;
use thiserror::Error;
use dll_syringe::{Syringe, process::OwnedProcess, error::InjectError};
use protocol::PatchFxrError;
use sysinfo::{Pid, System};

const AGENT_DLL_NAME: &str = "fxr_reloader_agent.dll";

const SUPPORTED_GAMES: [&str; 4] = [
    "eldenring.exe",
    "armoredcore6.exe",
    "sekiro.exe",
    "start_protected_game.exe",
];

/// Retrieves a list of running games that we should support
pub(crate) fn get_running_games() -> Vec<GameProcess> {
    let mut system = System::new();
    system.refresh_all();

    let mut processes = system.processes()
        .iter()
        .map(|x| GameProcess {
            pid: *x.0,
            name: x.1.name().to_string_lossy().into_owned()
        })
        .filter(|p| SUPPORTED_GAMES.contains(&p.name.as_str()))
        .collect::<Vec<GameProcess>>();

    processes.sort_by(|a, b| b.pid.as_u32().cmp(&a.pid.as_u32()));

    processes
}

#[derive(Error, Debug)]
pub(crate) enum PatchError {
    #[error("Failed to find specified process.")]
    FindingProcess,
    #[error("Failed to locate agent module after injection. {0}")]
    ModuleAcquisition(#[from] InjectError),
    #[error("Failed to read input FXR file. {0}")]
    InputFileRead(#[from] std::io::Error),
    #[error("Failed to patch FXR definition. {0}")]
    Patch(#[from] PatchFxrError),
    #[error("Encountered error with the syringe payload. {0}")]
    Payload(#[from] PayloadRpcError),
    #[error("Encountered error with the syringe load procedure. {0}")]
    LoadProcedure(#[from] LoadProcedureError),
    #[error("Failed to locate the RPC function after injecting agent.")]
    MissingPatchFunction,
    #[error("Failed to eject agent module after usage. {0}")]
    Eject(#[from] EjectError),
}

/// This function injects the agent DLL into the supplied process (if it's not in the process yet)
/// and calls the exposed `PatchFxr` function on it. We supply the selected FXR file's bytes to
/// `PatchFxr` when calling it. The Vec<u8> passed into `PatchFxr` is serialized with bincode
/// to avoid the unsafety around directly passing around `Vec<_>` across FFI barriers.
/// Once the `PatchFxr` method is done this function will eject the agent again.
pub(crate) fn call_fxr_patch<P: AsRef<Path>>(
    process: Pid,
    files: &[P],
) -> Result<(), PatchError> {
    let target_process = OwnedProcess::from_pid(process.as_u32())
        .map_err(|_| PatchError::FindingProcess)?;

    // Obtain an instance of the agent DLL in the remote process
    let syringe = Syringe::for_process(target_process);
    let agent_module = syringe.find_or_inject(AGENT_DLL_NAME)?;

    // Read the specified FXR file
    let file_contents = files.iter()
        .map(fs::read)
        .collect::<Result<Vec<_>, _>>()?;

    // Prepare a call to the agent DLL's patch function
    let remote_fn = unsafe {
        syringe.get_payload_procedure::<fn(Vec<Vec<u8>>) -> Result<(), PatchFxrError>>(agent_module, "PatchFxr")
    }?.ok_or(PatchError::MissingPatchFunction)?;

    // Call the thing with the FXR contents
    remote_fn.call(&file_contents)??;

    // Remove agent DLL from remote process memory again
    syringe.eject(agent_module)?;

    Ok(())
}

#[derive(Debug, Clone, Eq)]
pub(crate) struct GameProcess {
    pub pid: Pid,
    pub name: String,
}

impl PartialEq for GameProcess {
    fn eq(&self, other: &Self) -> bool {
        self.pid == other.pid
    }
}
