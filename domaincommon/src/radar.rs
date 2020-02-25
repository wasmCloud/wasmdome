use crate::{
    state::{MatchState, MechState},
    GridDirection, Point,
};

const RADAR_GRID: [(GridDirection, i32); 8] = [
    (GridDirection::NorthWest, 3),
    (GridDirection::North, 4),
    (GridDirection::NorthEast, 3),
    (GridDirection::West, 4),
    (GridDirection::East, 4),
    (GridDirection::SouthWest, 3),
    (GridDirection::South, 4),
    (GridDirection::SouthEast, 3),
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RadarPing {
    pub name: String,
    pub avatar: String,
    pub foe: bool,
    pub location: Point,
    pub distance: usize,
}

pub(crate) fn collect_targets(state: &MatchState, origin: &Point) -> Vec<(MechState, usize)> {
    RADAR_GRID
        .iter()
        .flat_map(|(dir, length)| origin.gather_points(&state.game_board, dir, *length as usize))
        .filter_map(|(p, d)| MatchState::mech_at(state, &p).map(|m| (m, d)))
        .collect::<Vec<_>>()
}

pub(crate) fn radar_ping(
    state: &MatchState,
    origin: &Point,
    scanning_team: &str,
) -> Vec<RadarPing> {
    collect_targets(state, origin)
        .into_iter()
        .map(|(t, d)| mech_to_ping(t, scanning_team, d))
        .collect()
}

pub(crate) fn mech_to_ping(mech: MechState, scanning_team: &str, distance: usize) -> RadarPing {
    RadarPing {
        name: mech.name,
        avatar: mech.avatar,
        location: mech.position,
        distance,
        foe: mech.team.to_lowercase() != scanning_team.to_lowercase(),
    }
}
