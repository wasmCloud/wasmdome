extern crate wasmdome_mech_sdk as mech;

use mech::*;

mech_handler!(handler);

/**
 * Strategy:
 * Move around (randomly? procedurally?)
 * Find enemy mech
 * Run & gun straight towards mech
 */
pub fn handler(mech: impl MechInstruments) -> Vec<MechCommand> {
     if let Some(scan) = mech.last_radar_scan() {
          // Prefer to destroy previous target
          if let Some(RegisterValue::Text(enemy_id)) = mech.register_get(EBX) {
               if let Some(enemy) = scan
                    .iter()
                    .filter(|ping| ping.foe)
                    .filter(|ping| ping.id == *enemy_id)
                    .collect::<Vec<_>>()
                    .get(0)
               {
                    times_up_lets_do_this(&mech, enemy)
               } else {
                    find_raid(&mech)
               }
          } else if let Some(enemy) = scan
               .iter()
               .filter_map(|ping| if ping.foe { Some(ping) } else { None })
               .take(1)
               .collect::<Vec<_>>()
               .get(0)
          {
               times_up_lets_do_this(&mech, enemy)
          } else {
               find_raid(&mech)
          }
     } else {
          find_raid(&mech)
     }
}

/// Determine if potential enemy is in range, if not proceed to move and rescan
fn find_raid(mech: &impl MechInstruments) -> Vec<MechCommand> {
     match mech.last_radar_scan() {
          Some(scan) => {
               if let Some(RegisterValue::Text(enemy_id)) = mech.register_get(EBX) {
                    if let Some(enemy) = scan
                         .iter()
                         .filter(|p| p.id == *enemy_id)
                         .collect::<Vec<_>>()
                         .get(0)
                    {
                         times_up_lets_do_this(mech, enemy)
                    } else {
                         vec![
                              move_mech(mech),
                              move_mech(mech),
                              move_mech(mech),
                              mech.request_radar(),
                         ]
                    }
               } else if let Some(enemy) = scan
                    .iter()
                    .filter(|p| p.foe)
                    .filter(|p| distance_to(&mech.position(), &p.location) <= mech.primary_range())
                    .collect::<Vec<_>>()
                    .get(0)
               {
                    vec![
                         mech.fire_primary(mech.direction_to(&enemy.location)),
                         mech.move_mech(mech.direction_to(&enemy.location)),
                         mech.move_mech(mech.direction_to(&enemy.location)),
                         mech.register_set(EBX, RegisterValue::Text(enemy.id.to_string())),
                    ]
               } else {
                    vec![
                         move_mech(mech),
                         move_mech(mech),
                         move_mech(mech),
                         mech.request_radar(),
                    ]
               }
          }
          None => vec![
               move_mech(mech),
               move_mech(mech),
               move_mech(mech),
               mech.request_radar(),
          ],
     }
}

fn times_up_lets_do_this(mech: &impl MechInstruments, enemy: &RadarPing) -> Vec<MechCommand> {
     let distance = distance_to(&mech.position(), &enemy.location);
     if distance >= 2 && distance <= mech.primary_range() - 1 {
          vec![
               mech.move_mech(mech.direction_to(&enemy.location)),
               mech.move_mech(mech.direction_to(&enemy.location)),
               mech.fire_primary(mech.direction_to(&enemy.location)),
               mech.register_set(EBX, RegisterValue::Text(enemy.id.to_string())),
          ]
     } else if distance <= mech.primary_range() {
          vec![
               mech.fire_primary(mech.direction_to(&enemy.location)),
               mech.fire_primary(mech.direction_to(&enemy.location)),
               mech.register_set(EBX, RegisterValue::Text(enemy.id.to_string())),
          ]
     } else {
          vec![
               mech.move_mech(mech.direction_to(&enemy.location)),
               mech.move_mech(mech.direction_to(&enemy.location)),
               mech.move_mech(mech.direction_to(&enemy.location)),
               mech.request_radar(),
          ]
     }
}

fn distance_to(a: &Point, b: &Point) -> u32 {
     ((b.x - a.x).pow(2) as f64 + (b.y - a.y).pow(2) as f64).sqrt() as u32
}

fn move_mech(mech: &impl MechInstruments) -> MechCommand {
     mech.move_mech(move_direction(mech.position(), mech.world_size()))
}

//TODO: Validate this moves jenkins with the correct behavior
fn move_direction(pos: Point, gb: GameBoard) -> GridDirection {
     // Run in a quadrant square pattern
     if pos.y >= gb.height as i32 / 4 {
          GridDirection::North
     } else if pos.x >= gb.width as i32 / 4 {
          GridDirection::West
     } else if pos.y < gb.height as i32 / 4 {
          GridDirection::South
     } else {
          GridDirection::East
     }
}
