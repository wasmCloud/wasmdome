extern crate wasmdome_domain as domain;
extern crate wasmdome_protocol as protocol;

use crossbeam_channel::unbounded;
use protocol::commands::{ArenaControlCommand::*, CreateMatch, MechQueryResponse};
use protocol::events::ArenaEvent;
use protocol::scheduler::StoredMatch;
use protocol::tools::{CredentialsRequest, CredentialsResponse};
use std::{error::Error, path::PathBuf, time::Duration};
use structopt::clap::AppSettings;
use structopt::StructOpt;
use uuid::Uuid;

#[macro_use]
extern crate prettytable;
use prettytable::Table;

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

    #[structopt(flatten)]
    action: WasmdomeAction,
}

#[derive(Debug, Clone, StructOpt)]
enum WasmdomeAction {
    /// Configure local credentials to compete in the arena by submitting an access token
    Compete {
        /// Your account public key
        account: String,

        /// Short-lived access token granted by the wasmdome.dev website
        token: String,
    },
    /// Query the schedule of upcoming matches
    Schedule,
    /// Run a wasmdome match using the local lattice
    Run {
        /// Maximum number of turns in the match
        #[structopt(short = "t", long = "max_turns")]
        max_turns: u32,

        /// Board height
        #[structopt(short = "h", long = "height")]
        board_height: u32,

        /// Board width
        #[structopt(short = "w", long = "width")]
        board_width: u32,
    },
}

fn handle_command(cmd: CliCommand) -> std::result::Result<(), Box<dyn ::std::error::Error>> {
    let nc = nats::connect("127.0.0.1")?; // Connect to the leaf node on loopback
    match cmd.action {
        WasmdomeAction::Schedule => check_schedule(nc)?,
        WasmdomeAction::Run {
            max_turns,
            board_height,
            board_width,
        } => run_match(nc, max_turns, board_height, board_width)?,
        WasmdomeAction::Compete { token, account } => change_compete_creds(nc, &account, &token)?,
    };
    Ok(())
}

fn change_compete_creds(
    nc: nats::Connection,
    account: &str,
    token: &str,
) -> Result<(), Box<dyn Error>> {
    let req = CredentialsRequest {
        account_key: account.to_string(),
        token: token.to_string(),
    };
    let res = nc.request_timeout(
        "wasmdome.public.creds.claim",
        &serde_json::to_vec(&req)?,
        Duration::from_millis(500),
    )?;
    write_arena_creds(serde_json::from_slice(&res.data)?)?;
    Ok(())
}

fn write_arena_creds(creds: CredentialsResponse) -> Result<(), Box<dyn Error>> {
    println!("{:?}", creds);
    let homedir = dirs::home_dir();
    if homedir.is_none() {
        return Err("Cannot locate home directory".into());
    }
    let domedir = homedir.unwrap().join(".wasmdome/");
    let _ = std::fs::create_dir_all(&domedir)?;
    if let CredentialsResponse::Valid {
        user_jwt,
        user_secret,
    } = creds
    {
        let fcontents = format!(
            r#"
-----BEGIN NATS USER JWT-----
{}
------END NATS USER JWT------
************************* IMPORTANT *************************
NKEY Seed printed below can be used to sign and prove identity.
NKEYs are sensitive and should be treated as secrets.
-----BEGIN USER NKEY SEED-----
{}
------END USER NKEY SEED------
*************************************************************
"#,
            user_jwt, user_secret
        );
        let domefile = domedir.join("arena.creds");
        std::fs::write(domefile, fcontents.as_bytes())?;
        println!("New arena credentials written to ~/.wasmdome/arena.creds");
        Ok(())
    } else {
        Err("Did not obtain valid arena credentials".into())
    }
}

fn check_schedule(nc: nats::Connection) -> Result<(), Box<dyn Error>> {
    let res = nc.request_timeout(
        "wasmdome.public.arena.schedule",
        "",
        std::time::Duration::from_millis(500),
    );

    if let Err(e) = res {
        println!("Error requesting schedule from the lattice: {}", e);
        return Ok(());
    };

    let data = &res.unwrap().data;
    let matches: Vec<StoredMatch> = serde_json::from_slice(data)?;
    let mut table = Table::new();
    if matches.len() > 0 {
        table.add_row(row![
            "Match Id",
            "Match Start",
            "Board Size (H ⨯ W)",
            "Max Actors",
            "Max Turns",
        ]);
        matches.iter().for_each(|m| {
            table.add_row(row![
                format!("{}", m.match_id),
                format!("{}", m.entry.match_start),
                format!("{} ⨯ {}", m.entry.board_height, m.entry.board_width),
                format!("{}", m.entry.max_actors),
                format!("{}", m.entry.max_turns),
            ]);
        });
    } else {
        println!("No schedule results available.");
        return Ok(());
    };

    table.printstd();
    Ok(())
}

fn run_match(
    nc: nats::Connection,
    max_turns: u32,
    board_height: u32,
    board_width: u32,
) -> Result<(), Box<dyn Error>> {
    let match_id = Uuid::new_v4().to_string();
    let sub = nc.subscribe("wasmdome.public.arena.events")?;

    let req = nc.request_timeout(
        "wasmdome.internal.arena.control",
        &serde_json::to_vec(&QueryMechs)?,
        std::time::Duration::from_millis(1500),
    );

    if req.is_err() {
        println!(
            "No response from engine-provider, please ensure you have an engine-provider running."
        );
        return Ok(());
    };

    let res: MechQueryResponse = serde_json::from_slice(&req.unwrap().data)?;

    if res.mechs.len() < 1 {
        println!(
            "No mechs were found in the lattice. Ensure you have scheduled at least one mech."
        );
        return Ok(());
    };

    let cm = StartMatch(CreateMatch {
        match_id: match_id.clone(),
        actors: res.mechs.iter().map(|m| m.id.clone()).collect(),
        board_height,
        board_width,
        max_turns,
        aps_per_turn: domain::state::APS_PER_TURN,
    });

    nc.request_timeout(
        "wasmdome.internal.arena.control",
        &serde_json::to_vec(&cm)?,
        std::time::Duration::from_millis(500),
    )?;

    let (s, r) = unbounded();

    sub.with_handler(move |msg| {
        let msg = serde_json::from_slice(&msg.data).unwrap();
        match msg {
            Some(ArenaEvent::MatchCompleted {
                match_id, cause, ..
            }) => {
                s.send(format!(
                    "Match \"{}\" completed.\nCause: {:?}",
                    match_id, cause
                ))
                .unwrap();
            }
            _ => (),
        };
        Ok(())
    });

    println!(
        "{}",
        r.recv_timeout(std::time::Duration::from_millis(5000))
            .unwrap_or("Timeout occurred retrieving game results.".to_string())
    );

    Ok(())
}
/*

*/

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
