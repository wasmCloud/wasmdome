//! # Assembly Mechs: Beyond WasmDome SDK
//!
//! _The year is 2020 and our containerized civilization is falling apart.
//! A cruel and villainous DevOps demon named **Boylur Plait** has descended from the cloud to Earth to challenge mankind to a tournament_.
//!
//! To win this tournament, _Assembly Mechs_ must compete in an absurdly over-dramatized contest.
//! These mechs will challenge their creator's ability to write code that will outlast and defeat everything that
//! the demon and its nearly infinite hordes pit against us. Humanity's only hope is to master a technology called **WebAssembly**,
//! win the tournament, and prove to the cloud nemesis that this world is well protected.
//!
//! ## How to Play
//! The game is played by your mech declaring a `handler` function. During each turn, the mech's handler will be invoked
//! and it will be responsible for returning a list of commands. These commands can include requests to move, fire a weapon,
//! perform a radar scan, etc. Commands cost _action points_ and you need to take care that you do not exceed the maximum number
//! of action points per turn (currently **4**).
//!
//! Your mech will have to make clever use of the limited resources and information available to it to devise a strategy for
//! winning the match.
//!
//! The mech interacts with its environment exclusively through the use of the [MechInstruments](trait.MechInstruments.html) trait.
//!
//! ## Possible Mech Actions
//!
//! The following is a list of actions a mech can take by using the appropriate methods in the _Assembly Mech SDK_:
//!
//!
//!| Action | AP Cost | Description |
//!| -------- | -------- | -------- |
//!| [move_mech](trait.MechInstruments.html#tymethod.move_mech)     | 1     | Moves the mech one grid unit in a given direction.|
//!| [fire_primary](trait.MechInstruments.html#tymethod.fire_primary)| 2 | Fires the mech's primary weapon in a given direction. Primary weapons fire a single small projectile that will damage the first thing it encounters. Primary weapon range is available via sensor interrogation. |
//!| [fire_secondary](trait.MechInstruments.html#tymethod.fire_secondary)| 4 | Fires the mech's secondary weapon in a given direction. Secondary weapons fire an explosive projectile that damages the first thing it encounters, as well as producing splash damage that radiates out from the point of impact. Secondary weapon range is available via sensor interrogation.  |
//!| [radar_scan](trait.MechInstruments.html#tymethod.radar_scan) | 1 | Performs a full radar scan of the mech's surroundings, reporting on detected enemies and obstacles. The mech will receive the results of the scan at the beginning of the next turn.|
//!
//!The default, unaffected power of a mech is **4** units, meaning that within a single turn a mech may fire its secondary weapon once,
//! move 4 times, or perform some other combination of actions. Accessing sensor values does not cost you anything.
//!
//! ## Warnings
//! Take care not to exceed the maximum number of action points consumed in a given turn. At best, commands exceeding your [power](trait.MechInstruments.html#tymethod.power)
//! will fail, at worst (depending on the match rules) your mech might be penalized for the attempt
//!
//! Collision damage is real, and your mech's hull will lose structural integrity when colliding with other mechs and with walls
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

use domain::state::MechState;
pub use domain::{
    commands::MechCommand, GameBoard, GridDirection, Point, RegisterOperation, RegisterValue, EAX,
    EBX, ECX,
};

use wascc_actor::prelude::*;

/// Declares the function to be used as the mech's turn handler
#[macro_export]
macro_rules! mech_handler {
    ($user_handler:ident) => {
        use protocol::commands::{TakeTurn, TakeTurnResponse};
        use protocol::events::MatchEvent;
        use protocol::OP_TAKE_TURN;
        use $crate::wascc_actor::prelude::*;

        actor_handlers!{ OP_TAKE_TURN => handle_take_turn, codec::core::OP_HEALTH_REQUEST => health }

        fn health(_req: codec::core::HealthRequest) -> HandlerResult<()> {
            Ok(())
        }

        fn handle_take_turn(
            take_turn: protocol::commands::TakeTurn,
        ) -> HandlerResult<TakeTurnResponse> {
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
            let response = TakeTurnResponse {
                commands: vec,
            };
            Ok(response)
        }
    };
}

/// The interface through which a mech interacts with the arena. Functions on the mech instruments panel are divided into two categories:
/// * Sensor access - These functions are synchronous and the results are accessed immediately within your code. There is no power/action point cost to using these functions.
/// * Commands - The mech's `handler` function must return a vector of commands. The instruments panel has shortcut functions that can be used to generate these commands. Each command has an action point cost associated with it, so take care not to exceed your turn budget
///
/// By cleverly and carefully combining the mech sensor functions with the commands it can issue to the arena, your goal is to build
/// a mech that can outsmart, outgun, and outmaneuver its opponents in the [wasmdome](https://wasmdome.dev).
pub trait MechInstruments {
    /// Obtains the current position of the mech
    fn position(&self) -> Point;
    /// Queries the hull integrity of the mech in remaining hit / damage points
    fn hull_integrity(&self) -> u32;
    /// Returns the number of action points this mech can consume per turn. While this may default to **4**, your code should use this value as a maximum in case matches started with different rules include more or less maximum APs
    fn power(&self) -> u32;
    /// Returns the range (in whole grid units) of the primary weapon. This defaults to **3** but your code should use this if you need to perform calculations based on range
    fn primary_range(&self) -> u32;
    /// Returns the range (in whole grid units) of the secondary weapon, which defaults to **6** but your code should use this if you need to perform calculations based on range
    fn secondary_range(&self) -> u32;
    /// Accesses the last radar scan (if any) performed by your mech. If on turn **x** your mech has a radar request in the command list, then on turn **x+1** that scan's results will be available
    fn last_radar_scan(&self) -> Option<Vec<RadarPing>>;
    /// A handy function that performs the Euclidean calculation for you in order to determine the direction between your mech and a target point
    fn direction_to(&self, target: &Point) -> GridDirection;
    /// Generate a random number between the min and max values (inclusive)
    fn random_number(&self, min: u32, max: u32) -> u32;
    /// Obtains the dimensions of the arena in which the mech resides
    fn world_size(&self) -> GameBoard;

    //- Registers

    /// Accumulates the value stored in the given register. If the register has not been initialized, the accumulator value supplied will be stored in the register (e.g. the value will be added to **0**)
    fn register_acc(&self, reg: u32, val: u64) -> MechCommand;
    /// Safely decrements (with a floor of **0**) the value in the given register
    fn register_dec(&self, reg: u32, val: u64) -> MechCommand;
    /// Sets the value in the given register. This will overwrite any previously existing value
    fn register_set(&self, reg: u32, val: RegisterValue) -> MechCommand;
    /// Queries the value (if any) stored in the given register
    fn register_get(&self, reg: u32) -> Option<&RegisterValue>;

    //- Generate commands

    /// Generates a radar request command to be processed by the game engine at the end of this turn
    fn request_radar(&self) -> MechCommand;
    /// Generates a request to fire the primary weapon
    fn fire_primary(&self, dir: GridDirection) -> MechCommand;
    /// Generates a request to fire the secondary weapon
    fn fire_secondary(&self, dir: GridDirection) -> MechCommand;
    /// Generates a request to move the mech
    fn move_mech(&self, dir: GridDirection) -> MechCommand;
}

/// A single result from a radar scan. When a mech queries for the last radar scan and
/// one is available, those results will be a vector of these radar pings
pub struct RadarPing {
    /// A unique identifier for this target. This is an opaque string and you should assign no internal meaning to it other than to distinguish one target from another
    pub id: String,
    /// Indicates if the discovered target is a friend or foe
    pub foe: bool,
    /// The location from which the radar response originated
    pub location: Point,
    /// A rounded, whole number indicating the distance to the discovered target
    pub distance: usize,
}

#[doc(hidden)]
pub struct WasmdomeMechInstruments {
    actor: String,
    turn: protocol::commands::TakeTurn,
}

#[doc(hidden)]
impl WasmdomeMechInstruments {
    pub fn new(turn: protocol::commands::TakeTurn, actor: String) -> Self {
        WasmdomeMechInstruments { turn, actor }
    }
}

#[doc(hidden)]
impl WasmdomeMechInstruments {
    fn current_mech(&self) -> &MechState {
        &self.turn.state.mechs[&self.actor]
    }

    #[allow(dead_code)]
    pub fn is_alive(&self) -> bool {
        self.current_mech().alive
    }
}

#[doc(hidden)]
impl MechInstruments for WasmdomeMechInstruments {
    fn position(&self) -> Point {
        self.current_mech().position.clone()
    }
    fn hull_integrity(&self) -> u32 {
        self.current_mech().health
    }

    fn power(&self) -> u32 {
        self.turn.state.parameters.aps_per_turn
    }

    fn primary_range(&self) -> u32 {
        domain::state::PRIMARY_RANGE as u32
    }

    fn secondary_range(&self) -> u32 {
        domain::state::SECONDARY_RANGE as u32
    }

    fn direction_to(&self, target: &Point) -> GridDirection {
        self.current_mech().position.bearing(target)
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

    //- Registers
    fn register_acc(&self, reg: u32, val: u64) -> MechCommand {
        MechCommand::RegisterUpdate {
            mech: self.current_mech().id.to_string(),
            reg,
            op: RegisterOperation::Accumulate(val),
            turn: self.turn.turn,
        }
    }

    fn register_dec(&self, reg: u32, val: u64) -> MechCommand {
        MechCommand::RegisterUpdate {
            mech: self.current_mech().id.to_string(),
            reg,
            op: RegisterOperation::Decrement(val),
            turn: self.turn.turn,
        }
    }

    fn register_set(&self, reg: u32, val: RegisterValue) -> MechCommand {
        MechCommand::RegisterUpdate {
            mech: self.current_mech().id.to_string(),
            reg,
            op: RegisterOperation::Set(val),
            turn: self.turn.turn,
        }
    }

    fn register_get(&self, reg: u32) -> Option<&RegisterValue> {
        self.current_mech().registers.get(&reg)
    }

    //- Generate commands

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
}
