use crate::{
    commands::MechCommand,
    events::{EndCause, GameEvent},
    DamageSource, GameBoard, GridDirection, MatchParameters, Point, RadarPing, RegisterOperation,
    RegisterValue, TurnStatus, EAX, EBX, ECX,
};
use eventsourcing::Result;
use eventsourcing::{Aggregate, AggregateState};
use std::collections::HashMap;

const WALL_DAMAGE: u32 = 50; // Lose HP for bouncing off obstacles
const PRIMARY_DAMAGE: u32 = 100;
const SECONDARY_DAMAGE: u32 = 140;
const SECONDARY_SPLASH_DAMAGE: u32 = 90;

const INITIAL_HEALTH: u32 = 1000;
pub const PRIMARY_RANGE: usize = 3;
pub const SECONDARY_RANGE: usize = 6;
pub const APS_PER_TURN: u32 = 4;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MatchState {
    pub parameters: MatchParameters,
    pub mechs: HashMap<String, MechState>,
    pub generation: u64,
    pub game_board: GameBoard,
    pub turn_status: TurnStatus,
    pub completed: Option<EndCause>,
    pub radar_pings: HashMap<String, Vec<RadarPing>>,
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
            radar_pings: HashMap::new(),
        }
    }

    fn validate_has_mech(state: &MatchState, mech: &str) -> Result<()> {
        if !state.mechs.contains_key(mech) {
            Err(eventsourcing::Error {
                kind: eventsourcing::Kind::CommandFailure(
                    "Command received for mech not in match".to_string(),
                ),
            })
        } else {
            Ok(())
        }
    }

    fn validate_can_take_action(state: &MatchState, mech: &str, cmd: &MechCommand) -> Result<()> {
        MatchState::validate_has_mech(state, mech)?;
        if state.mechs[mech].remaining_aps >= cmd.action_points() {
            Ok(())
        } else {
            Err(eventsourcing::Error {
                kind: eventsourcing::Kind::CommandFailure(
                    "Command received for mech would exceed mech's remaining actions".to_string(),
                ),
            })
        }
    }

    fn update_radar(state: &MatchState, actor: &str, pings: &[RadarPing]) -> MatchState {
        let mut state = state.clone();
        state.radar_pings.insert(actor.to_string(), pings.to_vec());
        state
    }

    fn finish_game(state: &MatchState, cause: &EndCause) -> MatchState {
        let mut state = state.clone();
        state.completed = Some(cause.clone());
        state
    }

    fn remaining_alive(state: &MatchState) -> Vec<String> {
        let mechs = state.mechs.clone();

        mechs
            .into_iter()
            .filter(|(_key, m)| m.alive)
            .map(|(key, _m)| key)
            .collect()
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

    pub(crate) fn mech_at(state: &MatchState, position: &Point) -> Option<MechState> {
        state
            .mechs
            .values()
            .find(|m| m.position == *position)
            .cloned()
    }

    pub(crate) fn nearest_unoccupied(
        state: &MatchState,
        position: &Option<Point>,
    ) -> Option<Point> {
        if let Some(position) = position {
            if Self::mech_at(state, position).is_some() {
                vec![
                    position.relative_point(&state.game_board, &GridDirection::North, 1),
                    position.relative_point(&state.game_board, &GridDirection::NorthEast, 1),
                    position.relative_point(&state.game_board, &GridDirection::East, 1),
                    position.relative_point(&state.game_board, &GridDirection::SouthEast, 1),
                    position.relative_point(&state.game_board, &GridDirection::South, 1),
                    position.relative_point(&state.game_board, &GridDirection::SouthWest, 1),
                    position.relative_point(&state.game_board, &GridDirection::West, 1),
                    position.relative_point(&state.game_board, &GridDirection::NorthWest, 1),
                ]
                .iter()
                .find_map(|p| Self::nearest_unoccupied(state, p))
            } else {
                Some(position.clone())
            }
        } else {
            None
        }
    }

    fn insert_mech(
        state: &MatchState,
        mech: &str,
        position: &Point,
        team: &str,
        avatar: &str,
        name: &str,
    ) -> MatchState {
        let mut state = state.clone();
        state.mechs.insert(
            mech.to_string(),
            MechState {
                position: position.clone(),
                id: mech.to_string(),
                team: team.to_string(),
                avatar: avatar.to_string(),
                name: name.to_string(),
                ..Default::default()
            },
        );
        MatchState {
            mechs: state.mechs,
            ..state
        }
    }

    fn advance_match_turn(state: &MatchState, turn: u32) -> MatchState {
        let mut state = MatchState::reset_mech_action_points(state);
        state.turn_status.taken.clear();
        state.turn_status.current = turn;
        state
    }

    fn advance_mech_turn(state: &MatchState, mech: &str) -> MatchState {
        let mut state = state.clone();
        state.turn_status.taken.insert(mech.to_string());
        state
    }

    fn deduct_mech_action_points(
        state: &MatchState,
        mech: &str,
        points_consumed: &u32,
    ) -> MatchState {
        let mut state = state.clone();
        let mech_state = state.mechs[mech].clone();
        state.mechs.insert(
            mech.to_string(),
            MechState {
                remaining_aps: mech_state.remaining_aps - points_consumed,
                ..mech_state
            },
        );
        state
    }

    fn reset_mech_action_points(state: &MatchState) -> MatchState {
        let mut state = state.clone();
        state.mechs = state
            .mechs
            .clone()
            .into_iter()
            .map(|(mech, mech_state)| {
                (
                    mech.clone(),
                    MechState {
                        remaining_aps: state.parameters.aps_per_turn,
                        ..mech_state.clone()
                    },
                )
            })
            .collect::<HashMap<_, _>>();
        state
    }

    fn update_register(
        state: &MatchState,
        mech: &str,
        reg: &u32,
        val: &RegisterValue,
    ) -> MatchState {
        let mut state = state.clone();
        let mech_state = state.mechs[mech].clone();
        let mut registers = mech_state.registers;
        registers.insert(*reg, val.clone());
        state.mechs.insert(
            mech.to_string(),
            MechState {
                registers,
                ..mech_state
            },
        );
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
    pub team: String,
    pub avatar: String,
    pub name: String,
    pub remaining_aps: u32,
    pub registers: HashMap<u32, RegisterValue>,
}

impl Default for MechState {
    fn default() -> Self {
        MechState::new(
            INITIAL_HEALTH,
            Point::new(0, 0),
            true,
            false,
            "None".to_string(),
            "earth".to_string(),
            "none".to_string(),
            "Anonymous".to_string(),
            4,
            HashMap::new(),
        )
    }
}

impl MechState {
    fn new(
        health: u32,
        position: Point,
        alive: bool,
        victor: bool,
        id: String,
        team: String,
        avatar: String,
        name: String,
        remaining_aps: u32,
        registers: HashMap<u32, RegisterValue>,
    ) -> Self {
        MechState {
            health,
            position,
            alive,
            victor,
            id,
            team,
            avatar,
            name,
            remaining_aps,
            registers,
        }
    }
}

pub struct Match;
impl Aggregate for Match {
    type Event = GameEvent;
    type Command = MechCommand;
    type State = MatchState;

    fn apply_event(state: &Self::State, evt: &Self::Event) -> Result<Self::State> {
        match evt {
            GameEvent::MechSpawned {
                mech,
                position,
                team,
                avatar,
                name,
            } => Ok(MatchState::insert_mech(
                state, mech, position, team, avatar, name,
            )),
            GameEvent::RadarScanCompleted { actor, results } => {
                Ok(MatchState::update_radar(state, actor, results))
            }
            GameEvent::PositionUpdated { position, mech } => {
                Ok(MatchState::modify_mech(state, mech, |m| MechState {
                    position: position.clone(),
                    ..m
                }))
            }
            GameEvent::ActionPointsConsumed {
                mech,
                points_consumed,
            } => Ok(MatchState::deduct_mech_action_points(
                state,
                mech,
                points_consumed,
            )),
            GameEvent::ActionPointsExceeded { .. } => {
                //TODO: Handle the penalty for exceeding action points appropriately
                Ok(state.clone())
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
                Ok(MatchState::advance_match_turn(&state, *new_turn))
            }
            GameEvent::MechTurnCompleted { mech, .. } => {
                Ok(MatchState::advance_mech_turn(state, mech))
            }
            GameEvent::GameFinished { cause } => Ok(MatchState::finish_game(state, cause)),
            GameEvent::RegisterUpdate { mech, reg, val } => {
                Ok(MatchState::update_register(state, mech, reg, val))
            }
        }
    }

    fn handle_command(state: &Self::State, cmd: &Self::Command) -> Result<Vec<Self::Event>> {
        use MechCommand::*;
        match cmd {
            Move { mech, .. }
            | FirePrimary { mech, .. }
            | FireSecondary { mech, .. }
            | RequestRadarScan { mech, .. }
                if MatchState::validate_can_take_action(state, mech, cmd).is_err() =>
            {
                return Ok(vec![GameEvent::ActionPointsExceeded {
                    mech: mech.to_string(),
                    cmd: cmd.clone(),
                }])
            }
            Move {
                mech, direction, ..
            } => Self::handle_move(state, mech, direction, cmd),
            FirePrimary {
                mech, direction, ..
            } => Self::handle_fire_primary(state, mech, direction, cmd),
            FireSecondary {
                mech, direction, ..
            } => Self::handle_fire_secondary(state, mech, direction, cmd),
            RequestRadarScan { mech, .. } => Self::handle_radar(state, mech, cmd),
            SpawnMech {
                mech,
                position,
                team,
                avatar,
                name,
            } => Ok(vec![GameEvent::MechSpawned {
                mech: mech.to_string(),
                position: MatchState::nearest_unoccupied(state, &Some(position.clone())).unwrap(),
                team: team.to_string(),
                avatar: avatar.to_string(),
                name: name.to_string(),
            }]),
            FinishTurn { mech, turn } => Self::handle_turn_finish(state, mech, *turn),
            RegisterUpdate { .. } => Self::handle_register_update(state, cmd),
        }
    }
}

impl Match {
    fn handle_move(
        state: &<Match as Aggregate>::State,
        mech: &str,
        dir: &GridDirection,
        cmd: &MechCommand,
    ) -> Result<Vec<<Match as Aggregate>::Event>> {
        MatchState::validate_has_mech(state, mech)?;
        Ok(vec![
            match state.mechs[mech]
                .position
                .relative_point(&state.game_board, dir, 1)
            {
                Some(p) => {
                    if let Some(m) = MatchState::mech_at(state, &p) {
                        GameEvent::DamageTaken {
                            damage: WALL_DAMAGE,
                            damage_source: DamageSource::MechCollision(m.name.to_string()),
                            damage_target: mech.to_string(),
                        }
                    } else {
                        GameEvent::PositionUpdated {
                            mech: mech.to_string(),
                            position: p,
                        }
                    }
                }
                None => GameEvent::DamageTaken {
                    damage_target: mech.to_string(),
                    damage: WALL_DAMAGE,
                    damage_source: DamageSource::Wall,
                },
            },
            GameEvent::ActionPointsConsumed {
                mech: mech.to_string(),
                points_consumed: cmd.action_points(),
            },
        ])
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
                    cause: EndCause::MaxTurnsCompleted {
                        survivors: MatchState::remaining_alive(state),
                    },
                });
            }
            Ok(evts)
        }
    }

    fn handle_fire_primary(
        state: &<Match as Aggregate>::State,
        mech: &str,
        dir: &GridDirection,
        cmd: &MechCommand,
    ) -> Result<Vec<<Match as Aggregate>::Event>> {
        MatchState::validate_has_mech(state, mech)?;
        let mut evts = Vec::new();
        let targets: Vec<_> = state.mechs[mech]
            .position
            .gather_points(&state.game_board, dir, PRIMARY_RANGE)
            .iter()
            .filter_map(|(p, _d)| MatchState::mech_at(state, p))
            .collect();
        if targets.len() > 0 {
            evts.extend(Self::do_damage(
                state,
                DamageSource::MechWeapon(mech.to_string()),
                &targets[0].id,
                PRIMARY_DAMAGE,
                targets[0].health,
            ));
        }
        evts.push(GameEvent::ActionPointsConsumed {
            mech: mech.to_string(),
            points_consumed: cmd.action_points(),
        });
        Ok(evts)
    }

    fn handle_fire_secondary(
        state: &<Match as Aggregate>::State,
        mech: &str,
        dir: &GridDirection,
        cmd: &MechCommand,
    ) -> Result<Vec<<Match as Aggregate>::Event>> {
        MatchState::validate_has_mech(state, mech)?;
        let mut evts = Vec::new();
        let targets: Vec<_> = state.mechs[mech]
            .position
            .gather_points(&state.game_board, dir, SECONDARY_RANGE)
            .iter()
            .filter_map(|(p, _d)| MatchState::mech_at(state, p))
            .collect();
        let splash_origin: Option<Point> = if targets.len() > 0 {
            // Projectile stopped at a target
            evts.extend(Self::do_damage(
                state,
                DamageSource::MechWeapon(mech.to_string()),
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
            evts.extend(
                splash_origin
                    .adjacent_points(&state.game_board)
                    .iter()
                    .filter_map(|p| MatchState::mech_at(state, p))
                    .flat_map(|m| {
                        Self::do_damage(
                            state,
                            DamageSource::MechWeapon(mech.to_string()),
                            &m.id,
                            SECONDARY_SPLASH_DAMAGE,
                            m.health,
                        )
                    })
                    .collect::<Vec<_>>(),
            );
        }

        evts.push(GameEvent::ActionPointsConsumed {
            mech: mech.to_string(),
            points_consumed: cmd.action_points(),
        });
        Ok(evts)
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
        state: &<Match as Aggregate>::State,
        mech: &str,
        cmd: &MechCommand,
    ) -> Result<Vec<<Match as Aggregate>::Event>> {
        let pings =
            crate::radar::radar_ping(state, &state.mechs[mech].position, &state.mechs[mech].team);
        Ok(vec![
            GameEvent::RadarScanCompleted {
                actor: mech.to_string(),
                results: pings,
            },
            GameEvent::ActionPointsConsumed {
                mech: mech.to_string(),
                points_consumed: cmd.action_points(),
            },
        ])
    }

    fn handle_register_update(
        state: &<Match as Aggregate>::State,
        cmd: &MechCommand,
    ) -> Result<Vec<<Match as Aggregate>::Event>> {
        let event = if let MechCommand::RegisterUpdate { mech, reg, op, .. } = cmd {
            let curr_val = state.mechs[mech].registers.get(reg);
            match op {
                RegisterOperation::Accumulate(acc) if *reg == EAX || *reg == ECX => {
                    if let Some(RegisterValue::Number(n)) = curr_val {
                        // Prevent positive overflow
                        let val = if u64::MAX - n < *acc {
                            u64::MAX
                        } else {
                            n + acc
                        };
                        vec![GameEvent::RegisterUpdate {
                            mech: mech.to_string(),
                            reg: reg.clone(),
                            val: RegisterValue::Number(val),
                        }]
                    } else {
                        vec![]
                    }
                }
                RegisterOperation::Decrement(dec) if *reg == EAX || *reg == ECX => {
                    if let Some(RegisterValue::Number(n)) = curr_val {
                        // Prevent negative overflow
                        let val = if n < dec { 0 } else { n - dec };
                        vec![GameEvent::RegisterUpdate {
                            mech: mech.to_string(),
                            reg: reg.clone(),
                            val: RegisterValue::Number(val),
                        }]
                    } else {
                        vec![]
                    }
                }
                RegisterOperation::Set(v) => match v {
                    RegisterValue::Number(_n) if *reg == EAX || *reg == ECX => {
                        vec![GameEvent::RegisterUpdate {
                            mech: mech.to_string(),
                            reg: reg.clone(),
                            val: v.clone(),
                        }]
                    }
                    RegisterValue::Text(_s) if *reg == EBX => vec![GameEvent::RegisterUpdate {
                        mech: mech.to_string(),
                        reg: reg.clone(),
                        val: v.clone(),
                    }],
                    _ => vec![],
                },
                _ => vec![],
            }
        } else {
            vec![]
        };
        Ok(event)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::eventsourcing::Aggregate;
    use crate::radar;
    use crate::radar::RadarPing;

    fn gen_root_state(mechs: Vec<(&str, Point)>, max_turns: u32) -> MatchState {
        let mut state = MatchState::new_with_parameters(MatchParameters {
            actors: mechs.iter().map(|(a, _p)| a.to_string()).collect(),
            match_id: "test_match".to_string(),
            max_turns: max_turns,
            aps_per_turn: 4,
            height: 24,
            width: 24,
        });

        for (mech, position) in mechs {
            let cmd = MechCommand::SpawnMech {
                mech: mech.to_string(),
                position: position.clone(),
                team: "earth".to_string(),
                avatar: "none".to_string(),
                name: format!("{}'s Mech", mech),
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

        if let EndCause::MaxTurnsCompleted { survivors } = state.completed.unwrap() {
            assert_eq!(survivors.len(), 2);
        } else {
            assert!(false);
        }
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

    #[test]
    fn radar_ping_basic() {
        let state = gen_root_state(
            vec![
                ("al", Point::new(10, 6)),
                ("bob", Point::new(13, 9)),
                ("steve", Point::new(14, 6)),
                ("nobody", Point::new(16, 9)),
            ],
            10,
        );
        let origin = Point::new(10, 6); // al's position
        let results = radar::collect_targets(&state, &origin);
        assert_eq!(results.len(), 2);
        assert_eq!(
            results.into_iter().map(|(m, _d)| m.id).collect::<Vec<_>>(),
            vec!["bob", "steve"]
        );
    }

    #[test]
    fn radar_ping_full() {
        let mut state = gen_root_state(
            vec![
                ("al", Point::new(10, 6)),
                ("bob", Point::new(13, 9)),
                ("steve", Point::new(14, 6)),
                ("nobody", Point::new(16, 9)),
            ],
            10,
        );
        state.mechs.get_mut("steve").unwrap().team = "boylur".to_string();
        let origin = Point::new(10, 6); // al's position
        let results = radar::radar_ping(&state, &origin, "earth");
        assert_eq!(
            results,
            vec![
                RadarPing {
                    name: "bob's Mech".to_string(),
                    avatar: "none".to_string(),
                    foe: false,
                    distance: 3,
                    location: Point::new(13, 9),
                },
                RadarPing {
                    name: "steve's Mech".to_string(),
                    avatar: "none".to_string(),
                    foe: true,
                    distance: 4,
                    location: Point::new(14, 6),
                }
            ]
        )
    }

    #[test]
    fn move_into_occupied_spot_causes_collision_dmg() {
        let state: MatchState = gen_root_state(
            vec![("al", Point::new(10, 6)), ("bob", Point::new(11, 6))],
            10,
        );

        let cmds = vec![MechCommand::Move {
            direction: GridDirection::East,
            mech: "al".to_string(),
            turn: 0,
        }];

        let state = cmds.iter().fold(state, |state, cmd| {
            Match::handle_command(&state, &cmd)
                .unwrap()
                .iter()
                .fold(state, |state, evt| Match::apply_event(&state, evt).unwrap())
        });

        assert_eq!(state.mechs["al"].health, INITIAL_HEALTH - WALL_DAMAGE);
        assert_eq!(
            state.mechs["al"].position,
            Point::new(10, 6) // he did not move to new location
        );
    }

    #[test]
    fn radar_ping_state() {
        let state = gen_root_state(
            vec![
                ("al", Point::new(10, 6)),
                ("bob", Point::new(13, 9)),
                ("steve", Point::new(14, 6)),
                ("nobody", Point::new(16, 9)),
            ],
            10,
        );

        let cmds = vec![
            MechCommand::RequestRadarScan {
                mech: "al".to_string(),
                turn: 0,
            },
            MechCommand::RequestRadarScan {
                mech: "nobody".to_string(),
                turn: 0,
            },
        ];

        let state = cmds.iter().fold(state, |state, cmd| {
            Match::handle_command(&state, &cmd)
                .unwrap()
                .iter()
                .fold(state, |state, evt| Match::apply_event(&state, evt).unwrap())
        });

        assert_eq!(
            state.radar_pings["al"],
            vec![
                RadarPing {
                    name: "bob's Mech".to_string(),
                    avatar: "none".to_string(),
                    foe: false,
                    distance: 3,
                    location: Point::new(13, 9),
                },
                RadarPing {
                    name: "steve's Mech".to_string(),
                    avatar: "none".to_string(),
                    foe: false,
                    distance: 4,
                    location: Point::new(14, 6),
                }
            ]
        );

        assert_eq!(
            state.radar_pings["nobody"],
            vec![RadarPing {
                name: "bob's Mech".to_string(),
                avatar: "none".to_string(),
                foe: false,
                distance: 3,
                location: Point::new(13, 9),
            },]
        )
    }

    #[test]
    fn nearest_unoccupied() {
        let state = gen_root_state(
            vec![
                ("al", Point::new(10, 6)),
                ("bob", Point::new(10, 7)),
                ("steve", Point::new(10, 5)),
                ("nobody", Point::new(9, 6)),
            ],
            10,
        );

        assert_eq!(None, MatchState::nearest_unoccupied(&state, &None));
        assert_eq!(
            Some(Point::new(10, 8)),
            MatchState::nearest_unoccupied(&state, &Some(Point::new(10, 7)))
        );
        assert_eq!(
            Some(Point::new(9, 7)),
            MatchState::nearest_unoccupied(&state, &Some(Point::new(9, 6)))
        );
    }

    #[test]
    fn cannot_spawn_on_occupied() {
        let state = gen_root_state(
            vec![
                ("al", Point::new(10, 6)),
                ("bob", Point::new(10, 7)),
                ("steve", Point::new(10, 5)),
                ("nobody", Point::new(9, 6)),
            ],
            10,
        );

        let cmds = vec![MechCommand::SpawnMech {
            position: Point::new(10, 6),
            avatar: "".to_string(),
            mech: "bounce".to_string(),
            name: "test".to_string(),
            team: "earth".to_string(),
        }];

        let state = cmds.iter().fold(state, |state, cmd| {
            Match::handle_command(&state, &cmd)
                .unwrap()
                .iter()
                .fold(state, |state, evt| Match::apply_event(&state, evt).unwrap())
        });

        assert_eq!(Point::new(10, 8), state.mechs["bounce"].position); // Go north until we can't, then find adjacent
    }
    #[test]
    fn action_points_limit() {
        let state = gen_root_state(
            vec![("al", Point::new(10, 6)), ("bob", Point::new(16, 11))], // Bob should be within splash range
            2,
        );

        let mut cmds = Vec::new();
        cmds.push(MechCommand::FireSecondary {
            turn: 0,
            direction: GridDirection::NorthEast,
            mech: "al".to_string(),
        });
        cmds.push(MechCommand::FirePrimary {
            turn: 0,
            direction: GridDirection::NorthEast,
            mech: "al".to_string(),
        });
        cmds.push(MechCommand::Move {
            turn: 0,
            direction: GridDirection::NorthEast,
            mech: "al".to_string(),
        });
        cmds.push(MechCommand::Move {
            turn: 0,
            direction: GridDirection::NorthEast,
            mech: "bob".to_string(),
        });
        cmds.push(MechCommand::Move {
            turn: 0,
            direction: GridDirection::NorthEast,
            mech: "bob".to_string(),
        });
        cmds.push(MechCommand::FirePrimary {
            turn: 0,
            direction: GridDirection::NorthEast,
            mech: "bob".to_string(),
        });
        cmds.push(MechCommand::Move {
            turn: 0,
            direction: GridDirection::SouthWest,
            mech: "bob".to_string(),
        });
        cmds.push(MechCommand::FinishTurn {
            turn: 0,
            mech: "al".to_string(),
        });
        cmds.push(MechCommand::FinishTurn {
            mech: "bob".to_string(),
            turn: 0,
        });

        let state = cmds.iter().fold(state, |state, cmd| {
            Match::handle_command(&state, &cmd)
                .unwrap()
                .iter()
                .fold(state, |state, evt| Match::apply_event(&state, evt).unwrap())
        });

        assert_eq!(
            state.mechs["bob"].health,
            INITIAL_HEALTH - SECONDARY_SPLASH_DAMAGE
        );

        let al_position = Point::new(10, 6);
        let bob_position = Point::new(18, 13);
        assert_eq!(state.mechs["al"].position.x, al_position.x);
        assert_eq!(state.mechs["al"].position.y, al_position.y);
        assert_eq!(state.mechs["bob"].position.x, bob_position.x);
        assert_eq!(state.mechs["bob"].position.y, bob_position.y);
    }

    #[test]
    fn action_points_limit_prevents_death() {
        let state = gen_root_state(
            vec![("al", Point::new(10, 6)), ("bob", Point::new(11, 6))], // Bob should be within splash range
            2,
        );

        let mut cmds = Vec::new();
        for i in 0..20 {
            cmds.push(MechCommand::FirePrimary {
                turn: i,
                direction: GridDirection::East,
                mech: "al".to_string(),
            });
        }
        cmds.push(MechCommand::FinishTurn {
            turn: 0,
            mech: "al".to_string(),
        });
        cmds.push(MechCommand::FinishTurn {
            mech: "bob".to_string(),
            turn: 0,
        });

        let state = cmds.iter().fold(state, |state, cmd| {
            Match::handle_command(&state, &cmd)
                .unwrap()
                .iter()
                .fold(state, |state, evt| Match::apply_event(&state, evt).unwrap())
        });

        assert_eq!(
            state.mechs["bob"].health,
            INITIAL_HEALTH - (PRIMARY_DAMAGE * 2)
        );
    }

    #[test]
    fn register_acc_modifies_mech_state() {
        let mech1 = "johnny";
        let mech2 = "bob";
        let state = gen_root_state(
            vec![(mech1, Point::new(10, 6)), (mech2, Point::new(11, 6))],
            2,
        );
        let mut cmds = Vec::new();
        cmds.push(MechCommand::RegisterUpdate {
            mech: mech1.to_string(),
            reg: EAX,
            op: RegisterOperation::Set(RegisterValue::Number(100)),
            turn: 0,
        });

        cmds.push(MechCommand::RegisterUpdate {
            mech: mech1.to_string(),
            reg: EAX,
            op: RegisterOperation::Set(RegisterValue::Number(30)),
            turn: 0,
        });

        cmds.push(MechCommand::RegisterUpdate {
            mech: mech1.to_string(),
            reg: EAX,
            op: RegisterOperation::Accumulate(25),
            turn: 0,
        });

        cmds.push(MechCommand::RegisterUpdate {
            mech: mech1.to_string(),
            reg: EAX,
            op: RegisterOperation::Accumulate(10),
            turn: 0,
        });

        let state = cmds.iter().fold(state, |state, cmd| {
            Match::handle_command(&state, &cmd)
                .unwrap()
                .iter()
                .fold(state, |state, evt| Match::apply_event(&state, evt).unwrap())
        });

        if let RegisterValue::Number(mech1eax) = state.mechs[mech1].registers.get(&EAX).unwrap() {
            assert_eq!(*mech1eax, 65);
            assert_ne!(*mech1eax, 30);
            assert_ne!(*mech1eax, 55);
        } else {
            panic!("Mech 1 EAX register was not successfully incremented");
        };
    }

    // Registers test
    #[test]
    fn register_dec_modifies_mech_state() {
        let mech1 = "johnny";
        let mech2 = "bob";
        let state = gen_root_state(
            vec![(mech1, Point::new(10, 6)), (mech2, Point::new(11, 6))],
            2,
        );
        let mut cmds = Vec::new();
        cmds.push(MechCommand::RegisterUpdate {
            mech: mech1.to_string(),
            reg: EAX,
            op: RegisterOperation::Set(RegisterValue::Number(30)),
            turn: 0,
        });

        cmds.push(MechCommand::RegisterUpdate {
            mech: mech1.to_string(),
            reg: EAX,
            op: RegisterOperation::Decrement(25),
            turn: 0,
        });

        cmds.push(MechCommand::RegisterUpdate {
            mech: mech1.to_string(),
            reg: EAX,
            op: RegisterOperation::Decrement(2),
            turn: 0,
        });

        let state = cmds.iter().fold(state, |state, cmd| {
            Match::handle_command(&state, &cmd)
                .unwrap()
                .iter()
                .fold(state, |state, evt| Match::apply_event(&state, evt).unwrap())
        });

        if let RegisterValue::Number(mech1eax) = state.mechs[mech1].registers.get(&EAX).unwrap() {
            assert_eq!(*mech1eax, 3);
            assert_ne!(*mech1eax, 30);
            assert_ne!(*mech1eax, 25);
            assert_ne!(*mech1eax, 5);
        } else {
            panic!("Mech 1 EAX register was not successfully decremented");
        };
    }

    #[test]
    fn register_dec_negative_overflow() {
        let mech1 = "johnny";
        let mech2 = "bob";
        let state = gen_root_state(
            vec![(mech1, Point::new(10, 6)), (mech2, Point::new(11, 6))],
            2,
        );
        let mut cmds = Vec::new();
        cmds.push(MechCommand::RegisterUpdate {
            mech: mech1.to_string(),
            reg: EAX,
            op: RegisterOperation::Set(RegisterValue::Number(30)),
            turn: 0,
        });

        for i in 0..10 {
            cmds.push(MechCommand::RegisterUpdate {
                mech: mech1.to_string(),
                reg: EAX,
                op: RegisterOperation::Decrement((i * 10).into()),
                turn: i,
            });
        }

        let state = cmds.iter().fold(state, |state, cmd| {
            Match::handle_command(&state, &cmd)
                .unwrap()
                .iter()
                .fold(state, |state, evt| Match::apply_event(&state, evt).unwrap())
        });

        if let RegisterValue::Number(mech1eax) = state.mechs[mech1].registers.get(&EAX).unwrap() {
            assert_eq!(*mech1eax, 0);
            assert_ne!(*mech1eax, 30);
        } else {
            panic!("Mech 1 EAX register was not successfully decremented");
        };
    }

    #[test]
    fn register_acc_positive_overflow() {
        let mech1 = "johnny";
        let mech2 = "bob";
        let state = gen_root_state(
            vec![(mech1, Point::new(10, 6)), (mech2, Point::new(11, 6))],
            2,
        );
        let mut cmds = Vec::new();
        cmds.push(MechCommand::RegisterUpdate {
            mech: mech1.to_string(),
            reg: EAX,
            op: RegisterOperation::Set(RegisterValue::Number(u64::MAX - 100)),
            turn: 0,
        });

        for i in 0..10 {
            cmds.push(MechCommand::RegisterUpdate {
                mech: mech1.to_string(),
                reg: EAX,
                op: RegisterOperation::Accumulate((i * 10).into()),
                turn: i,
            });
        }

        let state = cmds.iter().fold(state, |state, cmd| {
            Match::handle_command(&state, &cmd)
                .unwrap()
                .iter()
                .fold(state, |state, evt| Match::apply_event(&state, evt).unwrap())
        });

        if let RegisterValue::Number(mech1eax) = state.mechs[mech1].registers.get(&EAX).unwrap() {
            assert_eq!(*mech1eax, u64::MAX);
            assert_ne!(*mech1eax, 0);
        } else {
            panic!("Mech 1 EAX register was not successfully incremented");
        };
    }

    #[test]
    fn register_multiple_changes_test() {
        let mech1 = "johnny";
        let mech2 = "bob";
        let state = gen_root_state(
            vec![(mech1, Point::new(10, 6)), (mech2, Point::new(11, 6))],
            2,
        );
        let mut cmds = Vec::new();

        let mut register_val: u64 = 123;

        cmds.push(MechCommand::RegisterUpdate {
            mech: mech1.to_string(),
            reg: EAX,
            op: RegisterOperation::Set(RegisterValue::Number(register_val)),
            turn: 0,
        });

        for i in 0..10 {
            let modify_num: u64 = (i * 10).into();
            if i % 2 == 1 {
                register_val += modify_num;
                cmds.push(MechCommand::RegisterUpdate {
                    mech: mech1.to_string(),
                    reg: EAX,
                    op: RegisterOperation::Accumulate(modify_num),
                    turn: i,
                });
            } else {
                register_val -= modify_num;
                cmds.push(MechCommand::RegisterUpdate {
                    mech: mech1.to_string(),
                    reg: EAX,
                    op: RegisterOperation::Decrement(modify_num),
                    turn: i,
                });
            }
        }

        let state = cmds.iter().fold(state, |state, cmd| {
            Match::handle_command(&state, &cmd)
                .unwrap()
                .iter()
                .fold(state, |state, evt| Match::apply_event(&state, evt).unwrap())
        });

        if let RegisterValue::Number(mech1eax) = state.mechs[mech1].registers.get(&EAX).unwrap() {
            assert_eq!(*mech1eax, register_val);
        } else {
            panic!("Mech 1 EAX register was not successfully modified");
        };
    }

    #[test]
    fn register_incorrect_operations_rejected() {
        let mech1 = "johnny";
        let mech2 = "bob";
        let state = gen_root_state(
            vec![(mech1, Point::new(10, 6)), (mech2, Point::new(11, 6))],
            2,
        );
        let mut cmds = Vec::new();

        let eaxval = 123098;
        let ecxval = 87948;
        let ebxval = "boylur_plait".to_string();

        // Valid commands
        cmds.push(MechCommand::RegisterUpdate {
            mech: mech1.to_string(),
            reg: EAX,
            op: RegisterOperation::Set(RegisterValue::Number(eaxval)),
            turn: 0,
        });

        cmds.push(MechCommand::RegisterUpdate {
            mech: mech1.to_string(),
            reg: ECX,
            op: RegisterOperation::Set(RegisterValue::Number(ecxval)),
            turn: 0,
        });

        cmds.push(MechCommand::RegisterUpdate {
            mech: mech1.to_string(),
            reg: EBX,
            op: RegisterOperation::Set(RegisterValue::Text(ebxval.clone())),
            turn: 0,
        });

        // Invalid Commands

        cmds.push(MechCommand::RegisterUpdate {
            mech: mech1.to_string(),
            reg: EAX,
            op: RegisterOperation::Set(RegisterValue::Text("goodhealth".to_string())),
            turn: 0,
        });

        cmds.push(MechCommand::RegisterUpdate {
            mech: mech1.to_string(),
            reg: ECX,
            op: RegisterOperation::Set(RegisterValue::Text("jane".to_string())),
            turn: 0,
        });

        cmds.push(MechCommand::RegisterUpdate {
            mech: mech1.to_string(),
            reg: EBX,
            op: RegisterOperation::Set(RegisterValue::Number(42)),
            turn: 0,
        });

        cmds.push(MechCommand::RegisterUpdate {
            mech: mech1.to_string(),
            reg: EBX,
            op: RegisterOperation::Accumulate(42),
            turn: 0,
        });

        cmds.push(MechCommand::RegisterUpdate {
            mech: mech1.to_string(),
            reg: EBX,
            op: RegisterOperation::Decrement(123),
            turn: 0,
        });

        let state = cmds.iter().fold(state, |state, cmd| {
            Match::handle_command(&state, &cmd)
                .unwrap()
                .iter()
                .fold(state, |state, evt| Match::apply_event(&state, evt).unwrap())
        });

        if let RegisterValue::Number(mech1eax) = state.mechs[mech1].registers.get(&EAX).unwrap() {
            assert_eq!(*mech1eax, eaxval);
            assert_ne!(*mech1eax, ecxval);
        } else {
            panic!("Mech 1 EAX register was modified by an invalid operation");
        };

        if let RegisterValue::Number(mech1ecx) = state.mechs[mech1].registers.get(&ECX).unwrap() {
            assert_eq!(*mech1ecx, ecxval);
            assert_ne!(*mech1ecx, eaxval);
        } else {
            panic!("Mech 1 ECX register was modified by an invalid operation");
        };

        if let RegisterValue::Text(mech1ebx) = state.mechs[mech1].registers.get(&EBX).unwrap() {
            assert_eq!(*mech1ebx, ebxval);
        } else {
            panic!("Mech 1 EBX register was modified by an invalid operation");
        };
    }
}
