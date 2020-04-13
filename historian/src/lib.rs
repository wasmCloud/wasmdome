extern crate wascc_actor as actor;
extern crate wasmdome_protocol as protocol;

use protocol::events::*;

const SUBJECT_TRIGGER_REPLAY: &str = "wasmdome.history.replay";

use actor::events::EventStreamsHostBinding;
use actor::logger::AutomaticLoggerHostBinding;
use actor::prelude::*;
use std::collections::HashMap;

actor_handlers! { codec::messaging::OP_DELIVER_MESSAGE => handle_message, codec::core::OP_HEALTH_REQUEST => health }

fn health(_req: codec::core::HealthRequest) -> ReceiveResult {
    Ok(vec![])
}

fn handle_message(msg: codec::messaging::BrokerMessage) -> ReceiveResult {
    // This if statement is order sensitive since both these subjects have the same prefix.
    // BEWARE.
    let logger = logger::default();
    let events = events::default();
    if msg.subject == SUBJECT_TRIGGER_REPLAY {
        trigger_replay(logger, events, msg.body)
    } else if msg.subject.starts_with(SUBJECT_MATCH_EVENTS_PREFIX) {
        record_match_event(logger, events, msg.body)
    } else {
        Ok(vec![])
    }
}

fn record_match_event(
    logger: AutomaticLoggerHostBinding,
    events: EventStreamsHostBinding,
    msg: Vec<u8>,
) -> ReceiveResult {
    logger.info("Recording match event")?;
    let evt: MatchEvent = serde_json::from_slice(&msg)?;
    let mut hash = HashMap::new();
    hash.insert("json".to_string(), serde_json::to_string(&evt)?);

    let _id = events.write_event(
        &format!("wasmdome.history.match.{}", extract_match_id(&evt)),
        hash,
    )?;

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

fn trigger_replay(
    logger: AutomaticLoggerHostBinding,
    events: EventStreamsHostBinding,
    msg: Vec<u8>,
) -> ReceiveResult {
    let trigger: serde_json::Value = serde_json::from_slice(&msg)?;
    let match_id = trigger["match_id"].as_str().unwrap().to_string();
    logger.info(&format!("Triggering replay of match {}", match_id))?;
    replay(events, &match_id)
}

fn replay(events: EventStreamsHostBinding, match_id: &str) -> ReceiveResult {
    let evts = events.read_all(&format!("wasmdome.history.match.{}", match_id))?;
    let replay_subject = format!("wasmdome.match_events.{}.replay", match_id);
    for event in evts {
        messaging::default().publish(&replay_subject, None, event.values["json"].as_bytes())?;
    }
    Ok(vec![])
}
