#[macro_use]
extern crate log;

use wasmdome_protocol as protocol;

use natsclient::*;
use std::{collections::HashMap, path::PathBuf};
use structopt::clap::AppSettings;
use structopt::StructOpt;
use wascc_host::{Actor, NativeCapability, WasccHost};

#[derive(Debug, StructOpt, Clone)]
#[structopt(
    global_settings(&[AppSettings::ColoredHelp, AppSettings::VersionlessSubcommands]),
    name = "mech-host", 
    about = "A waSCC host for scheduling mech actors")]
struct Cli {
    #[structopt(flatten)]
    command: CliCommand,
}

#[derive(Debug, Clone, StructOpt)]
struct CliCommand {
    /// Path to the capability providers used by this host
    #[structopt(short = "p", long = "provider", parse(from_os_str))]
    provider_paths: Vec<PathBuf>,
}

fn handle_command(cmd: CliCommand) -> std::result::Result<(), Box<dyn ::std::error::Error>> {
    let host = WasccHost::new();
    cmd.provider_paths.iter().for_each(|p| {
        host.add_native_capability(NativeCapability::from_file(p, None).unwrap())
            .unwrap();
    });

    let opts = ClientOptions::builder()
        .cluster_uris(vec!["nats://localhost:4222".into()])
        .authentication(AuthenticationStyle::Anonymous)
        .build()?;
    let client = Client::from_options(opts)?;
    client.connect()?;
    let c = client.clone();
    client.queue_subscribe(
        "wasmdome.matches.*.scheduleactor",
        "mech-host",
        move |msg| {
            let schedule_req: protocol::commands::ScheduleActor =
                serde_json::from_slice(&msg.payload).unwrap();
            info!("Received actor schedule request [{:?}].", schedule_req);

            let (team, avatar, name) = if host.claims_for_actor(&schedule_req.actor).is_none() {
                info!("Starting new actor");
                let actor = Actor::from_gantry(&schedule_req.actor).unwrap();

                let t = get_team(&actor.tags());
                let a = get_avatar(&actor.tags());
                let name = actor.name();
                host.add_actor(actor).unwrap(); // TODO: kill unwrap
                                                // Mech actors subscribe to wasmdome.matches.{match}.turns.{actor}
                host.bind_actor(
                    &schedule_req.actor,
                    "wascc:messaging",
                    None,
                    gen_config(&schedule_req.actor),
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
        },
    )?;

    std::thread::park();
    Ok(())
}

fn gen_config(actor: &str) -> HashMap<String, String> {
    let mut hm = HashMap::new();
    hm.insert(
        "SUBSCRIPTION".to_string(),
        format!("wasmdome.matches.*.turns.{}", actor),
    );
    hm.insert("URL".to_string(), "nats://localhost:4222".to_string());
    hm
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

fn main() -> std::result::Result<(), Box<dyn ::std::error::Error>> {
    let args = Cli::from_args();
    let cmd = args.command;
    match env_logger::try_init() {
        Ok(_) => {}
        Err(_) => {}
    };
    match handle_command(cmd) {
        Ok(_) => {}
        Err(e) => {
            println!("Command line failure: {}", e);
        }
    }
    Ok(())
}
