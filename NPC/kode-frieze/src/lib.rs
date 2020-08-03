extern crate wasmdome_mech_sdk as mech;

use mech::*;

mech_handler!(handler);

pub fn handler(mech: impl MechInstruments) -> Vec<MechCommand> {
     if !is_in_center(&mech) {
          let mut cmds = move_to_center(&mech);
          cmds.push(mech.request_radar());
          cmds
     } else if let Some(scan) = mech.last_radar_scan() {
          seek_and_destroy_target(&mech, scan)
     } else {
          vec![mech.request_radar(), mech.register_acc(ECX, 1)]
     }
}

/// We accept being 1 square away to cover the even board width and height edge case
fn is_in_center(mech: &impl MechInstruments) -> bool {
     if distance_to(&mech.position(), &compute_center(mech.world_size())) <= 1 {
          true
     } else {
          false
     }
}

// Returns None if target already exists in the register
// Returns a mech command to set the target otherwise
fn seek_and_destroy_target(mech: &impl MechInstruments, scan: Vec<RadarPing>) -> Vec<MechCommand> {
     let scan = scan
          .iter()
          .filter(|s| distance_to(&mech.position(), &s.location) <= mech.primary_range())
          .filter(|s| s.foe)
          .collect::<Vec<_>>();

     match mech.register_get(EBX) {
          Some(RegisterValue::Text(target)) => {
               if let Some(target) = scan
                    .iter()
                    .filter(|s| s.id == *target)
                    .collect::<Vec<_>>()
                    .get(0)
               {
                    vec![
                         mech.fire_primary(mech.direction_to(&target.location)),
                         mech.fire_primary(mech.direction_to(&target.location)),
                    ]
               } else if let Some(target) = scan.get(0) {
                    vec![
                         mech.register_set(EBX, RegisterValue::Text(target.id.clone())),
                         mech.fire_primary(mech.direction_to(&target.location)),
                         mech.fire_primary(mech.direction_to(&target.location)),
                    ]
               } else {
                    vec![mech.request_radar(), mech.register_acc(ECX, 1)]
               }
          }
          _ => {
               if let Some(target) = scan.get(0) {
                    vec![
                         mech.register_set(EBX, RegisterValue::Text(target.id.clone())),
                         mech.fire_primary(mech.direction_to(&target.location)),
                         mech.fire_primary(mech.direction_to(&target.location)),
                    ]
               } else {
                    vec![mech.request_radar(), mech.register_acc(ECX, 1)]
               }
          }
     }
}

/// Up to 3 moves to allow for an extra action
fn move_to_center(mech: &impl MechInstruments) -> Vec<MechCommand> {
     let mut cmds = vec![];
     let center = compute_center(mech.world_size());

     for _ in 0..distance_to(&mech.position(), &center) {
          if cmds.len() < 3 {
               cmds.push(mech.move_mech(mech.direction_to(&center)));
          } else {
               break;
          }
     }

     cmds
}

fn distance_to(a: &Point, b: &Point) -> u32 {
     ((b.x - a.x).pow(2) as f64 + (b.y - a.y).pow(2) as f64).sqrt() as u32
}

fn compute_center(gameboard: GameBoard) -> Point {
     Point {
          x: (gameboard.width / 2) as i32,
          y: (gameboard.height / 2) as i32,
     }
}
