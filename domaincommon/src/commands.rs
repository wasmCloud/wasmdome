use crate::{GridDirection, Point};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MechCommand {
    Move {
        mech: String,
        direction: GridDirection,
    },
    FirePrimary {
        mech: String,
        direction: GridDirection,
    },
    FireSecondary {
        mech: String,
        direction: GridDirection,
    },
    RequestRadarScan {
        mech: String,
    },
    SpawnMech {
        mech: String,
        position: Point,
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
        }
    }
}
