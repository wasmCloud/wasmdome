use crate::{
    commands::MechCommand, events::GameEvent, DamageSource, GameBoard, GridDirection, Point,
};
use eventsourcing::Result;
use eventsourcing::{Aggregate, AggregateState};
use std::collections::HashMap;

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
}

impl MatchParameters {
    pub fn new(match_id: String, width: u32, height: u32, actors: Vec<String>) -> Self {
        MatchParameters {
            match_id,
            width,
            height,
            actors,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MatchState {
    pub parameters: MatchParameters,
    pub mechs: HashMap<String, MechState>,
    pub generation: u64,
    pub game_board: GameBoard,
}

impl MatchState {
    pub fn new_with_parameters(params: MatchParameters) -> MatchState {
        MatchState{
            parameters: params.clone(),
            mechs: HashMap::new(),
            generation: 0,
            game_board: GameBoard{
                height: params.height,
                width: params.width,
            }
        }
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
                    health: m.health - damage,
                    ..m
                }
            })),
            GameEvent::MechDestroyed { destroyed_mech } => {
                Ok(MatchState::modify_mech(state, destroyed_mech, |m| {
                    MechState {
                        alive: false,
                        health: 0,
                        ..m
                    }
                }))
            }
            GameEvent::MechWon { mech } => Ok(MatchState::modify_mech(state, mech, |m| {
                MechState { victor: true, ..m }
            })),
        }
    }

    fn handle_command(state: &Self::State, cmd: &Self::Command) -> Result<Vec<Self::Event>> {
        match cmd {
            MechCommand::Move { mech, direction } => Self::handle_move(state, mech, direction),
            MechCommand::FirePrimary { mech, direction } => {
                Self::handle_fire_primary(state, mech, direction)
            }
            MechCommand::FireSecondary { mech, direction } => {
                Self::handle_fire_secondary(state, mech, direction)
            }
            MechCommand::RequestRadarScan { mech } => Self::handle_radar(state, mech),
            MechCommand::SpawnMech { mech, position } => Ok(vec![GameEvent::MechSpawned {
                mech: mech.to_string(),
                position: position.clone(),
            }]),
        }
    }
}

impl Match {
    fn handle_move(
        state: &<Match as Aggregate>::State,
        mech: &str,
        direction: &GridDirection,
    ) -> Result<Vec<<Match as Aggregate>::Event>> {
        match state.mechs[mech]
            .position
            .relative_point(&state.game_board, direction)
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

    fn handle_fire_primary(
        state: &<Match as Aggregate>::State,
        mech: &str,
        dir: &GridDirection,
    ) -> Result<Vec<<Match as Aggregate>::Event>> {
        let targets: Vec<_> = state.mechs[mech]
            .position
            .gather_points(&state.game_board, dir, PRIMARY_RANGE)
            .iter()
            .filter_map(|p| MatchState::mech_at(state, p))
            .collect();
        if targets.len() > 0 {
            Ok(vec![GameEvent::DamageTaken {
                damage: PRIMARY_DAMAGE,
                damage_source: DamageSource::Mech(mech.to_string()),
                damage_target: targets[0].id.clone(),
            }])
        } else {
            Ok(vec![])
        }
    }

    fn handle_fire_secondary(
        state: &<Match as Aggregate>::State,
        mech: &str,
        dir: &GridDirection,
    ) -> Result<Vec<<Match as Aggregate>::Event>> {
        let mut events = Vec::new();
        let targets: Vec<_> = state.mechs[mech]
            .position
            .gather_points(&state.game_board, dir, SECONDARY_RANGE)
            .iter()
            .filter_map(|p| MatchState::mech_at(state, p))
            .collect();
        if targets.len() > 0 {
            events.push(GameEvent::DamageTaken {
                damage: SECONDARY_DAMAGE,
                damage_source: DamageSource::Mech(mech.to_string()),
                damage_target: targets[0].id.clone(),
            });
            // Apply splash damage to any mech adjacent to this point
            events.extend(
                targets[0]
                    .position
                    .adjacent_points(&state.game_board)
                    .iter()
                    .filter_map(|p| MatchState::mech_at(state, p))
                    .map(|m| GameEvent::DamageTaken {
                        damage: SECONDARY_SPLASH_DAMAGE,
                        damage_source: DamageSource::Mech(mech.to_string()),
                        damage_target: m.id.clone(),
                    })
                    .collect::<Vec<_>>(),
            );
        }
        Ok(events)
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

    fn gen_root_state(mechs: Vec<(&str, Point)>) -> MatchState {
        let mut state = MatchState::default();

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
    fn test_basic_spawn() {
        let state = gen_root_state(vec![
            ("bob", Point::new(10, 10)),
            ("alfred", Point::new(20, 20)),
        ]);

        assert_eq!(state.mechs.len(), 2);
        assert_eq!(state.mechs["bob"].position, Point::new(10, 10));
        assert_eq!(state.mechs["alfred"].position, Point::new(20, 20));
        assert_eq!(state.mechs["alfred"].alive, true);
    }

    #[test]
    fn test_off_board_collision() {
        let state = gen_root_state(vec![("jeeves", Point::new(0, 0))]);
        let cmd = MechCommand::Move {
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
    fn test_safe_move() {
        let state = gen_root_state(vec![("jeeves", Point::new(5, 5))]);
        let cmd = MechCommand::Move {
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
    fn test_primary_fire() {
        let state = gen_root_state(vec![
            ("shooter", Point::new(10, 6)),
            ("victim", Point::new(12, 8)),
        ]);

        let cmd = MechCommand::FirePrimary {
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
    fn test_secondary_fire() {
        let state = gen_root_state(vec![
            ("shooter", Point::new(10, 6)),
            ("victim", Point::new(12, 8)),
        ]);

        let cmd = MechCommand::FireSecondary {
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
    fn test_secondary_fire_with_splash() {
        let state = gen_root_state(vec![
            ("shooter", Point::new(10, 6)),
            ("victim", Point::new(11, 7)),
        ]);

        let cmd = MechCommand::FireSecondary {
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
        assert_eq!(
            state.mechs["shooter"].health,
            INITIAL_HEALTH - SECONDARY_SPLASH_DAMAGE
        );
    }
}
