extern crate wascc_actor as actor;
extern crate wasmdome_protocol as protocol;
#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate log;

use actor::prelude::*;
use domain::leaderboard::{Leaderboard, LeaderboardData};
use eventsourcing::Aggregate;
use protocol::events::*;
use wasmdome_domain as domain;

const ARENA_LEADERBOARD_GET: &str = "wasmdome.internal.arena.leaderboard.get";

actor_handlers! {
    codec::messaging::OP_DELIVER_MESSAGE => handle_message,
    codec::core::OP_HEALTH_REQUEST => health
}

pub fn health(_req: codec::core::HealthRequest) -> HandlerResult<()> {
    Ok(())
}

fn produce_leaderboard() -> HandlerResult<serde_json::Value> {
    let state: LeaderboardData = match &keyvalue::default().get("wasmdome:leaderboard")? {
        Some(lb) => serde_json::from_str(lb)?,
        None => LeaderboardData::default(),
    };
    let result = json!({
        "stats": state.stats,
        "mechs": state.mechs,
    });
    Ok(result)
}

fn handle_message(msg: codec::messaging::BrokerMessage) -> HandlerResult<()> {
    if msg.subject.starts_with("wasmdome.match.") && msg.subject.ends_with(".events") {
        handle_match_event(msg.body)
    } else if msg.subject == ARENA_LEADERBOARD_GET {
        let lb = produce_leaderboard()?;
        messaging::default().publish(&msg.reply_to, None, &serde_json::to_vec(&lb)?)?;
        Ok(())
    } else {
        Err("bad dispatch".into())
    }
}

fn handle_match_event(msg: Vec<u8>) -> HandlerResult<()> {
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
            Ok(())
        }
    }
}

#[cfg(test)]
mod test {
    use wasmdome_domain::leaderboard::{LeaderboardData, MechSummary, PlayerStats};

    // Here so we can fail a test if we change our serialization structure because
    // other apps (e.g. website) depend on this format
    #[test]
    fn leaderboard_as_json() {
        let mut lbd = LeaderboardData::default();
        lbd.stats.insert("boylur".to_string(), boylur_stats());
        lbd.mechs.insert(
            "boylur".to_string(),
            MechSummary {
                avatar: "boylur".to_string(),
                name: "Boylur Plait".to_string(),
                team: "boylur".to_string(),
                id: "boylur".to_string(),
            },
        );
        let json = serde_json::to_string(&lbd).unwrap();
        assert_eq!("{\"stats\":{\"boylur\":{\"score\":5000,\"wins\":10,\"draws\":10,\"kills\":100,\"deaths\":0}},\"mechs\":{\"boylur\":{\"id\":\"boylur\",\"name\":\"Boylur Plait\",\"avatar\":\"boylur\",\"team\":\"boylur\"}},\"generation\":0}",         
        json);
    }

    fn boylur_stats() -> PlayerStats {
        PlayerStats {
            score: 5000,
            wins: 10,
            draws: 10,
            kills: 100,
            deaths: 0,
        }
    }
}
