use crate::{
    commands::MechCommand,
    events::{EndCause, GameEvent},
    DamageSource,
};
use eventsourcing::{Aggregate, AggregateState};
use std::collections::HashMap;

const POINTS_DESTROY: usize = 100;
const POINTS_MATCH_WIN: usize = 10000;
const POINTS_MATCH_SURVIVE: usize = 2000;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LeaderboardData {
    scores: HashMap<String, usize>,
    generation: u64,
}

impl AggregateState for LeaderboardData {
    fn generation(&self) -> u64 {
        self.generation
    }
}

pub struct Leaderboard;
impl Aggregate for Leaderboard {
    type Event = GameEvent;
    type Command = MechCommand;
    type State = LeaderboardData;

    fn apply_event(state: &Self::State, evt: &Self::Event) -> eventsourcing::Result<Self::State> {
        match evt {
            GameEvent::MechDestroyed {
                damage_target,
                damage_source,
            } => Self::score_mech_death(state, damage_target.to_string(), damage_source.clone()),
            GameEvent::GameFinished {
                cause: EndCause::MechVictory(mech),
            } => Self::score_victory(state, mech.to_string()),
            GameEvent::GameFinished {
                cause: EndCause::MaxTurnsCompleted { survivors },
            } => Self::score_draw(state, survivors.clone()),
            _ => Ok(state.clone()),
        }
    }

    /// This aggregate doesn't handle commands
    fn handle_command(
        _state: &Self::State,
        _cmd: &Self::Command,
    ) -> eventsourcing::Result<Vec<Self::Event>> {
        Ok(vec![])
    }
}

impl Leaderboard {
    fn score_mech_death(
        state: &LeaderboardData,
        _target: String,
        source: DamageSource,
    ) -> eventsourcing::Result<LeaderboardData> {
        let mut state = state.clone();
        if let DamageSource::Mech(attacker) = source {
            state
                .scores
                .entry(attacker)
                .and_modify(|e| *e += POINTS_DESTROY)
                .or_insert(POINTS_DESTROY);
        }
        state.generation += 1;

        Ok(state)
    }

    fn score_victory(
        state: &LeaderboardData,
        mech: String,
    ) -> eventsourcing::Result<LeaderboardData> {
        let mut state = state.clone();

        state
            .scores
            .entry(mech)
            .and_modify(|e| *e += POINTS_MATCH_WIN)
            .or_insert(POINTS_MATCH_WIN);

        state.generation += 1;
        Ok(state)
    }

    fn score_draw(
        state: &LeaderboardData,
        survivors: Vec<String>,
    ) -> eventsourcing::Result<LeaderboardData> {
        let mut state = state.clone();

        for survivor in survivors {
            state
                .scores
                .entry(survivor)
                .and_modify(|e| *e += POINTS_MATCH_SURVIVE)
                .or_insert(POINTS_MATCH_SURVIVE);
        }
        Ok(state)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn award_points_for_kill() {
        let evts = vec![
            GameEvent::MechDestroyed {
                damage_source: DamageSource::Mech("al".to_string()),
                damage_target: "bob".to_string(),
            },
            GameEvent::MechDestroyed {
                damage_source: DamageSource::Mech("al".to_string()),
                damage_target: "steve".to_string(),
            },
        ];

        let state = LeaderboardData::default();
        let state = evts.iter().fold(state, |state, evt| {
            Leaderboard::apply_event(&state, evt).unwrap()
        });
        assert_eq!(state.scores["al"], 2 * POINTS_DESTROY);
    }

    #[test]
    fn award_points_for_win() {
        let evts = vec![
            GameEvent::GameFinished {
                cause: EndCause::MechVictory("al".to_string()),
            },
            GameEvent::GameFinished {
                cause: EndCause::MechVictory("bob".to_string()),
            },
        ];

        let state = LeaderboardData::default();
        let state = evts.iter().fold(state, |state, evt| {
            Leaderboard::apply_event(&state, evt).unwrap()
        });
        assert_eq!(state.scores["al"], POINTS_MATCH_WIN);
        assert_eq!(state.scores["bob"], POINTS_MATCH_WIN);
    }

    #[test]
    fn award_points_for_draw() {
        let evts = vec![
            GameEvent::GameFinished {
                cause: EndCause::MaxTurnsCompleted {
                    survivors: vec!["al".to_string(), "bob".to_string()],
                },
            },
            GameEvent::GameFinished {
                cause: EndCause::MechVictory("al".to_string()),
            },
        ];
        let state = LeaderboardData::default();
        let state = evts.iter().fold(state, |state, evt| {
            Leaderboard::apply_event(&state, evt).unwrap()
        });
        assert_eq!(state.scores["al"], POINTS_MATCH_WIN + POINTS_MATCH_SURVIVE);
        assert_eq!(state.scores["bob"], POINTS_MATCH_SURVIVE);
    }
}
