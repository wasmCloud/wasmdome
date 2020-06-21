#[macro_use]
extern crate serde_derive;

pub const OP_TAKE_TURN: &str = "wdTakeTurn";

pub mod events {
    use chrono::prelude::*;
    use domain::events::EndCause;
    use wasmdome_domain as domain;

    pub fn events_subject(match_id: Option<&str>) -> String {
        if let Some(match_id) = match_id {
            format!("wasmdome.match.{}.events", match_id)
        } else {
            "wasmdome.arena.events".to_string()
        }
    }

    pub fn arena_control_subject() -> String {
        "wasmdome.arena.control".to_string()
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub enum ArenaEvent {
        MechConnected {
            actor: String,
            time: DateTime<Utc>,
        },
        MechDisconnected {
            actor: String,
            time: DateTime<Utc>,
        },
        MatchStarted {
            match_id: String,
            actors: Vec<String>,
            board_height: u32,
            board_width: u32,
            start_time: DateTime<Utc>,
        },
        MatchCompleted {
            match_id: String,
            cause: EndCause,
            time: DateTime<Utc>,
        },
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub enum MatchEvent {
        /// Emitted by the core engine so that downstream listeners (e.g. historian, leaderboard) can process
        TurnEvent {
            actor: String,
            match_id: String,
            turn: u32,
            turn_event: domain::events::GameEvent,
        },
    }
}

pub mod commands {
    use wasmdome_domain as domain;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum ArenaControlCommand {
        StartMatch(CreateMatch),
    }

    /// Sent on a match subject to tell a given mech to take its turn. The response
    /// to this should be an acknowledgement containing the list of commands performed
    /// by that mech.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct TakeTurn {
        pub actor: String,
        pub match_id: String,
        pub turn: u32,
        pub state: domain::state::MatchState,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct TakeTurnResponse {
        pub commands: Vec<domain::commands::MechCommand>,
    }

    /// Signals the desire to create a new match
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct CreateMatch {
        pub match_id: String,
        pub actors: Vec<String>,
        pub board_height: u32,
        pub board_width: u32,
        pub max_turns: u32,
        pub aps_per_turn: u32,
    }
}
