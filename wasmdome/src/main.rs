#[macro_use]
extern crate log;

use wasmdome_domain as common;

use common::commands::MechCommand;
use wasmdome_protocol as protocol;

use natsclient::*;
use std::path::PathBuf;
use structopt::clap::AppSettings;
use structopt::StructOpt;
use wascc_host::{host, Actor, NativeCapability};

#[derive(Debug, StructOpt, Clone)]
#[structopt(
    global_settings(&[AppSettings::ColoredHelp, AppSettings::VersionlessSubcommands]),
    name = "wasmdome", 
    about = "An offline test environment for Assembly Mechs: Beyond WasmDome")]
struct Cli {
    #[structopt(flatten)]
    command: CliCommand,
}

#[derive(Debug, Clone, StructOpt)]
struct CliCommand {
    /// Path to the mechs (actors) used by wasmdome
    #[structopt(short = "m", long = "mech", parse(from_os_str))]
    actor_paths: Vec<PathBuf>,
}

fn preconfigure_host() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // Load 
    // 1 - real NATS
    // 2 - in-memory K/V
    // 3 - in-memory streams
    host::add_native_capability(NativeCapability::from_file("./libnats_provider.so")?)?;
    host::add_native_capability(NativeCapability::from_file("./libkeyvalue.so")?)?;
    host::add_native_capability(NativeCapability::from_file("./libtest_streams_provider.so")?)?;

    host::add_actor(Actor::from_file("./command_processor.wasm")?)?;
    host::add_actor(Actor::from_file("./match_coord.wasm")?)?;
    host::add_actor(Actor::from_file("./historian.wasm")?)?;
    Ok(())
}

fn handle_command(cmd: CliCommand) -> std::result::Result<(), Box<dyn ::std::error::Error>> {
    if cmd.actor_paths.is_empty() {
        return Err("You need to specify at least one mech".into());
    }
    let opts = ClientOptions::builder()
        .cluster_uris(vec!["nats://localhost:4222".into()])
        .authentication(AuthenticationStyle::Anonymous)
        .build()?;
    let client = Client::from_options(opts)?;
    let client = match client.connect() {
        Ok(c) => c,
        Err(e) => {
            error!("Unable to connect to NATS. Is it running?\nUse `docker run -p 4222:4222 -p 6222:6222 -p 8222:8222 nats` to launch it in docker.");
            return Err(Box::new(e));
        }
    };
    preconfigure_host()?;
    let c = client.clone();
    Ok(())
}

fn main() -> std::result::Result<(), Box<dyn ::std::error::Error>> {
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
