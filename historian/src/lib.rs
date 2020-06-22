#[macro_use]
extern crate log;
extern crate wascc_actor as actor;
extern crate wasmdome_protocol as protocol;

use protocol::events::*;

const SUBJECT_TRIGGER_REPLAY: &str = "wasmdome.history.replay";

use actor::events::EventStreamsHostBinding;
use actor::prelude::*;
use std::collections::HashMap;

actor_handlers! {
    codec::messaging::OP_DELIVER_MESSAGE => handle_message,
    codec::core::OP_HEALTH_REQUEST => health
}

fn health(_req: codec::core::HealthRequest) -> HandlerResult<()> {
    Ok(())
}

fn handle_message(msg: codec::messaging::BrokerMessage) -> HandlerResult<()> {
    // This if statement is order sensitive since both these subjects have the same prefix.
    // BEWARE.
    let events = events::default();
    if msg.subject == SUBJECT_TRIGGER_REPLAY {
        trigger_replay(events, msg.body)
    } else if is_match_event_subject(&msg.subject) {
        //TODO? does not currently record arena events like actor up/down and match start/complete
        record_match_event(events, msg.body)
    } else {
        Ok(())
    }
}

/// Returns the stream ID for a match as persisted in the event stream provider
/// NOT to be confused with a message broker subject for a match
fn match_stream_id(match_id: &str) -> String {
    format!("wasmdome.history.match.{}", match_id)
}

fn is_match_event_subject(subject: &str) -> bool {
    subject.to_lowercase().starts_with("wasmdome.match.")
        && subject.to_lowercase().ends_with(".events")
}

/// Returns the subject on which MatchEvents will be replayed
fn replay_subject(match_id: &str) -> String {
    format!("wasmdome.match.{}.events.replay", match_id) // live subject is `wasmdome.match.{}.events`
}

fn record_match_event(events: EventStreamsHostBinding, msg: Vec<u8>) -> HandlerResult<()> {
    let evt: MatchEvent = serde_json::from_slice(&msg)?;
    trace!("Recording match event: {:?}", evt);
    let mut hash = HashMap::new();
    hash.insert("json".to_string(), serde_json::to_string(&evt)?);

    let _id = events.write_event(&match_stream_id(&extract_match_id(&evt)), hash)?;

    Ok(())
}

fn extract_match_id(evt: &MatchEvent) -> String {
    match evt {
        MatchEvent::TurnEvent { match_id, .. } => match_id.to_string(),
    }
}

fn trigger_replay(events: EventStreamsHostBinding, msg: Vec<u8>) -> HandlerResult<()> {
    let trigger: serde_json::Value = serde_json::from_slice(&msg)?;
    let match_id = trigger["match_id"].as_str().unwrap().to_string();
    trace!("Triggering replay of match {}", match_id);
    replay(events, &match_id)
}

fn replay(events: EventStreamsHostBinding, match_id: &str) -> HandlerResult<()> {
    let evts = events.read_all(&match_stream_id(match_id))?;
    let replay_subject = replay_subject(match_id);
    for event in evts {
        messaging::default().publish(&replay_subject, None, event.values["json"].as_bytes())?;
    }
    Ok(())
}
