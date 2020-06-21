use crate::store::MatchStore;
use chrono::prelude::*;
use domain::eventsourcing::Aggregate;
use domain::{
    commands::MechCommand,
    events::GameEvent,
    state::{Match, MatchState},
    Point,
};
use protocol::{
    commands::{TakeTurn, TakeTurnResponse},
    events::{ArenaEvent, MatchEvent},
    OP_TAKE_TURN,
};
use std::sync::{Arc, RwLock};
use wascc_codec::{capabilities::Dispatcher, deserialize, serialize};

pub(crate) fn spawn_mechs(
    nc: Arc<nats::Connection>,
    state: MatchState,
    actors: Vec<String>,
) -> MatchState {
    let mut state = state.clone();
    for actor in actors {
        let cmd = MechCommand::SpawnMech {
            mech: actor.to_string(),
            position: random_spawnpoint(state.parameters.height, state.parameters.width),
            team: get_team(&vec![]), // TODO: get tags from actor claims during binding
            avatar: get_avatar(&vec![]).to_string(), // TODO: get tags from actor claims during binding
            name: format!("{}'s Mech", actor), // TODO: pull mech name from the signed actor's name
        };
        for event in Match::handle_command(&state, &cmd).unwrap() {
            nc.publish(
                &protocol::events::events_subject(Some(&state.parameters.match_id)),
                &serde_json::to_vec(&turn_event(&event, &actor, &state.parameters.match_id))
                    .unwrap(),
            )
            .unwrap();
            state = Match::apply_event(&state, &event).unwrap();
        }
    }
    state
}

pub(crate) fn turn_event(evt: &GameEvent, match_id: &str, actor: &str) -> MatchEvent {
    MatchEvent::TurnEvent {
        actor: actor.to_string(),
        match_id: match_id.to_string(),
        turn_event: evt.clone(),
        turn: 0,
    }
}

pub(crate) fn random_spawnpoint(board_height: u32, board_width: u32) -> Point {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let x: u32 = rng.gen_range(1, board_width);
    let y: u32 = rng.gen_range(1, board_height);
    Point::new(x as _, y as _)
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

pub(crate) fn manage_match(
    nc: Arc<nats::Connection>,
    dispatcher: Arc<RwLock<Box<dyn Dispatcher>>>,
    store: Arc<RwLock<MatchStore>>,
    actors: Vec<String>,
    match_id: String,
) {
    info!("Starting thread to manage match {}", match_id);
    std::thread::spawn(move || {
        let mut match_complete = false;
        let mut turn = 0;
        while !match_complete {
            let mut state = store.read().unwrap().get_match_state(&match_id).unwrap();
            for pk in &actors {
                let tt = TakeTurn {
                    actor: pk.to_string(),
                    match_id: match_id.to_string(),
                    turn,
                    state: state.clone(),
                };
                let mech_turn_result =
                    dispatcher
                        .read()
                        .unwrap()
                        .dispatch(pk, OP_TAKE_TURN, &serialize(tt).unwrap());
                match mech_turn_result {
                    Ok(tr) => {
                        let tr: TakeTurnResponse = deserialize(&tr).unwrap();
                        state = process_turn_response(
                            nc.clone(),
                            pk,
                            tr,
                            store.clone(),
                            &state,
                            &match_id,
                            turn,
                        );
                    }
                    Err(e) => error!("Failed to get turn actions from actor {}: {}", pk, e),
                }
            }
            turn = turn + 1;
            match_complete = check_match_over(&state);
            if match_complete {
                publish_match_complete(nc.clone(), &state);
                info!("Match {} completed", match_id);
            }
        }
    });
}

fn publish_match_complete(nc: Arc<nats::Connection>, state: &MatchState) {
    nc.publish(
        &protocol::events::events_subject(None),
        serde_json::to_string(&ArenaEvent::MatchCompleted {
            time: Utc::now(),
            match_id: state.parameters.match_id.to_string(),
            cause: state.completed.as_ref().unwrap().clone(),
        })
        .unwrap(),
    )
    .unwrap();
}

fn check_match_over(state: &MatchState) -> bool {
    state.completed.is_some() || (state.turn_status.current > state.parameters.max_turns)
}

fn process_turn_response(
    nc: Arc<nats::Connection>,
    actor: &str,
    resp: TakeTurnResponse,
    store: Arc<RwLock<MatchStore>>,
    state: &MatchState,
    match_id: &str,
    turn: u32,
) -> MatchState {
    let state = state.clone();
    let state = resp.commands.into_iter().fold(state, |state, cmd| {
        apply_command(nc.clone(), &state, cmd, actor, turn, match_id)
    });
    store
        .write()
        .unwrap()
        .save_match_state(match_id, state.clone())
        .unwrap();
    state
}

fn apply_command(
    nc: Arc<nats::Connection>,
    state: &MatchState,
    cmd: MechCommand,
    actor: &str,
    turn: u32,
    match_id: &str,
) -> MatchState {
    let state = state.clone();
    Match::handle_command(&state, &cmd)
        .unwrap()
        .iter()
        .fold(state, |state, evt| {
            publish_event(nc.clone(), actor, match_id, turn, evt); // This is so side-effecty. Fix this.
            match Match::apply_event(&state, evt) {
                Ok(evt) => evt,
                Err(e) => {
                    error!("Event processing failure: {}", e);
                    state
                }
            }
        })
}

fn publish_event(
    nc: Arc<nats::Connection>,
    actor: &str,
    match_id: &str,
    turn: u32,
    event: &GameEvent,
) {
    let subject = protocol::events::events_subject(Some(match_id));
    nc.publish(
        &subject,
        &serde_json::to_vec(&MatchEvent::TurnEvent {
            turn,
            actor: actor.to_string(),
            match_id: match_id.to_string(),
            turn_event: event.clone(),
        })
        .unwrap(),
    )
    .unwrap();
}

pub(crate) fn spawn_health_check(
    nc: Arc<nats::Connection>,
    dispatcher: Arc<RwLock<Box<dyn Dispatcher>>>,
    store: Arc<RwLock<MatchStore>>,
) {
    ::std::thread::spawn(move || loop {
        ::std::thread::sleep(::std::time::Duration::from_secs(10));
        perform_health_check(store.clone(), dispatcher.clone(), nc.clone());
    });
}

pub(crate) fn perform_health_check(
    store: Arc<RwLock<MatchStore>>,
    dispatcher: Arc<RwLock<Box<dyn Dispatcher>>>,
    nc: Arc<nats::Connection>,
) {
    let ba = store.write().unwrap().bound_actors().unwrap();
    for actor in ba.iter() {
        let h = dispatcher.read().unwrap().dispatch(
            actor,
            codec::core::OP_HEALTH_REQUEST,
            &serialize(&codec::core::HealthRequest { placeholder: true }).unwrap(),
        );
        if h.is_err() {
            info!("Health check on {} failed, unbinding.", actor);
            publish_disconnect_event(nc.clone(), actor);
            store.write().unwrap().remove_bound_actor(actor).unwrap();
        }
    }
}

pub(crate) fn publish_disconnect_event(nc: Arc<nats::Connection>, actor: &str) {
    nc.publish(
        &protocol::events::events_subject(None),
        serde_json::to_string(&ArenaEvent::MechDisconnected {
            actor: actor.to_string(),
            time: Utc::now(),
        })
        .unwrap(),
    )
    .unwrap();
}
