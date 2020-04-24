#[macro_use]
extern crate log;

use wasmdome_domain as common;
// use common::commands::MechCommand;
use wasmdome_protocol as protocol;

use natsclient::*;
use protocol::events::MatchEvent;
use std::{collections::HashMap, path::PathBuf};
use structopt::clap::AppSettings;
use structopt::StructOpt;
use wascc_host::{Actor, NativeCapability, WasccHost};
use wascc_inmemory_streams::InmemoryStreamsProvider;
use wascc_keyvalue::KeyvalueProvider;
use wascc_logging::LoggingProvider;
use wascc_nats::NatsProvider;

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

fn preconfigure_host() -> std::result::Result<WasccHost, Box<dyn std::error::Error>> {
    let host = WasccHost::new();
    let nats = NatsProvider::new();
    let kv = KeyvalueProvider::new();
    let streams = InmemoryStreamsProvider::new();
    let logger = LoggingProvider::new();
    host.add_native_capability(NativeCapability::from_instance(nats, None)?)?;
    host.add_native_capability(NativeCapability::from_instance(kv, None)?)?;
    host.add_native_capability(NativeCapability::from_instance(streams, None)?)?;
    host.add_native_capability(NativeCapability::from_instance(logger, None)?)?;

    // Load
    // 1 - Command Processor (messaging + inmemory keyvalue + logging)
    // 2 - Match Coordinator (messaging + inmemory keyvalue + logging + extras)
    // 3 - Historian (messaging + streams + logging)
    host.add_actor(Actor::from_file("./command_processor_signed.wasm")?)?;
    host.bind_actor(
        "MA45FJLFLFWCAYQERFKYYOSRT522ZBOZ3RHFS5BNP5SDI4WIM3QAYBIN",
        "wascc:messaging",
        None,
        generate_nats_config("wasmdome.match_events.*"),
    )?;
    host.bind_actor(
        "MA45FJLFLFWCAYQERFKYYOSRT522ZBOZ3RHFS5BNP5SDI4WIM3QAYBIN",
        "wascc:keyvalue",
        None,
        HashMap::new(),
    )?;
    host.bind_actor(
        "MA45FJLFLFWCAYQERFKYYOSRT522ZBOZ3RHFS5BNP5SDI4WIM3QAYBIN",
        "wascc:logging",
        None,
        HashMap::new(),
    )?;

    host.add_actor(Actor::from_file("./match_coord_signed.wasm")?)?;
    host.bind_actor(
        "MAE56QXOS7IRMYLZ6IJV3PRT2UGWJMDORZSL7AQMX3KBDRU5OA3ZMUNL",
        "wascc:messaging",
        None,
        generate_nats_config("wasmdome.matches.create, wasmdome.match_events.*"),
    )?;
    host.bind_actor(
        "MAE56QXOS7IRMYLZ6IJV3PRT2UGWJMDORZSL7AQMX3KBDRU5OA3ZMUNL",
        "wascc:keyvalue",
        None,
        HashMap::new(),
    )?;
    host.bind_actor(
        "MAE56QXOS7IRMYLZ6IJV3PRT2UGWJMDORZSL7AQMX3KBDRU5OA3ZMUNL",
        "wascc:logging",
        None,
        HashMap::new(),
    )?;

    host.add_actor(Actor::from_file("./historian_signed.wasm")?)?;
    host.bind_actor(
        "MBO3DAWYCI7UVVTLM2CKB5GX7GYQ3MLYPUXLBQFBODIKZM4M5XXF2FW3",
        "wascc:messaging",
        None,
        generate_nats_config("wasmdome.history.replay,wasmdome.match_events.*"),
    )?;
    host.bind_actor(
        "MBO3DAWYCI7UVVTLM2CKB5GX7GYQ3MLYPUXLBQFBODIKZM4M5XXF2FW3",
        "wascc:eventstreams",
        None,
        HashMap::new(),
    )?;
    host.bind_actor(
        "MBO3DAWYCI7UVVTLM2CKB5GX7GYQ3MLYPUXLBQFBODIKZM4M5XXF2FW3",
        "wascc:logging",
        None,
        HashMap::new(),
    )?;

    Ok(host)
}

fn handle_command(cmd: CliCommand) -> std::result::Result<(), Box<dyn ::std::error::Error>> {
    if cmd.actor_paths.is_empty() {
        return Err("You need to specify at least one mech".into());
    }

    let valid_actors = cmd.actor_paths.iter().fold(
        HashMap::<String, std::path::PathBuf>::new(),
        |mut hm, path| {
            hm.insert(Actor::from_file(&path).unwrap().public_key(), path.clone());
            hm
        },
    );

    let host = preconfigure_host()?;

    let opts = ClientOptions::builder()
        .cluster_uris(vec!["nats://localhost:4222".into()])
        .authentication(AuthenticationStyle::Anonymous)
        .build()?;
    let client = Client::from_options(opts)?;
    match client.connect() {
        Ok(c) => c,
        Err(e) => {
            error!("Unable to connect to NATS. Is it running?\nUse `docker run -p 4222:4222 -p 6222:6222 -p 8222:8222 nats` to launch it in docker.");
            return Err(Box::new(e));
        }
    };

    let c = client.clone();
    client.subscribe("wasmdome.matches.*.scheduleactor", move |msg| {
        info!("About to unwrap");
        let schedule_req: protocol::commands::ScheduleActor =
            serde_json::from_slice(&msg.payload).unwrap();
        info!("Received actor schedule request [{:?}].", schedule_req);

        let (team, avatar, name) = if host.claims_for_actor(&schedule_req.actor).is_none() {
            info!("Starting new actor");
            let actor = Actor::from_file(valid_actors.get(&schedule_req.actor).unwrap()).unwrap();

            let t = get_team(&actor.tags());
            let a = get_avatar(&actor.tags());
            let name = actor.name();
            host.add_actor(actor).unwrap(); // TODO: kill unwrap
                                            // Mech actors subscribe to wasmdome.matches.{match}.turns.{actor}
            host.bind_actor(
                &schedule_req.actor,
                "wascc:messaging",
                None,
                generate_nats_config(&format!("wasmdome.matches.*.turns.{}", &schedule_req.actor)),
            )
            .unwrap();
            (t, a, name)
        } else {
            info!("Acknowledging start for existing actor");
            let claims = host.claims_for_actor(&schedule_req.actor).unwrap();
            (
                get_team(&claims.metadata.as_ref().unwrap().tags.as_ref().unwrap()),
                get_avatar(&claims.metadata.as_ref().unwrap().tags.as_ref().unwrap()),
                claims
                    .metadata
                    .as_ref()
                    .unwrap()
                    .name
                    .as_ref()
                    .map_or("Unnamed".to_string(), |n| n.to_string()),
            )
        };

        let scheduled = protocol::events::MatchEvent::ActorStarted {
            name,
            avatar: avatar,
            team: team,
            actor: schedule_req.actor.clone(),
            match_id: schedule_req.match_id.clone(),
        };
        c.publish(
            &format!("wasmdome.match_events.{}", schedule_req.match_id),
            &serde_json::to_vec(&scheduled).unwrap(),
            None,
        )
        .unwrap();
        Ok(())
    })?;

    use crossbeam_channel::unbounded;
    let (s, r) = unbounded();
    client.subscribe("wasmdome.match_events.*", move |msg| {
        let event: MatchEvent = serde_json::from_slice(&msg.payload).unwrap();

        if let MatchEvent::TurnEvent {
            turn_event: common::events::GameEvent::GameFinished { cause },
            ..
        } = event
        {
            println!("{:?}", cause);
            s.send(true).unwrap();
        }
        Ok(())
    })?;
    let _ = r.recv().unwrap();

    Ok(())
}

fn get_team(tags: &Vec<String>) -> String {
    if tags.contains(&"npc".to_string()) {
        "boylur".to_string()
    } else {
        "earth".to_string()
    }
}

fn get_avatar(tags: &Vec<String>) -> String {
    match tags.iter().find(|t| t.starts_with("avatar-")) {
        Some(t) => t.replace("avatar-", ""),
        None => "none".to_string(),
    }
}

fn generate_nats_config(sub: &str) -> HashMap<String, String> {
    let mut hm = HashMap::new();
    hm.insert("SUBSCRIPTION".to_string(), sub.to_string());
    hm.insert("URL".to_string(), "nats://localhost:4222".to_string());

    hm
}

fn main() -> std::result::Result<(), Box<dyn ::std::error::Error>> {
    let args = Cli::from_args();
    let cmd = args.command;
    let _ = env_logger::builder().format_module_path(false).try_init();

    match handle_command(cmd) {
        Ok(_) => {}
        Err(e) => {
            println!("Command line failure: {}", e);
        }
    }
    Ok(())
}
