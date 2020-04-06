use wasmdome_domain as domain;

use wascc_actor::keyvalue::KeyValueStoreHostBinding;

pub(crate) fn load_state(
    kv: &KeyValueStoreHostBinding,
    match_id: &str,
) -> ::std::result::Result<domain::state::MatchState, Box<dyn ::std::error::Error>> {
    let raw = kv.get(&format!("match:{}", match_id))?;
    let state: domain::state::MatchState = serde_json::from_str(&raw.unwrap())?;
    Ok(state)
}

pub(crate) fn set_state(
    kv: &KeyValueStoreHostBinding,
    match_id: &str,
    state: domain::state::MatchState,
) -> ::std::result::Result<(), Box<dyn ::std::error::Error>> {
    kv.set(
        &format!("match:{}", match_id),
        &serde_json::to_string(&state)?,
        None,
    )?;
    Ok(())
}
