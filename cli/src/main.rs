use std::error::Error;
use std::fs;
use std::path;

use clap::Parser;
use dll_syringe::{Syringe, process::OwnedProcess};

#[derive(Parser, Debug)]
#[command(name = "fxr-reloader-cli")]
#[command(author = "Chainfailure")]
#[command(version)]
/// Reload in-memory FXRs with the supplied FXRS. 
///
/// IMPORTANT:
/// This tool will not reload FXRs that are not in-memory and will not ensure 
/// that patched FXRs persist when the game refetches them from the BDTs 
/// itself.
struct Args {
    #[arg(short)]
    #[arg(long)]
    /// The process ID of a running Elden Ring instance
    process_id: u32,
    
    #[arg(short)]
    #[arg(long)]
    #[arg(required = true)]
    #[arg(num_args = 1..)]
    /// The FXR files to be reloaded
    fxrs: Vec<path::PathBuf>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    let target_process = OwnedProcess::from_pid(args.process_id)?;

    // Obtain an instance of the agent DLL in the remote process
    let syringe = Syringe::for_process(target_process);
    let agent_module = syringe.find_or_inject(protocol::AGENT_DLL_NAME)?;

    let remote_fn = unsafe {
        syringe.get_payload_procedure::<fn(Vec<Vec<u8>>)>(agent_module, "PatchFxr")
    }.unwrap().unwrap();

    let file_contents = args.fxrs.iter()
        .map(fs::read)
        .collect::<Result<Vec<_>, _>>()?;

    remote_fn.call(&file_contents)?;

    // Remove agent DLL from remote process memory again
    syringe.eject(agent_module)?;

    Ok(())
}
