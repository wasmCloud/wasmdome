#[macro_use]
extern crate log;

use std::{collections::HashMap, path::PathBuf};
use structopt::clap::AppSettings;
use structopt::StructOpt;
use wascc_host::{host, Actor, NativeCapability};


#[derive(Debug, StructOpt, Clone)]
#[structopt(
    global_settings(&[AppSettings::ColoredHelp, AppSettings::VersionlessSubcommands]),
    name = "match-coordinator", 
    about = "Hosts the match coordinator actor for WasmDome")]
struct Cli {
    #[structopt(flatten)]
    command: CliCommand,
}


#[derive(Debug, Clone, StructOpt)]
struct CliCommand {
    /// Path to the signed WebAssembly module responsible for match coordination
    #[structopt(short = "c", long = "coordinator", parse(from_os_str))]
    coordinator_path: PathBuf,    

    /// Path to the capability providers used by this host
    #[structopt(short = "p", long = "provider", parse(from_os_str))]
    provider_paths: Vec<PathBuf>,
}


fn handle_command(cmd: CliCommand) -> Result<(), Box<dyn ::std::error::Error>> {    
    host::add_actor(Actor::from_file(cmd.coordinator_path)?)?;    
    
    cmd.provider_paths.iter().for_each(|p| {
        host::add_native_capability(NativeCapability::from_file(p).unwrap()).unwrap();
    });

    host::configure(
        "MBM7CUPD6VOXKKKPLY23KXYFYG2AAU2M24X6BOME3VOVKMIOZILAVP5N",
        "wascc:messaging",
        generate_config("wasmdome.matches.create, wasmdome.match_events.*"),
    )?;
    host::configure(
        "MBM7CUPD6VOXKKKPLY23KXYFYG2AAU2M24X6BOME3VOVKMIOZILAVP5N",
        "wascc:keyvalue",
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
