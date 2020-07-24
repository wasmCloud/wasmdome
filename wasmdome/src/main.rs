extern crate wasmdome_protocol as protocol;

use protocol::scheduler::StoredMatch;
use protocol::tools::{CredentialsRequest, CredentialsResponse};
use std::{error::Error, path::PathBuf, time::Duration};
use structopt::clap::AppSettings;
use structopt::StructOpt;
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
        #[structopt(short = "a", long = "account")]
        account: String,

        /// Short-lived access token granted by the wasmdome.dev website
        #[structopt(short = "t", long = "token")]
        token: String,
    },
    /// Query the schedule of upcoming matches
    Schedule {},
    /// Run a wasmdome match using the local lattice
    Run {
        /// Maximum number of turns in the match
        #[structopt(short = "t", long = "max_turns")]
        max_turns: u32,

        /// Maximum number of actors in the match
        #[structopt(short = "a", long = "max_actors")]
        max_actors: u32,

        /// Board height
        #[structopt(short = "h", long = "height")]
        height: u32,

        /// Board width
        #[structopt(short = "w", long = "width")]
        width: u32,
    },
}

fn handle_command(cmd: CliCommand) -> std::result::Result<(), Box<dyn ::std::error::Error>> {
    let nc = nats::connect("127.0.0.1")?; // Connect to the leaf node on loopback
    match cmd.action {
        WasmdomeAction::Compete { token, account } => change_compete_creds(nc, &account, &token)?,
        WasmdomeAction::Schedule { .. } => check_schedule(nc)?,
        WasmdomeAction::Run {
            max_turns,
            max_actors,
            height,
            width,
        } => run_match(nc, max_turns, max_actors, height, width)?,
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
            "Actions per turn"
        ]);
        matches.iter().for_each(|m| {
            table.add_row(row![
                format!("{}", m.match_id),
                format!("{}", m.entry.match_start),
                format!("{} ⨯ {}", m.entry.board_height, m.entry.board_width),
                format!("{}", m.entry.max_actors),
                format!("{}", m.entry.max_turns),
                format!("{}", m.aps_per_turn),
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
    max_actors: u32,
    height: u32,
    width: u32,
) -> Result<(), Box<dyn Error>> {
    // publish game start command onto local lattice (through nc)
    // let res = nc.request_timeout(
    //     "wasmdome.public.arena.schedule",
    //     "",
    //     std::time::Duration::from_millis(500),
    // );
    // subscribe to arena events topic and then display match conclusion event, or timeout with an error

    // ensure engine-provider is running
    // ensure at least 1 mech is scheduled in the match
    // wasmdome run -t 100 -a 20 -h 8 -w 8, turn limit, actors, height, width
    // when do we start the match?
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
