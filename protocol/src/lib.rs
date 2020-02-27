#[macro_use]
extern crate serde_derive;

pub mod events {
    use wasmdome_domain as domain;

    // 💩💩💩 This is an annoying hack to get around the restriction that you can't use
    // 💩💩💩 format! with a string constant, so instead we use a macro to generate
    // 💩💩💩 a string literal.
    #[macro_export]
    macro_rules! match_events_subject {
        () => {
            "wasmdome.match_events.{}"
        };
    }

    pub const SUBJECT_MATCH_EVENTS_PREFIX: &str = "wasmdome.match_events.";

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub enum MatchEvent {
        MatchCreated {
            match_id: String,
            actors: Vec<String>,
            board_height: u32,
            board_width: u32,
        },
        ActorStarted {
            actor: String,
            match_id: String,
            name: String,
            avatar: String,
            team: String,
        },
        MatchStarted {
            match_id: String,
        },
        /// Published in response to a TakeTurn command. The command processor will be listening for this event
        TurnRequested {
            actor: String,
            match_id: String,
            turn: u32,
            commands: Vec<domain::commands::MechCommand>,
        },
        /// Emitted by the command processor so that downstream listeners (e.g. historian, leaderboard) can process
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

    #[macro_export]
    macro_rules! turns_subject {
        () => {
            "wasmdome.matches.{}.turns.{}"
        };
    }

    pub const SUBJECT_CREATE_MATCH: &str = "wasmdome.matches.create";
    pub const SUBJECT_MATCH_COMMANDS_PREFIX: &str = "wasmdome.matches";
    pub const SUBJECT_SCHEDULE_ACTOR: &str = "scheduleactor";

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
        pub max_turns: u32,
    }

    /// Response to a request to start a match. Indicates that the
    /// match is starting
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct StartMatchAck {
        pub match_id: String,
    }
}
