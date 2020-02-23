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
use domain::{commands::MechCommand, state::Match};
use domaincommon as domain;
use domaincommon::events::GameEvent;
use domaincommon::state::MatchState;
use eventsourcing::Aggregate;
use protocol::events::MatchEvent;
use wasmdome_protocol as protocol;

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
    if msg
        .subject
        .starts_with(protocol::events::SUBJECT_MATCH_EVENTS_PREFIX)
    {
        handle_event(ctx, serde_json::from_slice(&msg.body)?)
    } else {
        Ok(vec![])
    }
}

fn handle_event(ctx: &CapabilitiesContext, event: MatchEvent) -> ReceiveResult {
    if let MatchEvent::TurnRequested {
        actor,
        match_id,
        turn,
        commands,
    } = event
    {
        take_turn(ctx, &actor, &match_id, turn, commands)?;
        Ok(vec![])
    } else {
        Ok(vec![])
    }
}

/// 1. Load the current state of the match
/// 2. Apply each turn command to that state
/// 2a. Publish each event resulting from that command
/// 3. Save state
fn take_turn(
    ctx: &CapabilitiesContext,
    actor: &str,
    match_id: &str,
    turn: u32,
    commands: Vec<MechCommand>,
) -> ::std::result::Result<(), Box<dyn ::std::error::Error>> {
    let state = load_state(ctx, match_id)?;
    let state = commands.into_iter().fold(state, |state, cmd| {
        apply_command(ctx, &state, cmd, actor, turn, match_id)
    });
    set_state(ctx, match_id, state)?;
    Ok(())
}

fn apply_command(
    ctx: &CapabilitiesContext,
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
            publish_event(ctx, actor, match_id, turn, evt); // This is so side-effecty. Fix this.
            Match::apply_event(&state, evt).unwrap()
        })
}

fn publish_event(
    ctx: &CapabilitiesContext,
    actor: &str,
    match_id: &str,
    turn: u32,
    event: &GameEvent,
) {
    ctx.msg()
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
    ctx: &CapabilitiesContext,
    match_id: &str,
) -> ::std::result::Result<domain::state::MatchState, Box<dyn ::std::error::Error>> {
    let raw = ctx.kv().get(&format!("match:{}", match_id))?;
    let state: domain::state::MatchState = serde_json::from_str(&raw.unwrap())?;
    Ok(state)
}

fn set_state(
    ctx: &CapabilitiesContext,
    match_id: &str,
    state: domain::state::MatchState,
) -> ::std::result::Result<(), Box<dyn ::std::error::Error>> {
    ctx.kv().set(
        &format!("match:{}", match_id),
        &serde_json::to_string(&state)?,
        None,
    )?;
    Ok(())
}
