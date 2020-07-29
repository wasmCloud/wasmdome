extern crate wasmdome_mech_sdk as mech;

use mech::*;

mech_handler!(handler);

pub fn handler(mech: impl MechInstruments) -> Vec<MechCommand> {
    let mut cmds = Vec::new();

    let corner = closest_corner(&mech);
    if let Some(direction) = corner {
        cmds.push(mech.move_mech(direction));
    } else {
        cmds.push(mech.fire_primary(fire_direction(&mech)));
        cmds.push(mech.fire_secondary(fire_direction(&mech)));
    };

    cmds
}

fn closest_corner(mech: &impl MechInstruments) -> Option<GridDirection> {
    let width = mech.world_size().width as i32;
    let height = mech.world_size().height as i32;
    let position = mech.position();

    let horizontal = if position.x == 0 || position.x == width {
        None
    } else if position.x < width - position.x {
        Some(GridDirection::West)
    } else {
        Some(GridDirection::East)
    };

    let vertical = if position.y == 0 || position.y == height {
        None
    } else if position.y < height - position.y {
        Some(GridDirection::North)
    } else {
        Some(GridDirection::South)
    };

    match (vertical, horizontal) {
        (Some(GridDirection::North), Some(GridDirection::West)) => Some(GridDirection::NorthWest),
        (Some(GridDirection::North), Some(GridDirection::East)) => Some(GridDirection::NorthEast),
        (Some(GridDirection::South), Some(GridDirection::East)) => Some(GridDirection::SouthEast),
        (Some(GridDirection::South), Some(GridDirection::West)) => Some(GridDirection::SouthWest),
        (Some(GridDirection::North), None) => Some(GridDirection::North),
        (Some(GridDirection::South), None) => Some(GridDirection::South),
        (None, Some(GridDirection::West)) => Some(GridDirection::West),
        (None, Some(GridDirection::East)) => Some(GridDirection::East),
        (None, None) => None,
        (_, _) => None,
    }
}

fn fire_direction(mech: &impl MechInstruments) -> GridDirection {
    let position = mech.position();
    let world = mech.world_size();
    let _height = world.height as i32;
    let _width = world.width as i32;
    let direction = match (position.x, position.y) {
        (0, 0) => mech.random_number(2, 4),                // NW Corner
        (0, _height) => mech.random_number(0, 2),          // SW Corner
        (_width, 0) => mech.random_number(4, 6),           // NE Corner
        (_width, _height) => mech.random_number(6, 8) % 8, // SE Corner
    };
    match direction {
        0 => GridDirection::North,
        1 => GridDirection::NorthEast,
        2 => GridDirection::East,
        3 => GridDirection::SouthEast,
        4 => GridDirection::South,
        5 => GridDirection::SouthWest,
        6 => GridDirection::West,
        7 => GridDirection::NorthWest,
        _ => GridDirection::North,
    }
}
