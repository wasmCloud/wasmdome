// Copyright 2015-2019 Capital One Services, LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

extern crate wascc_actor as actor;
use actor::prelude::*;
use domaincommon as domain;
use protocol::commands::*;
use protocol::events::*;
use wasmdome_protocol as protocol;

mod store;

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
    if msg.subject == SUBJECT_CREATE_MATCH {
        create_match(ctx, msg.body, &msg.reply_to)
    } else if msg.subject.starts_with(SUBJECT_MATCH_EVENTS_PREFIX) {
        handle_match_event(ctx, msg.body)
    } else {
        Ok(vec![])
    }
}

fn handle_match_event(ctx: &CapabilitiesContext, msg: Vec<u8>) -> ReceiveResult {
    let evt: MatchEvent = serde_json::from_slice(&msg)?;
    match evt {
        MatchEvent::ActorScheduled { match_id, actor } => {
            spawn_actor(ctx, &match_id, &actor)?;
            if is_match_ready(ctx, &match_id)? {
                start_match(ctx, &match_id)?;
            }
            Ok(vec![])
        }
        _ => Ok(vec![]),
    }
}

/// Load match state, apply the spawn actor event to it, save state again
fn spawn_actor(
    ctx: &CapabilitiesContext,
    match_id: &str,
    actor: &str,
) -> ::std::result::Result<(), Box<dyn ::std::error::Error>> {
    ctx.log(&format!("Spawning actor {} into match {}", actor, match_id));
    use domain::eventsourcing::Aggregate;

    let mut state: domain::state::MatchState = store::load_state(ctx, match_id)?;
    let cmd = domain::commands::MechCommand::SpawnMech {
        mech: actor.to_string(),
        position: domain::Point::new(0, 0), // TODO: don't spawn them on the origin
    };
    for event in domain::state::Match::handle_command(&state, &cmd).unwrap() {
        state = domain::state::Match::apply_event(&state, &event).unwrap();
    }

    store::set_state(ctx, match_id, state)?;

    Ok(())
}

/// Publish MatchStarted event, initiate the "turn loop" for getting commands from actors
fn start_match(ctx: &CapabilitiesContext, match_id: &str) -> ReceiveResult {
    let started = protocol::events::MatchEvent::MatchStarted {
        match_id: match_id.to_string(),
    };
    let subject = format!("wasmdome.match_events.{}", match_id);
    ctx.msg()
        .publish(&subject, None, &serde_json::to_vec(&started)?)?;

    let state = store::load_state(ctx, match_id)?;

    for actor in state.parameters.actors {
        let turn_subject = format!("wasmdome.matches.{}.turns.{}", match_id, actor);
        let turn = protocol::commands::TakeTurn {
            actor: actor.to_string(),
            match_id: match_id.to_string(),
            turn: 0,
        };
        ctx
            .msg()  
            .publish(&turn_subject, None, &serde_json::to_vec(&turn)?)?;        
    }

    Ok(vec![])
}

/// A match is ready to start when all of the required actors have been scheduled
fn is_match_ready(
    ctx: &CapabilitiesContext,
    match_id: &str,
) -> ::std::result::Result<bool, Box<dyn ::std::error::Error>> {
    let raw = ctx.kv().get(&format!("match:{}", match_id))?;
    Ok(raw.map_or(false, |v| {
        let state: domaincommon::state::MatchState = serde_json::from_str(&v).unwrap();
        state.parameters.actors.len() == state.mechs.len()
    }))
}

/// 0. create match state in KV store
/// 1. Reply with start ack
/// 2. Publish MatchCreated event
/// 3. Send ScheduleActor command for each actor in the match
fn create_match(ctx: &CapabilitiesContext, msg: Vec<u8>, reply_to: &str) -> ReceiveResult {
    use domain::state::{MatchParameters, MatchState};
    let createmsg: CreateMatch = serde_json::from_slice(&msg)?;

    let ack = StartMatchAck {
        match_id: createmsg.match_id.clone(),
    };
    let params = MatchParameters::new(
        createmsg.match_id.clone(),
        createmsg.board_width,
        createmsg.board_height,
        createmsg.actors.clone(),
    );
    let state = MatchState::new_with_parameters(params);
    store::set_state(ctx, &createmsg.match_id, state)?;

    ctx.msg()
        .publish(reply_to, None, &serde_json::to_vec(&ack)?)?;
    ctx.msg().publish(
        &format!("wasmdome.matches.{}.events", createmsg.match_id),
        None,
        &serde_json::to_vec(&MatchEvent::MatchCreated {
            match_id: createmsg.match_id.clone(),
            board_height: createmsg.board_height,
            board_width: createmsg.board_width,
            actors: createmsg.actors.clone(),
        })?,
    )?;

    for actor in createmsg.actors {
        let sched = ScheduleActor {
            actor,
            match_id: createmsg.match_id.clone(),
        };
        ctx.msg().publish(
            &format!(
                "{}.{}.{}",
                SUBJECT_MATCH_COMMANDS_PREFIX, createmsg.match_id, SUBJECT_SCHEDULE_ACTOR
            ),
            None,
            &serde_json::to_vec(&sched)?,
        )?;        
    }
    Ok(vec![])
}
