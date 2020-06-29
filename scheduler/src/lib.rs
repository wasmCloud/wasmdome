// Copyright 2015-2020 Capital One Services, LLC
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

#[macro_use]
extern crate serde;

extern crate wascc_actor as actor;

const SUBJECT_REQUEST_SCHEDULE: &str = "wasmdome.public.arena.schedule";
const SUBJECT_ADD_MATCH: &str = "wasmdome.internal.arena.new_match";
const SUBJECT_DEL_MATCH: &str = "wasmdome.internal.arena.del_match";

const APS_PER_TURN: u32 = 4;

use actor::prelude::*;
use chrono::DateTime;
use chrono::Utc;

actor_handlers! {
    codec::messaging::OP_DELIVER_MESSAGE => handle_message,
    codec::core::OP_HEALTH_REQUEST => health
}

fn handle_message(msg: codec::messaging::BrokerMessage) -> HandlerResult<()> {
    match msg.subject.as_str() {
        SUBJECT_REQUEST_SCHEDULE => get_schedule(&msg.reply_to),
        SUBJECT_ADD_MATCH => add_match(&msg.reply_to, serde_json::from_slice(&msg.body)?),
        SUBJECT_DEL_MATCH => del_match(&msg.reply_to, serde_json::from_slice(&msg.body)?),
        _ => Err("Unexpected subject".into()),
    }
}

fn health(_req: codec::core::HealthRequest) -> HandlerResult<()> {
    Ok(())
}

fn get_schedule(reply_to: &str) -> HandlerResult<()> {
    let mut result = Vec::new();
    let matches = keyvalue::default().set_members(&match_set_key())?;
    for match_id in matches {
        let sm: StoredMatch =
            serde_json::from_str(&keyvalue::default().get(&match_key(&match_id))?.unwrap())?;
        result.push(sm);
    }
    result.sort_by(|a, b| a.entry.match_start.cmp(&b.entry.match_start));
    messaging::default().publish(reply_to, None, &serde_json::to_vec(&result)?)?;
    Ok(())
}

fn add_match(reply_to: &str, match_schedule: MatchScheduleEntry) -> HandlerResult<()> {
    let sm = StoredMatch {
        match_id: extras::default().get_guid()?,
        entry: match_schedule.clone(),
        aps_per_turn: APS_PER_TURN,
    };
    let _ =
        keyvalue::default().set(&match_key(&sm.match_id), &serde_json::to_string(&sm)?, None)?;
    let _ = keyvalue::default().set_add(&match_set_key(), &sm.match_id)?;
    let _ = messaging::default().publish(reply_to, None, &serde_json::to_vec(&sm)?)?;
    Ok(())
}

fn del_match(reply_to: &str, match_id: MatchIdentifier) -> HandlerResult<()> {
    let key = match_key(&match_id.match_id);
    keyvalue::default().del_key(&key)?;
    keyvalue::default().set_remove(&match_set_key(), &key)?;
    messaging::default().publish(reply_to, None, b"OK")?;
    Ok(())
}

fn match_key(match_id: &str) -> String {
    format!("wasmdome:sched_matches:{}", match_id)
}

fn match_set_key() -> String {
    "wasmdome:sched_matches".to_string()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct MatchScheduleEntry {
    pub max_actors: u32,
    pub board_height: u32,
    pub board_width: u32,
    pub max_turns: u32,
    pub match_start: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct MatchIdentifier {
    pub match_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct StoredMatch {
    match_id: String,
    entry: MatchScheduleEntry,
    aps_per_turn: u32,
}
