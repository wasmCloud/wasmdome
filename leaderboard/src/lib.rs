extern crate wascc_actor as actor;
extern crate wasmdome_protocol as protocol;
#[macro_use]
extern crate serde_json;

use actor::prelude::*;
use domain::leaderboard::{Leaderboard, LeaderboardData};
use eventsourcing::Aggregate;
use protocol::events::*;
use wasmdome_domain as domain;

actor_handlers! { codec::messaging::OP_DELIVER_MESSAGE => handle_message,
codec::http::OP_HANDLE_REQUEST => produce_leaderboard,
codec::core::OP_HEALTH_REQUEST => health }

pub fn health(_req: codec::core::HealthRequest) -> ReceiveResult {
    Ok(vec![])
}

fn produce_leaderboard(_msg: codec::http::Request) -> ReceiveResult {
    let state: LeaderboardData = match &keyvalue::default().get("wasmdome:leaderboard")? {
        Some(lb) => serde_json::from_str(lb)?,
        None => LeaderboardData::default(),
    };
    let result = json!({
        "scores": state.scores,
    });
    Ok(serialize(codec::http::Response::json(result, 200, "OK"))?)
}

fn handle_message(msg: codec::messaging::BrokerMessage) -> ReceiveResult {
    if msg.subject.starts_with(SUBJECT_MATCH_EVENTS_PREFIX) {
        handle_match_event(msg.body)
    } else {
        // Ignore the message
        Ok(vec![])
    }
}

fn handle_match_event(msg: Vec<u8>) -> ReceiveResult {
    let evt: MatchEvent = serde_json::from_slice(&msg)?;

    match evt {
        MatchEvent::TurnEvent { turn_event, .. } => {
            let kv = keyvalue::default();
            let state: LeaderboardData = match &kv.get("wasmdome:leaderboard")? {
                Some(lb) => serde_json::from_str(lb)?,
                None => LeaderboardData::default(),
            };
            let new_state = Leaderboard::apply_event(&state, &turn_event)?;
            kv.set(
                "wasmdome:leaderboard",
                &serde_json::to_string(&new_state)?,
                None,
            )?;
            Ok(vec![])
        }
        _ => Ok(vec![]),
    }
}
