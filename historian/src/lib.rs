extern crate wascc_actor as actor;
extern crate wasmdome_protocol as protocol;

use protocol::events::*;

const SUBJECT_TRIGGER_REPLAY: &str = "wasmdome.history.replay";

use actor::prelude::*;
use std::collections::HashMap;

actor_receive!(receive);

pub fn receive(ctx: &CapabilitiesContext, operation: &str, msg: &[u8]) -> ReceiveResult {
    match operation {
        messaging::OP_DELIVER_MESSAGE => handle_message(ctx, msg),
        core::OP_HEALTH_REQUEST => Ok(vec![]),
        _ => Err("Unknown operation".into()),
    }
}

fn handle_message(
    ctx: &CapabilitiesContext,
    msg: impl Into<messaging::DeliverMessage>,
) -> ReceiveResult {
    let msg = msg.into().message.unwrap();
    // This if statement is order sensitive since both these subjects have the same prefix. 
    // BEWARE.
    if msg.subject == SUBJECT_TRIGGER_REPLAY {
        trigger_replay(ctx, msg.body)
    } else if msg.subject.starts_with(SUBJECT_MATCH_EVENTS_PREFIX) {
        record_match_event(ctx, msg.body)        
    } else {
        Ok(vec![])
    }
}

fn record_match_event(ctx: &CapabilitiesContext, msg: Vec<u8>) -> ReceiveResult {
    ctx.log("Recording match event");
    let evt: MatchEvent = serde_json::from_slice(&msg)?;    
    let mut hash = HashMap::new();
    hash.insert("json".to_string(), serde_json::to_string(&evt)?);

    let _id = ctx
        .events()
        .write_event(&format!("wasmdome.history.match.{}", extract_match_id(&evt)), hash)?;

    Ok(vec![])
}

fn extract_match_id(evt: &MatchEvent) -> String {
    match evt {
        MatchEvent::MatchCreated { match_id, .. } => match_id.to_string(),
        MatchEvent::ActorStarted { match_id, .. } => match_id.to_string(),
        MatchEvent::MatchStarted { match_id, .. } => match_id.to_string(),
        MatchEvent::TurnRequested { match_id, .. } => match_id.to_string(),
        MatchEvent::TurnEvent { match_id, .. } => match_id.to_string(),
    }
}

fn trigger_replay(ctx: &CapabilitiesContext, msg: Vec<u8>) -> ReceiveResult {
    let trigger: serde_json::Value = serde_json::from_slice(&msg)?;
    let match_id = trigger["match_id"].as_str().unwrap().to_string();
    ctx.log(&format!("Triggering replay of match {}", match_id));
    
    replay(ctx, &match_id)
}

fn replay(ctx: &CapabilitiesContext, match_id: &str) -> ReceiveResult {
    let evts = ctx
        .events()
        .read_all(&format!("wasmdome.history.match.{}", match_id))?;
    let replay_subject = format!("wasmdome.match_events.{}.replay", match_id);
    for event in evts {
        ctx.msg()
            .publish(&replay_subject, None, event.values["json"].as_bytes())?;
    }
    Ok(vec![])
}
