extern crate wasmdome_protocol as protocol;

use protocol::tools::{CredentialsRequest, CredentialsResponse};
use std::{collections::HashMap, error::Error, path::PathBuf, time::Duration};
use structopt::clap::AppSettings;
use structopt::StructOpt;

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
}

fn handle_command(cmd: CliCommand) -> std::result::Result<(), Box<dyn ::std::error::Error>> {
    let nc = nats::connect("127.0.0.1")?; // Connect to the leaf node on loopback
    println!("{:?}", cmd);
    match cmd.action {
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
/*
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
