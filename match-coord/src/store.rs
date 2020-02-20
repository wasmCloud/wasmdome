use domaincommon as domain;
use wascc_actor::prelude::*;

pub(crate) fn load_state(
    ctx: &CapabilitiesContext,
    match_id: &str,
) -> ::std::result::Result<domain::state::MatchState, Box<dyn ::std::error::Error>> {
    let raw = ctx.kv().get(&format!("match:{}", match_id))?;
    let state: domain::state::MatchState = serde_json::from_str(&raw.unwrap())?;
    Ok(state)
}

pub(crate) fn set_state(
    ctx: &CapabilitiesContext,
    match_id: &str,
    state: domain::state::MatchState,
) -> ::std::result::Result<(), Box<dyn ::std::error::Error>> {
    ctx.kv().set(
        &format!("match:{}", match_id),
        &serde_json::to_string(&state)?,
        None,
    )?;
    Ok(())
}
