#[macro_use]
extern crate serde_derive;

pub mod events {
    pub const SUBJECT_EVENTS_MASK: &str = "wasmdome.match_events.{}";
    pub const SUBJECT_MATCH_EVENTS_PREFIX: &str = "wasmdome.match_events.";

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub enum MatchEvent {
        ActorScheduled { actor: String, match_id: String },
        MatchStarted { match_id: String },
        MatchCreated { match_id: String, actors: Vec<String>, board_height: u32, board_width: u32 }
    }

}

pub mod commands {
    pub const SUBJECT_CREATE_MATCH: &str = "wasmdome.matches.create";
    pub const SUBJECT_MATCH_COMMANDS_PREFIX: &str = "wasmdome.matches";
    pub const SUBJECT_SCHEDULE_ACTOR: &str = "scheduleactor";

    /// Requests that a given actor be scheduled for a given match (auction style)
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ScheduleActor {
        pub actor: String,
        pub match_id: String,
    }

    /// Response to a messaging request to schedule an actor
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ScheduleActorAck {
        pub actor: String,
        pub match_id: String,
    }

    /// Signals the desire to create a new match
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct CreateMatch {        
        pub match_id: String, 
        pub actors: Vec<String>,        
        pub board_height: u32,
        pub board_width: u32,
    }

    /// Response to a request to start a match. Indicates that the
    /// match is starting
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct StartMatchAck {
        pub match_id: String,
    }
}