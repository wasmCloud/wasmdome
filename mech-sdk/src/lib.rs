//! # mech-sdk
//!
//! TBD
//!
//! # Example
//! ```
//! extern crate wasmdome_mech_sdk as mech;
//!
//! use mech::*;
//!
//! mech_handler!(handler);
//!
//! // Respond to a request to take a turn
//! pub fn handler(mech: impl MechInstruments) -> Vec<MechCommand> {
//!     // Respond with up to 4 action points worth of actions
//!     vec![
//!         mech.request_radar(),
//!         mech.move_mech(GridDirection::North),
//!         mech.fire_primary(GridDirection::South),
//!     ]
//! }
//!
//! ```

pub extern crate wascc_actor;
pub extern crate wasmdome_protocol as protocol;

use wasmdome_domain as domain;

pub use domain::commands::MechCommand;
use domain::state::MechState;
pub use domain::GameBoard;
pub use domain::GridDirection;
pub use domain::Point;

use wascc_actor::prelude::*;

#[macro_export]
macro_rules! mech_handler {
    ($user_handler:ident) => {
        use protocol::commands::TakeTurn;
        use protocol::events::MatchEvent;
        use $crate::wascc_actor::prelude::*;

        actor_handlers!{ codec::messaging::OP_DELIVER_MESSAGE => handle_message, codec::core::OP_HEALTH_REQUEST => health }

        fn health(_req: codec::core::HealthRequest) -> ReceiveResult {
            Ok(vec![])
        }

        fn handle_message(
            msg: codec::messaging::BrokerMessage,
        ) -> CallResult {
            let take_turn: TakeTurn = serde_json::from_slice(&msg.body)?;
            let mech =
                $crate::WasmdomeMechInstruments::new(take_turn.clone(), take_turn.actor.clone());
            let mut vec = if mech.is_alive() {
                $user_handler(mech)
            } else {
                Vec::new()
            };
            vec.push(MechCommand::FinishTurn {
                mech: take_turn.actor.clone(),
                turn: take_turn.turn,
            });
            let request = MatchEvent::TurnRequested {
                actor: take_turn.actor,
                match_id: take_turn.match_id.clone(),
                turn: take_turn.turn,
                commands: vec,
            };
            messaging::default().publish(
                &format!(
                    $crate::protocol::match_events_subject!(),
                    take_turn.match_id
                ),
                None,
                &serde_json::to_vec(&request)?,
            )?;
            Ok(vec![])
        }
    };
}

pub trait MechInstruments {
    fn position(&self) -> Point;
    fn hull_integrity(&self) -> u32;
    fn power(&self) -> u32;
    fn primary_range(&self) -> u32;
    fn secondary_range(&self) -> u32;
    fn last_radar_scan(&self) -> Option<Vec<RadarPing>>;
    fn direction_to(&self, target: &Point) -> GridDirection;
    fn random_number(&self, min: u32, max: u32) -> u32;
    fn world_size(&self) -> GameBoard;

    //- Generate commands

    fn request_radar(&self) -> MechCommand;
    fn fire_primary(&self, dir: GridDirection) -> MechCommand;
    fn fire_secondary(&self, dir: GridDirection) -> MechCommand;
    fn move_mech(&self, dir: GridDirection) -> MechCommand;
}

pub struct RadarPing {
    pub id: String,
    pub foe: bool,
    pub location: Point,
    pub distance: usize,
}

pub struct WasmdomeMechInstruments {
    actor: String,
    turn: protocol::commands::TakeTurn,
}

impl WasmdomeMechInstruments {
    pub fn new(turn: protocol::commands::TakeTurn, actor: String) -> Self {
        WasmdomeMechInstruments { turn, actor }
    }
}

impl WasmdomeMechInstruments {
    fn current_mech(&self) -> &MechState {
        &self.turn.state.mechs[&self.actor]
    }

    #[allow(dead_code)]
    pub fn is_alive(&self) -> bool {
        self.current_mech().alive
    }
}

impl MechInstruments for WasmdomeMechInstruments {
    fn position(&self) -> Point {
        self.current_mech().position.clone()
    }
    fn hull_integrity(&self) -> u32 {
        self.current_mech().health
    }

    fn power(&self) -> u32 {
        domain::state::APS_PER_TURN
    }

    fn direction_to(&self, target: &Point) -> GridDirection {
        self.current_mech().position.bearing(target)
    }

    fn primary_range(&self) -> u32 {
        domain::state::PRIMARY_RANGE as u32
    }

    fn secondary_range(&self) -> u32 {
        domain::state::SECONDARY_RANGE as u32
    }

    fn request_radar(&self) -> MechCommand {
        MechCommand::RequestRadarScan {
            turn: self.turn.turn,
            mech: self.actor.to_string(),
        }
    }

    fn fire_primary(&self, dir: GridDirection) -> MechCommand {
        MechCommand::FirePrimary {
            turn: self.turn.turn,
            mech: self.actor.to_string(),
            direction: dir,
        }
    }

    fn fire_secondary(&self, dir: GridDirection) -> MechCommand {
        MechCommand::FireSecondary {
            turn: self.turn.turn,
            mech: self.actor.to_string(),
            direction: dir,
        }
    }

    fn move_mech(&self, dir: GridDirection) -> MechCommand {
        MechCommand::Move {
            turn: self.turn.turn,
            mech: self.actor.to_string(),
            direction: dir,
        }
    }

    fn last_radar_scan(&self) -> Option<Vec<RadarPing>> {
        self.turn.state.radar_pings.get(&self.actor).map(|pings| {
            pings
                .iter()
                .map(|p| RadarPing {
                    id: p.name.to_string(),
                    distance: p.distance,
                    foe: p.foe,
                    location: p.location.clone(),
                })
                .collect()
        })
    }

    fn random_number(&self, min: u32, max: u32) -> u32 {
        match extras::default().get_random(min, max) {
            Ok(r) => r,
            Err(_) => 0,
        }
    }

    fn world_size(&self) -> GameBoard {
        GameBoard {
            height: self.turn.state.parameters.height,
            width: self.turn.state.parameters.width,
        }
    }
}
