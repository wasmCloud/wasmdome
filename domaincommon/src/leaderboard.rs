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
pub struct PlayerStats {
    pub score: usize,
    pub wins: usize,    
    pub draws: usize,
    pub kills: usize,
    pub deaths: usize,    
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LeaderboardData {
    pub stats: HashMap<String, PlayerStats>,
    pub generation: u64,
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
    // Source kills target
    fn score_mech_death(
        state: &LeaderboardData,
        target: String,
        source: DamageSource,
    ) -> eventsourcing::Result<LeaderboardData> {
        let mut state = state.clone();
        if let DamageSource::MechWeapon(attacker) = source {
            state
                .stats
                .entry(attacker)
                .and_modify(|e| {
                    e.score += POINTS_DESTROY;
                    e.kills += 1;
                })
                .or_insert(PlayerStats{
                    score: POINTS_DESTROY,
                    kills: 1,
                    ..Default::default()
                });
            state
                .stats
                .entry(target)
                .and_modify(|e| {
                    e.deaths += 1;
                })
                .or_insert(PlayerStats{
                    deaths: 1,
                    ..Default::default()
                });
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
            .stats
            .entry(mech)
            .and_modify(|e| {
                e.score += POINTS_MATCH_WIN;
                e.wins += 1;
            })
            .or_insert(PlayerStats{
                score: POINTS_MATCH_WIN,
                wins: 1,
                ..Default::default()
            });

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
                .stats
                .entry(survivor)
                .and_modify(|e| {
                    e.score += POINTS_MATCH_SURVIVE;
                    e.draws += 1;
                })
                .or_insert(PlayerStats{
                    score:POINTS_MATCH_SURVIVE,
                    draws: 1,
                    ..Default::default()
                });
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
                damage_source: DamageSource::MechWeapon("al".to_string()),
                damage_target: "bob".to_string(),
            },
            GameEvent::MechDestroyed {
                damage_source: DamageSource::MechWeapon("al".to_string()),
                damage_target: "steve".to_string(),
            },
        ];

        let state = LeaderboardData::default();
        let state = evts.iter().fold(state, |state, evt| {
            Leaderboard::apply_event(&state, evt).unwrap()
        });
        assert_eq!(state.stats["al"].score, 2 * POINTS_DESTROY);
        assert_eq!(state.stats["al"].kills, 2);
        assert_eq!(state.stats["bob"].deaths, 1);
        assert_eq!(state.stats["steve"].deaths, 1);
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
        assert_eq!(state.stats["al"].score, POINTS_MATCH_WIN);
        assert_eq!(state.stats["bob"].score, POINTS_MATCH_WIN);
        assert_eq!(state.stats["al"].wins, 1);
        assert_eq!(state.stats["bob"].wins, 1);
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
        assert_eq!(state.stats["al"].score, POINTS_MATCH_WIN + POINTS_MATCH_SURVIVE);
        assert_eq!(state.stats["bob"].score, POINTS_MATCH_SURVIVE);
        assert_eq!(state.stats["al"].draws, 1);
        assert_eq!(state.stats["bob"].draws, 1);
        assert_eq!(state.stats["al"].wins, 1);
    }
}
