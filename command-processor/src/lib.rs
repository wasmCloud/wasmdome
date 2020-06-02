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
use actor::keyvalue::KeyValueStoreHostBinding;
use actor::prelude::*;
use wasmdome_domain as domain;

use domain::events::GameEvent;
use domain::state::MatchState;
use domain::{commands::MechCommand, state::Match};
use eventsourcing::Aggregate;
use protocol::events::MatchEvent;
use wasmdome_protocol as protocol;

actor_handlers! { codec::messaging::OP_DELIVER_MESSAGE => handle_message, codec::core::OP_HEALTH_REQUEST => health }

fn health(_req: codec::core::HealthRequest) -> HandlerResult<()> {
    Ok(())
}

fn handle_message(msg: codec::messaging::BrokerMessage) -> HandlerResult<()> {
    if msg
        .subject
        .starts_with(protocol::events::SUBJECT_MATCH_EVENTS_PREFIX)
    {
        handle_event(serde_json::from_slice(&msg.body)?)
    } else {
        Ok(())
    }
}

fn handle_event(event: MatchEvent) -> HandlerResult<()> {
    if let MatchEvent::TurnRequested {
        actor,
        match_id,
        turn,
        commands,
    } = event
    {
        take_turn(&actor, &match_id, turn, commands)?;
        Ok(())
    } else {
        Ok(())
    }
}

/// 1. Load the current state of the match
/// 2. Apply each turn command to that state
/// 2a. Publish each event resulting from that command
/// 3. Save state
fn take_turn(
    actor: &str,
    match_id: &str,
    turn: u32,
    commands: Vec<MechCommand>,
) -> ::std::result::Result<(), Box<dyn ::std::error::Error>> {
    let kv = keyvalue::default();
    let state = load_state(&kv, match_id)?;
    let state = commands.into_iter().fold(state, |state, cmd| {
        apply_command(&state, cmd, actor, turn, match_id)
    });
    set_state(&kv, match_id, state)?;
    Ok(())
}

fn apply_command(
    state: &MatchState,
    cmd: MechCommand,
    actor: &str,
    turn: u32,
    match_id: &str,
) -> MatchState {
    let state = state.clone();
    //TODO: Examine here for if the event actually goes through
    Match::handle_command(&state, &cmd)
        .unwrap()
        .iter()
        .fold(state, |state, evt| {
            publish_event(actor, match_id, turn, evt); // This is so side-effecty. Fix this.
            match Match::apply_event(&state, evt) {
                Ok(evt) => evt,
                Err(e) => {
                    logger::default().info(&format!("Event processing failure: {}", e));
                    state
                }
            }
        })
}

fn publish_event(actor: &str, match_id: &str, turn: u32, event: &GameEvent) {
    messaging::default()
        .publish(
            &format!("wasmdome.match_events.{}", match_id),
            None,
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

fn load_state(
    kv: &KeyValueStoreHostBinding,
    match_id: &str,
) -> ::std::result::Result<domain::state::MatchState, Box<dyn ::std::error::Error>> {
    let raw = kv.get(&format!("match:{}", match_id))?;
    let state: domain::state::MatchState = serde_json::from_str(&raw.unwrap())?;
    Ok(state)
}

fn set_state(
    kv: &KeyValueStoreHostBinding,
    match_id: &str,
    state: domain::state::MatchState,
) -> ::std::result::Result<(), Box<dyn ::std::error::Error>> {
    kv.set(
        &format!("match:{}", match_id),
        &serde_json::to_string(&state)?,
        None,
    )?;
    Ok(())
}
