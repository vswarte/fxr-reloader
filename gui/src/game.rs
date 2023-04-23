use std::fs;
use std::fmt;
use std::path::PathBuf;
use dll_syringe::{Syringe, process::OwnedProcess, error::InjectError};
use sysinfo::{ProcessExt, Pid, PidExt, System, SystemExt};

const AGENT_DLL_NAME: &str = "fxr_reloader_agent.dll";

/// Retrieves a list of running games that we should support
pub(crate) fn get_running_games() -> Vec<GameProcess> {
    let mut system = System::new();
    system.refresh_processes();

    system.processes()
        .iter()
        .map(|x| GameProcess { pid: *x.0, name: x.1.name().to_string() })
        .filter(|x| {
            x.name == "eldenring.exe" ||
            x.name == "sekiro.exe"
        })
        .collect::<Vec<GameProcess>>()
}

#[derive(Debug)]
pub(crate) enum GameProcessError {
    FindingProcessError,
    ModuleAcquisitionError(InjectError),
    InputFileReadError,
}

/// This function injects the agent DLL into the supplied process (if it's not in the process yet)
/// and calls the exposed `PatchFxr` function on it. We supply the selected FXR file's bytes to
/// `PatchFxr` when calling it. The Vec<u8> passed into `PatchFxr` is serialized with bincode
/// to avoid the unsafety around directly passing around `Vec<_>` across FFI barriers.
/// Once the `PatchFxr` method is done this function will eject the agent again.
pub(crate) fn call_fxr_patch(process: Pid, process_name: String, file: PathBuf) -> Result<(), GameProcessError> {
    let target_process = OwnedProcess::from_pid(process.as_u32())
        .map_err(|_| GameProcessError::FindingProcessError)?;

    // Obtain an instance of the agent DLL in the remote process
    let syringe = Syringe::for_process(target_process);
    let agent_module = syringe.find_or_inject(AGENT_DLL_NAME)
        .map_err(|x| GameProcessError::ModuleAcquisitionError(x))?;

    // Read the specified FXR file
    let file_contents = fs::read(file)
        .map_err(|_| GameProcessError::InputFileReadError)?;

    // Prepare a call to the agent DLL's patch function
    let remote_fn = unsafe {
        syringe.get_payload_procedure::<fn(String, Vec<u8>)>(agent_module, "PatchFxr")
    }.unwrap().unwrap();

    // Call the thing with the FXR contents
    remote_fn.call(process_name, &file_contents).unwrap();

    // Remove agent DLL from remote process memory again
    syringe.eject(agent_module).unwrap();

    Ok(())
}

#[derive(Debug, Clone, Eq)]
pub(crate) struct GameProcess {
    pub pid: Pid,
    pub name: String,
}

impl fmt::Display for GameProcess {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.name, self.pid)
    }
}

impl PartialEq for GameProcess {
    fn eq(&self, other: &Self) -> bool {
        self.pid == other.pid
    }
}
