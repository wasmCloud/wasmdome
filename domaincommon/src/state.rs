use crate::{
    commands::MechCommand,
    events::{EndCause, GameEvent},
    DamageSource, GameBoard, GridDirection, Point,
};
use eventsourcing::Result;
use eventsourcing::{Aggregate, AggregateState};
use std::collections::{HashMap, HashSet};

const WALL_DAMAGE: u32 = 10; // Lose 10 HP for bouncing off obstacles
const PRIMARY_DAMAGE: u32 = 75;
const SECONDARY_DAMAGE: u32 = 110;
const SECONDARY_SPLASH_DAMAGE: u32 = 90;

const INITIAL_HEALTH: u32 = 1000;
const PRIMARY_RANGE: usize = 3;
const SECONDARY_RANGE: usize = 6;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MatchParameters {
    pub match_id: String,
    pub width: u32,
    pub height: u32,
    pub actors: Vec<String>,
    pub max_turns: u32,
}

impl MatchParameters {
    pub fn new(
        match_id: String,
        width: u32,
        height: u32,
        max_turns: u32,
        actors: Vec<String>,
    ) -> Self {
        MatchParameters {
            match_id,
            width,
            height,
            actors,
            max_turns,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TurnStatus {
    pub current: u32,
    pub taken: HashSet<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MatchState {
    pub parameters: MatchParameters,
    pub mechs: HashMap<String, MechState>,
    pub generation: u64,
    pub game_board: GameBoard,
    pub turn_status: TurnStatus,
    pub completed: Option<EndCause>,
}

impl MatchState {
    pub fn new_with_parameters(params: MatchParameters) -> MatchState {
        MatchState {
            parameters: params.clone(),
            mechs: HashMap::new(),
            generation: 0,
            game_board: GameBoard {
                height: params.height,
                width: params.width,
            },
            completed: None,
            turn_status: Default::default(),
        }
    }

    fn validate_has_mech(state: &MatchState, mech: &str) -> Result<()> {
        if !state.mechs.contains_key(mech) {
            return Err(eventsourcing::Error {
                kind: eventsourcing::Kind::CommandFailure(
                    "Command received for mech not in match".to_string(),
                ),
            });
        }
        Ok(())
    }

    fn finish_game(state: &MatchState, cause: &EndCause) -> MatchState {
        let mut state = state.clone();
        state.completed = Some(cause.clone());
        state
    }

    fn modify_mech<F>(state: &MatchState, mech: &str, fun: F) -> MatchState
    where
        F: Fn(MechState) -> MechState,
    {
        let mut state = state.clone();
        state.mechs = state
            .mechs
            .clone()
            .into_iter()
            .map(|(key, ms)| {
                if key == mech {
                    (key, fun(ms))
                } else {
                    (key, ms)
                }
            })
            .collect();
        state.generation = state.generation + 1;
        state
    }

    fn mech_at(state: &MatchState, position: &Point) -> Option<MechState> {
        state
            .mechs
            .values()
            .find(|m| m.position == *position)
            .cloned()
    }

    fn insert_mech(state: &MatchState, mech: &str, position: &Point) -> MatchState {
        let mut state = state.clone();
        state.mechs.insert(
            mech.to_string(),
            MechState {
                position: position.clone(),
                id: mech.to_string(),
                ..Default::default()
            },
        );
        MatchState {
            mechs: state.mechs,
            ..state
        }
    }

    fn advance_match_turn(state: &MatchState, turn: u32) -> MatchState {
        let mut state = state.clone();
        state.turn_status.taken.clear();
        state.turn_status.current = turn;
        state
    }

    fn advance_mech_turn(state: &MatchState, mech: &str) -> MatchState {
        let mut state = state.clone();
        state.turn_status.taken.insert(mech.to_string());
        state
    }
}

impl AggregateState for MatchState {
    fn generation(&self) -> u64 {
        self.generation
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MechState {
    pub id: String,
    pub health: u32,
    pub position: Point,
    pub alive: bool,
    pub victor: bool,
}

impl Default for MechState {
    fn default() -> MechState {
        MechState {
            health: INITIAL_HEALTH,
            position: Point::new(0, 0),
            alive: true,
            victor: false,
            id: "None".to_string(),
        }
    }
}

pub struct Match;
impl Aggregate for Match {
    type Event = GameEvent;
    type Command = MechCommand;
    type State = MatchState;

    fn apply_event(state: &Self::State, evt: &Self::Event) -> Result<Self::State> {
        println!("{:?}", evt);
        match evt {
            GameEvent::MechSpawned { mech, position } => {
                Ok(MatchState::insert_mech(state, mech, position))
            }
            GameEvent::PositionUpdated { position, mech } => {
                Ok(MatchState::modify_mech(state, mech, |m| MechState {
                    position: position.clone(),
                    ..m
                }))
            }
            GameEvent::DamageTaken {
                damage_target,
                damage,
                ..
            } => Ok(MatchState::modify_mech(state, damage_target, |m| {
                MechState {
                    health: m.health - damage.min(&m.health),
                    ..m
                }
            })),
            GameEvent::MechDestroyed { damage_target, .. } => {
                Ok(MatchState::modify_mech(state, damage_target, |m| {
                    MechState {
                        alive: false,
                        health: 0,
                        ..m
                    }
                }))
            }
            GameEvent::MatchTurnCompleted { new_turn } => {
                Ok(MatchState::advance_match_turn(state, *new_turn))
            }
            GameEvent::MechTurnCompleted { mech, .. } => {
                Ok(MatchState::advance_mech_turn(state, mech))
            }
            GameEvent::GameFinished { cause } => Ok(MatchState::finish_game(state, cause)),
        }
    }

    fn handle_command(state: &Self::State, cmd: &Self::Command) -> Result<Vec<Self::Event>> {
        match cmd {
            MechCommand::Move {
                mech, direction, ..
            } => Self::handle_move(state, mech, direction),
            MechCommand::FirePrimary {
                mech, direction, ..
            } => Self::handle_fire_primary(state, mech, direction),
            MechCommand::FireSecondary {
                mech, direction, ..
            } => Self::handle_fire_secondary(state, mech, direction),
            MechCommand::RequestRadarScan { mech, .. } => Self::handle_radar(state, mech),
            MechCommand::SpawnMech { mech, position, .. } => Ok(vec![GameEvent::MechSpawned {
                mech: mech.to_string(),
                position: position.clone(),
            }]),
            MechCommand::FinishTurn { mech, turn } => Self::handle_turn_finish(state, mech, *turn),
        }
    }
}

impl Match {
    fn handle_move(
        state: &<Match as Aggregate>::State,
        mech: &str,
        direction: &GridDirection,
    ) -> Result<Vec<<Match as Aggregate>::Event>> {
        MatchState::validate_has_mech(state, mech)?;
        match state.mechs[mech]
            .position
            .relative_point(&state.game_board, direction, 1)
        {
            Some(p) => Ok(vec![GameEvent::PositionUpdated {
                mech: mech.to_string(),
                position: p,
            }]),
            None => Ok(vec![GameEvent::DamageTaken {
                damage_target: mech.to_string(),
                damage: WALL_DAMAGE,
                damage_source: DamageSource::Wall,
            }]),
        }
    }

    fn handle_turn_finish(
        state: &<Match as Aggregate>::State,
        mech: &str,
        turn: u32,
    ) -> Result<Vec<<Match as Aggregate>::Event>> {
        MatchState::validate_has_mech(state, mech)?;
        if state.turn_status.taken.contains(mech) && state.turn_status.current == turn {
            Err(eventsourcing::Error {
                kind: eventsourcing::Kind::CommandFailure(
                    "Cannot mark the same turn completed multiple times for the same mech"
                        .to_string(),
                ),
            })
        } else {
            let mut evts = Vec::new();
            evts.push(GameEvent::MechTurnCompleted {
                mech: mech.to_string(),
                turn,
            });
            if state.turn_status.taken.len() == state.parameters.actors.len() - 1 {
                // this state won't change until the event is processed, so the count is down by 1
                evts.push(GameEvent::MatchTurnCompleted {
                    new_turn: state.turn_status.current + 1,
                });
            }
            // if completing this turn will bump the current turn to the max turns, then we're done
            if state.turn_status.current == state.parameters.max_turns - 1 {
                evts.push(GameEvent::GameFinished {
                    cause: EndCause::MaxTurnsCompleted,
                });
            }
            Ok(evts)
        }
    }

    fn handle_fire_primary(
        state: &<Match as Aggregate>::State,
        mech: &str,
        dir: &GridDirection,
    ) -> Result<Vec<<Match as Aggregate>::Event>> {
        let mut evts = Vec::new();
        MatchState::validate_has_mech(state, mech)?;
        let targets: Vec<_> = state.mechs[mech]
            .position
            .gather_points(&state.game_board, dir, PRIMARY_RANGE)
            .iter()
            .filter_map(|p| MatchState::mech_at(state, p))
            .collect();
        if targets.len() > 0 {
            evts.extend(Self::do_damage(
                state,
                DamageSource::Mech(mech.to_string()),
                &targets[0].id,
                PRIMARY_DAMAGE,
                targets[0].health,
            ));
        }
        Ok(evts)
    }

    fn handle_fire_secondary(
        state: &<Match as Aggregate>::State,
        mech: &str,
        dir: &GridDirection,
    ) -> Result<Vec<<Match as Aggregate>::Event>> {
        MatchState::validate_has_mech(state, mech)?;
        let mut events = Vec::new();
        let targets: Vec<_> = state.mechs[mech]
            .position
            .gather_points(&state.game_board, dir, SECONDARY_RANGE)
            .iter()
            .filter_map(|p| MatchState::mech_at(state, p))
            .collect();
        let splash_origin: Option<Point> = if targets.len() > 0 {
            // Projectile stopped at a target
            events.extend(Self::do_damage(
                state,
                DamageSource::Mech(mech.to_string()),
                &targets[0].id,
                SECONDARY_DAMAGE,
                targets[0].health,
            ));
            Some(targets[0].position.clone())
        } else {
            // Projectile flew unobstructed in target direction, which could
            // be a point off the board
            state.mechs[mech].position.relative_point(
                &state.game_board,
                dir,
                SECONDARY_RANGE as i32,
            )
        };

        // landing zone could've been off the board
        if let Some(splash_origin) = splash_origin {
            // Apply splash damage to any mech adjacent to this point, even if target spot was empty
            events.extend(
                splash_origin
                    .adjacent_points(&state.game_board)
                    .iter()
                    .filter_map(|p| MatchState::mech_at(state, p))
                    .flat_map(|m| {
                        Self::do_damage(
                            state,
                            DamageSource::Mech(mech.to_string()),
                            &m.id,
                            SECONDARY_SPLASH_DAMAGE,
                            m.health,
                        )
                    })
                    .collect::<Vec<_>>(),
            );
        }

        Ok(events)
    }

    fn do_damage(
        state: &MatchState,
        from: DamageSource,
        to: &str,
        amt: u32,
        remaining_health: u32,
    ) -> Vec<<Match as Aggregate>::Event> {
        let mut evts = Vec::new();
        evts.push(GameEvent::DamageTaken {
            damage: amt,
            damage_source: from.clone(),
            damage_target: to.to_string(),
        });
        if amt >= remaining_health {
            evts.push(GameEvent::MechDestroyed {
                damage_target: to.to_string(),
                damage_source: from.clone(),
            });
            let remaining_mechs = Self::living_mechs(state)
                .into_iter()
                .filter(|m| m != &to)
                .collect::<Vec<_>>();
            if remaining_mechs.len() == 1 {
                // Game Over
                evts.push(GameEvent::GameFinished {
                    cause: EndCause::MechVictory(remaining_mechs[0].clone()),
                })
            }
        }
        evts
    }

    fn living_mechs(state: &MatchState) -> Vec<String> {
        state
            .clone()
            .mechs
            .into_iter()
            .filter_map(|(id, m)| if m.alive { Some(id) } else { None })
            .collect()
    }

    fn handle_radar(
        _state: &<Match as Aggregate>::State,
        _mech: &str,
    ) -> Result<Vec<<Match as Aggregate>::Event>> {
        Ok(vec![])
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::eventsourcing::Aggregate;

    fn gen_root_state(mechs: Vec<(&str, Point)>, max_turns: u32) -> MatchState {
        let mut state = MatchState::new_with_parameters(MatchParameters {
            actors: mechs.iter().map(|(a, _p)| a.to_string()).collect(),
            match_id: "test_match".to_string(),
            max_turns: max_turns,
            height: 24,
            width: 24,
        });

        for (mech, position) in mechs {
            let cmd = MechCommand::SpawnMech {
                mech: mech.to_string(),
                position: position.clone(),
            };
            for event in Match::handle_command(&state, &cmd).unwrap() {
                state = Match::apply_event(&state, &event).unwrap();
            }
        }
        state
    }

    #[test]
    fn basic_spawn() {
        let state = gen_root_state(
            vec![("bob", Point::new(10, 10)), ("alfred", Point::new(20, 20))],
            10,
        );

        assert_eq!(state.mechs.len(), 2);
        assert_eq!(state.mechs["bob"].position, Point::new(10, 10));
        assert_eq!(state.mechs["alfred"].position, Point::new(20, 20));
        assert_eq!(state.mechs["alfred"].alive, true);
    }

    #[test]
    fn off_board_collision() {
        let state = gen_root_state(vec![("jeeves", Point::new(0, 0))], 10);
        let cmd = MechCommand::Move {
            turn: 0,
            mech: "jeeves".to_string(),
            direction: GridDirection::South,
        };

        let state = Match::handle_command(&state, &cmd)
            .unwrap()
            .iter()
            .fold(state, |state, evt| Match::apply_event(&state, evt).unwrap());

        assert_eq!(state.mechs["jeeves"].health, INITIAL_HEALTH - WALL_DAMAGE);
    }

    #[test]
    fn safe_move() {
        let state = gen_root_state(vec![("jeeves", Point::new(5, 5))], 10);
        let cmd = MechCommand::Move {
            turn: 0,
            mech: "jeeves".to_string(),
            direction: GridDirection::NorthEast,
        };

        let state = Match::handle_command(&state, &cmd)
            .unwrap()
            .iter()
            .fold(state, |state, evt| Match::apply_event(&state, evt).unwrap());

        assert_eq!(state.mechs["jeeves"].position, Point::new(6, 6));
    }

    #[test]
    fn primary_fire() {
        let state = gen_root_state(
            vec![
                ("shooter", Point::new(10, 6)),
                ("victim", Point::new(12, 8)),
            ],
            10,
        );

        let cmd = MechCommand::FirePrimary {
            turn: 0,
            mech: "shooter".to_string(),
            direction: GridDirection::NorthEast,
        };

        let state = Match::handle_command(&state, &cmd)
            .unwrap()
            .iter()
            .fold(state, |state, evt| Match::apply_event(&state, evt).unwrap());

        println!("{:?}", state);
        assert_eq!(
            state.mechs["victim"].health,
            INITIAL_HEALTH - PRIMARY_DAMAGE
        );
    }

    #[test]
    fn secondary_fire() {
        let state = gen_root_state(
            vec![
                ("shooter", Point::new(10, 6)),
                ("victim", Point::new(12, 8)),
            ],
            10,
        );

        let cmd = MechCommand::FireSecondary {
            turn: 0,
            mech: "shooter".to_string(),
            direction: GridDirection::NorthEast,
        };

        let state = Match::handle_command(&state, &cmd)
            .unwrap()
            .iter()
            .fold(state, |state, evt| Match::apply_event(&state, evt).unwrap());

        println!("{:?}", state);
        assert_eq!(
            state.mechs["victim"].health,
            INITIAL_HEALTH - SECONDARY_DAMAGE
        );
    }

    #[test]
    fn secondary_fire_with_splash() {
        let state = gen_root_state(
            vec![
                ("shooter", Point::new(10, 6)),
                ("victim", Point::new(11, 7)),
            ],
            10,
        );

        let cmd = MechCommand::FireSecondary {
            turn: 0,
            mech: "shooter".to_string(),
            direction: GridDirection::NorthEast,
        };

        let state = Match::handle_command(&state, &cmd)
            .unwrap()
            .iter()
            .fold(state, |state, evt| Match::apply_event(&state, evt).unwrap());

        assert_eq!(
            state.mechs["victim"].health,
            INITIAL_HEALTH - SECONDARY_DAMAGE
        );
        assert_eq!(
            state.mechs["shooter"].health,
            INITIAL_HEALTH - SECONDARY_SPLASH_DAMAGE
        );
    }

    #[test]
    fn take_turns() {
        let state = gen_root_state(
            vec![("al", Point::new(10, 6)), ("bob", Point::new(11, 7))],
            10,
        );

        let al1 = MechCommand::FirePrimary {
            turn: 0,
            mech: "al".to_string(),
            direction: GridDirection::South,
        };
        let al2 = MechCommand::FinishTurn {
            mech: "al".to_string(),
            turn: 0,
        };
        let bob1 = MechCommand::Move {
            turn: 0,
            mech: "bob".to_string(),
            direction: GridDirection::South,
        };
        let bob2 = MechCommand::FinishTurn {
            mech: "bob".to_string(),
            turn: 0,
        };

        let cmds = vec![al1.clone(), al2.clone(), bob1, bob2];
        let state = cmds.iter().fold(state, |state, cmd| {
            Match::handle_command(&state, &cmd)
                .unwrap()
                .iter()
                .fold(state, |state, evt| Match::apply_event(&state, evt).unwrap())
        });
        println!("{:?}", state);
        assert_eq!(state.turn_status.current, 1);

        let cmds = vec![al1, al2.clone()];
        let state = gen_root_state(
            vec![("al", Point::new(10, 6)), ("bob", Point::new(11, 7))],
            10,
        );

        let state = cmds.iter().fold(state, |state, cmd| {
            Match::handle_command(&state, &cmd)
                .unwrap()
                .iter()
                .fold(state, |state, evt| Match::apply_event(&state, evt).unwrap())
        });
        // this state should now not accept another turn from Al...
        let evt = Match::handle_command(&state, &al2);
        assert!(evt.is_err());
    }

    #[test]
    fn game_finishes_on_turns() {
        let state = gen_root_state(
            vec![("al", Point::new(10, 6)), ("bob", Point::new(11, 7))],
            10,
        );

        // fill 10 turns of activity
        let mut cmds = Vec::new();
        for i in 0..10 {
            cmds.push(MechCommand::FirePrimary {
                turn: i,
                direction: GridDirection::West,
                mech: "al".to_string(),
            });
            cmds.push(MechCommand::FinishTurn {
                turn: i,
                mech: "al".to_string(),
            });
            cmds.push(MechCommand::FirePrimary {
                turn: i,
                direction: GridDirection::East,
                mech: "bob".to_string(),
            });
            cmds.push(MechCommand::FinishTurn {
                mech: "bob".to_string(),
                turn: i,
            })
        }

        let state = cmds.iter().fold(state, |state, cmd| {
            Match::handle_command(&state, &cmd)
                .unwrap()
                .iter()
                .fold(state, |state, evt| Match::apply_event(&state, evt).unwrap())
        });

        assert_eq!(state.completed.unwrap(), EndCause::MaxTurnsCompleted);
    }

    #[test]
    fn game_finishes_on_victor() {
        let path_to_death = INITIAL_HEALTH / PRIMARY_DAMAGE;
        let state = gen_root_state(
            vec![("al", Point::new(10, 6)), ("bob", Point::new(11, 6))],
            path_to_death + 2,
        );

        let mut cmds = Vec::new();
        for i in 0..path_to_death + 1 {
            cmds.push(MechCommand::FirePrimary {
                turn: i,
                direction: GridDirection::East,
                mech: "al".to_string(),
            });
            cmds.push(MechCommand::FinishTurn {
                turn: i,
                mech: "al".to_string(),
            });
            cmds.push(MechCommand::FinishTurn {
                mech: "bob".to_string(),
                turn: i,
            })
        }

        let state = cmds.iter().fold(state, |state, cmd| {
            Match::handle_command(&state, &cmd)
                .unwrap()
                .iter()
                .fold(state, |state, evt| Match::apply_event(&state, evt).unwrap())
        });

        assert_eq!(
            state.completed.unwrap(),
            EndCause::MechVictory("al".to_string())
        );
    }

    #[test]
    fn death_by_splash() {
        let path_to_death = INITIAL_HEALTH / SECONDARY_SPLASH_DAMAGE;
        let state = gen_root_state(
            vec![("al", Point::new(10, 6)), ("bob", Point::new(16, 11))], // Bob should be within splash range
            path_to_death + 2,
        );

        let mut cmds = Vec::new();
        for i in 0..path_to_death + 1 {
            cmds.push(MechCommand::FireSecondary {
                turn: i,
                direction: GridDirection::NorthEast,
                mech: "al".to_string(),
            });
            cmds.push(MechCommand::FinishTurn {
                turn: i,
                mech: "al".to_string(),
            });
            cmds.push(MechCommand::FinishTurn {
                mech: "bob".to_string(),
                turn: i,
            })
        }

        let state = cmds.iter().fold(state, |state, cmd| {
            Match::handle_command(&state, &cmd)
                .unwrap()
                .iter()
                .fold(state, |state, evt| Match::apply_event(&state, evt).unwrap())
        });

        assert_eq!(
            state.completed.unwrap(),
            EndCause::MechVictory("al".to_string())
        );
    }
}
