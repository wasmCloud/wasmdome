use domain::state::MatchState;
use std::collections::{HashMap, HashSet};

const REDIS_URL_KEY: &str = "WASMDOME_ENGINE_REDIS_URL";

extern crate redis;
use redis::Commands;

pub(crate) struct MatchStore {
    matches: Option<HashMap<String, MatchState>>,
    bound_actors: HashSet<String>,
    client: Option<redis::Client>,
}

impl Default for MatchStore {
    fn default() -> MatchStore {
        MatchStore {
            matches: Some(HashMap::new()),
            client: None,
            bound_actors: HashSet::new(),
        }
    }
}

impl MatchStore {
    pub fn new() -> MatchStore {
        if let Ok(url) = std::env::var(REDIS_URL_KEY) {
            info!("Wasmdome Engine Provider Using Redis: {}", url);
            MatchStore::from_redis_url(&url)
        } else {
            info!("Wasmdome Engine Provider Using In-Memory Data");
            MatchStore::default()
        }
    }

    fn from_redis_url(url: &str) -> MatchStore {
        MatchStore {
            matches: None,
            client: Some(redis::Client::open(url).unwrap()),
            bound_actors: HashSet::new(),
        }
    }

    pub fn add_bound_actor(&mut self, actor: &str) -> Result<(), Box<dyn ::std::error::Error>> {
        if self.client.is_none() {
            let _ = self.bound_actors.insert(actor.to_string());
            Ok(())
        } else {
            let k = actors_key();
            self.client
                .as_mut()
                .unwrap()
                .sadd(k, actor)
                .map(|_: ()| ())
                .map_err(|e| e.into())
        }
    }

    pub fn bound_actors(&mut self) -> Result<Vec<String>, Box<dyn ::std::error::Error>> {
        if self.client.is_none() {
            let ba: Vec<String> = self.bound_actors.clone().into_iter().collect();
            Ok(ba)
        } else {
            let k = actors_key();
            self.client
                .as_mut()
                .unwrap()
                .smembers(k)
                .map(|ba: Vec<String>| ba.clone())
                .map_err(|e| e.into())
        }
    }

    pub fn remove_bound_actor(&mut self, actor: &str) -> Result<(), Box<dyn ::std::error::Error>> {
        if self.client.is_none() {
            let _ = self.bound_actors.remove(actor);
            Ok(())
        } else {
            let k = actors_key();
            self.client
                .as_mut()
                .unwrap()
                .srem(k, actor.to_string())
                .map(|_: ()| ())
                .map_err(|e| e.into())
        }
    }

    pub fn save_match_state(
        &mut self,
        match_id: &str,
        state: domain::state::MatchState,
    ) -> ::std::result::Result<(), Box<dyn ::std::error::Error>> {
        if self.matches.is_some() {
            let _existed = self
                .matches
                .as_mut()
                .unwrap()
                .insert(match_id.to_string(), state);
            Ok(())
        } else {
            self.client
                .as_ref()
                .unwrap()
                .get_connection()?
                .set(&match_key(match_id), serde_json::to_string(&state)?)
                .map(|_: ()| ())
                .map_err(|e| e.into())
        }
    }

    pub fn get_match_state(
        &self,
        match_id: &str,
    ) -> ::std::result::Result<domain::state::MatchState, Box<dyn ::std::error::Error>> {
        if self.matches.is_some() {
            self.matches
                .as_ref()
                .unwrap()
                .get(match_id)
                .cloned()
                .ok_or("No such match".into())
        } else {
            let s: String = self
                .client
                .as_ref()
                .unwrap()
                .get_connection()?
                .get(&match_key(match_id))?;
            Ok(serde_json::from_str(&s)?)
        }
    }
}

fn match_key(match_id: &str) -> String {
    format!("wasmdome:matches:{}:state", match_id)
}

fn actors_key() -> String {
    "wasmdome:actors".to_string()
}