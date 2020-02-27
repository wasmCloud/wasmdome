extern crate wasmdome_protocol as protocol;
extern crate wascc_actor as actor;
#[macro_use]
extern crate serde_json;

use actor::prelude::*;
use protocol::events::*;
use eventsourcing::Aggregate;
use wasmdome_domain as domain;
use domain::leaderboard::{LeaderboardData, Leaderboard};

actor_receive!(receive);

pub fn receive(ctx: &CapabilitiesContext, operation: &str, msg: &[u8]) -> ReceiveResult {    
    match operation {
        messaging::OP_DELIVER_MESSAGE => handle_message(ctx, msg),
        http::OP_HANDLE_REQUEST => produce_leaderboard(ctx, msg),
        core::OP_HEALTH_REQUEST => Ok(vec![]),
        _ => Err("Unknown operation".into()),
    }
}

fn produce_leaderboard(ctx: &CapabilitiesContext, _msg: impl Into<http::Request>) -> ReceiveResult {
    let state: LeaderboardData = match &ctx.kv().get("wasmdome:leaderboard")? {
        Some(lb) => serde_json::from_str(lb)?,
        None => LeaderboardData::default()
    };
    let result = json!({
        "scores": state.scores,
    });
    Ok(protobytes(http::Response::json(result, 200, "OK"))?)

}

fn handle_message(
    ctx: &CapabilitiesContext,
    msg: impl Into<messaging::DeliverMessage>,
) -> ReceiveResult {
    let msg = msg.into().message.unwrap();
    if msg.subject.starts_with(SUBJECT_MATCH_EVENTS_PREFIX) {
        handle_match_event(ctx, msg.body)
    } else {
        // Ignore the message
        Ok(vec![])
    }
}

fn handle_match_event(ctx: &CapabilitiesContext, msg: Vec<u8>) -> ReceiveResult {
    let evt: MatchEvent = serde_json::from_slice(&msg)?;

    match evt {
        MatchEvent::TurnEvent{turn_event, ..  } => {
            let state: LeaderboardData = match &ctx.kv().get("wasmdome:leaderboard")? {
                Some(lb) => serde_json::from_str(lb)?,
                None => LeaderboardData::default()
            };
            let new_state = Leaderboard::apply_event(&state, &turn_event)?;            
            ctx.kv().set("wasmdome:leaderboard", &serde_json::to_string(&new_state)?, None)?;
            Ok(vec![])
        },
        _ => Ok(vec![])
    }
    
}