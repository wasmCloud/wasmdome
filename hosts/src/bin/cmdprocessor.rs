extern crate log;

use std::{collections::HashMap, path::PathBuf};
use structopt::clap::AppSettings;
use structopt::StructOpt;
use wascc_host::{Actor, NativeCapability, WasccHost};

#[derive(Debug, StructOpt, Clone)]
#[structopt(
    global_settings(&[AppSettings::ColoredHelp, AppSettings::VersionlessSubcommands]),
    name = "cmdprocessor", 
    about = "Processes Mech Turn Commands, Emits Match Events")]
struct Cli {
    #[structopt(flatten)]
    command: CliCommand,
}

#[derive(Debug, Clone, StructOpt)]
struct CliCommand {
    /// Path to the signed WebAssembly module responsible for command processing
    #[structopt(short = "c", long = "cmdproc", parse(from_os_str))]
    processor_path: PathBuf,

    /// Path to the capability providers used by this host
    #[structopt(short = "p", long = "provider", parse(from_os_str))]
    provider_paths: Vec<PathBuf>,
}

fn handle_command(cmd: CliCommand) -> Result<(), Box<dyn ::std::error::Error>> {
    let host = WasccHost::new();
    host.add_actor(Actor::from_file(cmd.processor_path)?)?;

    cmd.provider_paths.iter().for_each(|p| {
        host.add_native_capability(NativeCapability::from_file(p, None).unwrap())
            .unwrap();
    });

    // Listens for match events, processing `TurnRequested` events
    host.bind_actor(
        "MBCMWXKIR2YSPI3PTIKCF4KJ4XVBUJHVY3UFT4K75SAWS5BZ7VIPMSNZ",
        "wascc:messaging",
        None,
        generate_config("wasmdome.match_events.*"),
    )?;
    host.bind_actor(
        "MBCMWXKIR2YSPI3PTIKCF4KJ4XVBUJHVY3UFT4K75SAWS5BZ7VIPMSNZ",
        "wascc:keyvalue",
        None,
        redis_config(),
    )?;

    std::thread::park();
    Ok(())
}

fn main() -> Result<(), Box<dyn ::std::error::Error>> {
    let args = Cli::from_args();
    let cmd = args.command;
    env_logger::init();

    match handle_command(cmd) {
        Ok(_) => {}
        Err(e) => {
            println!("Command line failure: {}", e);
        }
    }
    Ok(())
}

fn generate_config(sub: &str) -> HashMap<String, String> {
    let mut hm = HashMap::new();
    hm.insert("SUBSCRIPTION".to_string(), sub.to_string());
    hm.insert("URL".to_string(), "nats://localhost:4222".to_string());

    hm
}

fn redis_config() -> HashMap<String, String> {
    let mut hm = HashMap::new();
    hm.insert("URL".to_string(), "redis://127.0.0.1:6379".to_string());

    hm
}
