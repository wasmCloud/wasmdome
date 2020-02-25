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
//! pub fn handler(mech: impl MechInstruments) -> Vec<MechCommand> {
//!     vec![]
//! }
//!
//! ```

pub extern crate wascc_actor;
pub extern crate wasmdome_protocol as protocol;

use domaincommon::GridDirection;
pub use domaincommon::commands::MechCommand;
use domaincommon::state::MechState;
use domaincommon::Point;

#[macro_export]
macro_rules! mech_handler {
    ($user_handler:ident) => {
        use protocol::commands::TakeTurn;
        use protocol::events::MatchEvent;
        use $crate::wascc_actor::prelude::*;

        actor_receive!(handle_wascc);
        fn handle_wascc(ctx: &CapabilitiesContext, operation: &str, msg: &[u8]) -> CallResult {
            match operation {
                messaging::OP_DELIVER_MESSAGE => handle_message(ctx, msg),
                core::OP_HEALTH_REQUEST => Ok(vec![]),
                _ => Err(format!("Mech Handler: Unrecognized operation: {}", operation).into()),
            }
        }

        fn handle_message(
            ctx: &CapabilitiesContext,
            msg: impl Into<messaging::DeliverMessage>,
        ) -> CallResult {
            let take_turn: TakeTurn = serde_json::from_slice(&msg.into().message.unwrap().body)?;
            let mech =
                $crate::WasmdomeMechInstruments::new(take_turn.clone(), take_turn.actor.clone());
            let mut vec = $user_handler(mech);
            vec.push(MechCommand::FinishTurn{
                mech: take_turn.actor.clone(),
                turn: take_turn.turn,
            });
            let request = MatchEvent::TurnRequested {
                actor: take_turn.actor,
                match_id: take_turn.match_id.clone(),
                turn: take_turn.turn,
                commands: vec,
            };
            ctx.msg().publish(
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
}

impl MechInstruments for WasmdomeMechInstruments {
    fn position(&self) -> Point {
        self.current_mech().position.clone()
    }
    fn hull_integrity(&self) -> u32 {
        self.current_mech().health
    }

    fn power(&self) -> u32 {
        domaincommon::state::APS_PER_TURN
    }
    
    fn direction_to(&self, target: &Point) -> GridDirection {        
        self.current_mech().position.bearing(target)
    }

    fn primary_range(&self) -> u32 {
        domaincommon::state::PRIMARY_RANGE as u32
    }

    fn secondary_range(&self) -> u32 {
        domaincommon::state::SECONDARY_RANGE as u32
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
}
