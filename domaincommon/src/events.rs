use crate::{DamageSource, Point, DOMAIN_VERSION};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum EndCause {
    MaxTurnsCompleted,
    MechVictory(String),
}

#[derive(Serialize, Deserialize, Debug, Clone, Event)]
#[event_type_version(DOMAIN_VERSION)]
#[event_source("events://wasmdome.dev/game")]
pub enum GameEvent {
    PositionUpdated {
        mech: String,
        position: Point,
    },
    DamageTaken {
        damage_target: String,
        damage: u32,
        damage_source: DamageSource,
    },
    MechDestroyed {
        damage_target: String,
        damage_source: DamageSource,
    },
    MechSpawned {
        mech: String,
        position: Point,
    },
    MechTurnCompleted {
        mech: String,
        turn: u32,
    },
    MatchTurnCompleted {
        new_turn: u32,
    },
    GameFinished {
        cause: EndCause,
    },
}
