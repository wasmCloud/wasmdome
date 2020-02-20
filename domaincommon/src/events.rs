use crate::{DamageSource, Point, DOMAIN_VERSION};

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
    MechWon {
        mech: String,
    },
    MechDestroyed {
        destroyed_mech: String,
    },
    MechSpawned {
        mech: String,
        position: Point,
    },
}
