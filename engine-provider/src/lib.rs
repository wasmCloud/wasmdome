#[macro_use]
extern crate wascc_codec as codec;

#[macro_use]
extern crate log;

use chrono::prelude::*;

mod store;

use store::MatchStore;

extern crate wasmdome_domain as domain;
extern crate wasmdome_protocol as protocol;

use codec::capabilities::{
    CapabilityDescriptor, CapabilityProvider, Dispatcher, NullDispatcher, OperationDirection,
    OP_GET_CAPABILITY_DESCRIPTOR,
};
use codec::core::{CapabilityConfiguration, OP_BIND_ACTOR, OP_REMOVE_ACTOR};
use codec::{deserialize, serialize};
use protocol::commands::*;
use protocol::{events::ArenaEvent, OP_TAKE_TURN};

use domain::state::MatchState;
use std::error::Error;
use std::sync::{Arc, RwLock};

const SYSTEM_ACTOR: &str = "system";
const CAPABILITY_ID: &str = "wasmdome:engine";
const VERSION: &str = env!("CARGO_PKG_VERSION");
const REVISION: u32 = 0;

const LATTICE_HOST_KEY: &str = "LATTICE_HOST"; // env var name
const DEFAULT_LATTICE_HOST: &str = "127.0.0.1"; // default mode is anonymous via loopback
const LATTICE_CREDSFILE_KEY: &str = "LATTICE_CREDS_FILE";

#[cfg(not(feature = "static_plugin"))]
capability_provider!(WasmdomeEngineProvider, WasmdomeEngineProvider::new);

pub struct WasmdomeEngineProvider {
    nc: Arc<nats::Connection>,
    dispatcher: Arc<RwLock<Box<dyn Dispatcher>>>,
    store: Arc<RwLock<MatchStore>>,
}

impl Default for WasmdomeEngineProvider {
    fn default() -> Self {
        let _ = env_logger::try_init();

        WasmdomeEngineProvider {
            dispatcher: Arc::new(RwLock::new(Box::new(NullDispatcher::new()))),
            store: Arc::new(RwLock::new(MatchStore::new())),
            nc: Arc::new(get_connection()),
        }
    }
}

fn get_connection() -> nats::Connection {
    let host = get_env(LATTICE_HOST_KEY, DEFAULT_LATTICE_HOST);
    info!("Lattice Host: {}", host);
    let mut opts = if let Some(creds) = get_credsfile() {
        nats::ConnectionOptions::with_credentials(creds)
    } else {
        nats::ConnectionOptions::new()
    };
    opts = opts.with_name("waSCC Lattice");
    opts.connect(&host).unwrap()
}

fn get_credsfile() -> Option<String> {
    std::env::var(LATTICE_CREDSFILE_KEY).ok()
}

fn get_env(var: &str, default: &str) -> String {
    match std::env::var(var) {
        Ok(val) => {
            if val.is_empty() {
                default.to_string()
            } else {
                val.to_string()
            }
        }
        Err(_) => default.to_string(),
    }
}

impl WasmdomeEngineProvider {
    pub fn new() -> Self {
        Self::default()
    }

    fn configure(&self, config: CapabilityConfiguration) -> Result<Vec<u8>, Box<dyn Error>> {
        // Handle actor binding metadata here...
        // This is typically where you would establish a
        // client or connection to a resource on behalf of
        // an actor
        info!("Binding actor {}", config.module);
        self.store
            .write()
            .unwrap()
            .add_bound_actor(&config.module)?;

        self.nc
            .publish(
                &protocol::events::events_subject(None),
                serde_json::to_string(&ArenaEvent::MechConnected {
                    actor: config.module.to_string(),
                    time: Utc::now(),
                })
                .unwrap(),
            )
            .unwrap();

        Ok(vec![])
    }

    fn deconfigure(&self, config: CapabilityConfiguration) -> Result<Vec<u8>, Box<dyn Error>> {
        // Handle removal of resources claimed by an actor here

        self.store
            .write()
            .unwrap()
            .remove_bound_actor(&config.module)?;

        publish_disconnect_event(self.nc.clone(), &config.module);
        Ok(vec![])
    }

    // Capability providers must provide a descriptor to the host containing metadata and a list of supported operations
    fn get_descriptor(&self) -> Result<Vec<u8>, Box<dyn Error>> {
        Ok(serialize(
            CapabilityDescriptor::builder()
                .id(CAPABILITY_ID)
                .name("Assembly Mechs: Beyond Wasmdome Game Engine Provider")
                .long_description("A capability provider that exposes the core of the game engine")
                .version(VERSION)
                .revision(REVISION)
                .with_operation(
                    OP_TAKE_TURN,
                    OperationDirection::ToActor,
                    "Send to the actor to obtain the actor's action sequence for the given turn",
                ) // TODO: make the operation descriptors match your real interface
                .build(),
        )?)
    }
}

fn remove_noshows(orig: &Vec<String>, healthy: &Vec<String>) -> Vec<String> {
    let mut filtered = orig.clone();
    filtered.retain(|s| healthy.contains(s));
    filtered
}

impl CapabilityProvider for WasmdomeEngineProvider {
    // Invoked by the runtime host to give this provider plugin the ability to communicate
    // with actors
    fn configure_dispatch(&self, dispatcher: Box<dyn Dispatcher>) -> Result<(), Box<dyn Error>> {
        trace!("Dispatcher received.");
        let mut lock = self.dispatcher.write().unwrap();
        *lock = dispatcher;

        spawn_health_check(self.nc.clone(), self.dispatcher.clone(), self.store.clone());
        let (nc, dp, sto) = (self.nc.clone(), self.dispatcher.clone(), self.store.clone());
        let _h = self
            .nc
            .subscribe(&protocol::events::arena_control_subject())?
            .with_handler(move |msg| {
                let ac: ArenaControlCommand = serde_json::from_slice(&msg.data).unwrap();
                handle_control_command(ac, nc.clone(), dp.clone(), sto.clone());
                Ok(())
            });

        Ok(())
    }

    // Invoked by host runtime to allow an actor to make use of the capability
    // All providers MUST handle the "configure" message, even if no work will be done
    fn handle_call(&self, actor: &str, op: &str, msg: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
        trace!("Received host call from {}, operation - {}", actor, op);

        match op {
            OP_BIND_ACTOR if actor == SYSTEM_ACTOR => self.configure(deserialize(msg)?),
            OP_REMOVE_ACTOR if actor == SYSTEM_ACTOR => self.deconfigure(deserialize(msg)?),
            OP_GET_CAPABILITY_DESCRIPTOR if actor == SYSTEM_ACTOR => self.get_descriptor(),
            //  OP_START_MATCH => self.start_match(deserialize(msg)?),
            _ => Err("bad dispatch".into()),
        }
    }
}

fn handle_control_command(
    ac: ArenaControlCommand,
    nc: Arc<nats::Connection>,
    dispatcher: Arc<RwLock<Box<dyn Dispatcher>>>,
    store: Arc<RwLock<MatchStore>>,
) {
    use ArenaControlCommand::*;
    match ac {
        StartMatch(cm) => match start_match(cm, store, dispatcher, nc) {
            Ok(_) => {}
            Err(e) => {
                error!("Failed to start match: {}", e);
            }
        },
    };
}

fn start_match(
    createmsg: CreateMatch,
    store: Arc<RwLock<MatchStore>>,
    dispatcher: Arc<RwLock<Box<dyn Dispatcher>>>,
    nc: Arc<nats::Connection>,
) -> Result<Vec<u8>, Box<dyn Error>> {
    // ensure that right before we start the match, we're confident all mechs have responded to a health request
    perform_health_check(store.clone(), dispatcher.clone(), nc.clone());
    use domain::MatchParameters;

    let params = MatchParameters::new(
        createmsg.match_id.clone(),
        createmsg.board_width,
        createmsg.board_height,
        createmsg.max_turns,
        createmsg.aps_per_turn,
        remove_noshows(&createmsg.actors, &store.write().unwrap().bound_actors()?), // use this instead of the match params list because this one's filtered by healthy
    );
    let mut state = MatchState::new_with_parameters(params.clone());
    state = spawn_mechs(nc.clone(), state, store.write().unwrap().bound_actors()?);
    store
        .write()
        .unwrap()
        .save_match_state(&createmsg.match_id, state)?;

    nc.publish(
        &protocol::events::events_subject(None),
        &serde_json::to_string(&ArenaEvent::MatchStarted {
            match_id: createmsg.match_id.clone(),
            actors: params.actors.clone(),
            board_height: params.height,
            board_width: params.width,
            start_time: Utc::now(),
        })?,
    )?;

    manage_match(
        nc.clone(),
        dispatcher.clone(),
        store.clone(),
        createmsg.actors.clone(),
        createmsg.match_id.clone(),
    );
    Ok(vec![])
}

mod game_logic;
use game_logic::*;
