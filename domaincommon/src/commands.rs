use crate::{GridDirection, Point, RegisterOperation};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MechCommand {
    Move {
        turn: u32,
        mech: String,
        direction: GridDirection,
    },
    FirePrimary {
        turn: u32,
        mech: String,
        direction: GridDirection,
    },
    FireSecondary {
        turn: u32,
        mech: String,
        direction: GridDirection,
    },
    RequestRadarScan {
        turn: u32,
        mech: String,
    },
    SpawnMech {
        mech: String,
        position: Point,
        team: String,
        avatar: String,
        name: String,
    },
    /// Marks a turn as complete. One of these must be at the end of every
    /// array that comes out of an actor's turn (the developer will not need
    /// to manually append this, the SDK will)
    FinishTurn {
        mech: String,
        turn: u32,
    },
    /// Mech Register Commands
    RegisterUpdate {
        mech: String,
        turn: u32,
        reg: u32,
        op: RegisterOperation,
    },
}

impl MechCommand {
    pub fn action_points(&self) -> u32 {
        match self {
            MechCommand::Move { .. } => 1,
            MechCommand::FirePrimary { .. } => 2,
            MechCommand::FireSecondary { .. } => 4,
            MechCommand::RequestRadarScan { .. } => 1,
            MechCommand::SpawnMech { .. } => 0,
            MechCommand::FinishTurn { .. } => 0,
            MechCommand::RegisterUpdate { .. } => 0,
        }
    }
}
