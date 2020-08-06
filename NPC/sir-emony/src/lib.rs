extern crate wasmdome_mech_sdk as mech;

use mech::*;

mech_handler!(handler);

/*
 * Strategy:
 * find friendly mech store it in EBX register to keep track (locate_friendly) otherwise move around to find it
 * if distance >= 4, move twice towards it
 * Otherwise, fire primary at any mech in range (not in the same direction as the friendly)
 */
pub fn handler(mech: impl MechInstruments) -> Vec<MechCommand> {

     if let Some(RegisterValue::Text(friendly_id)) = mech.register_get(EBX) {
          if let Some(scan) = mech.last_radar_scan() {
               if let Some(friendly) = radar_ping_for_id(scan, friendly_id.to_string()) {
                    if friendly.distance > 2 {
                         stay_with_friendly(&mech, &friendly)
                    } else {
                         supporting_fire(&mech, friendly)
                    }
               } else {
                    locate_friendly(&mech)
               }
          } else {
               vec![
                    mech.request_radar()
               ]
          }
     } else {
          locate_friendly(&mech)
     }
}

fn locate_friendly(mech: &impl MechInstruments) -> Vec<MechCommand> {
     if let Some(scan) = mech.last_radar_scan() {
          let friendlies = scan.iter().filter(|p| !p.foe).collect::<Vec<_>>();
          if let Some(friendly) = friendlies.get(0) {
               let mut catch_up = vec![mech.register_set(EBX, RegisterValue::Text(friendly.id.to_string()))];
               catch_up.append(&mut stay_with_friendly(mech, friendly));
               return catch_up
          }
     }
     vec![
          move_mech(mech),
          move_mech(mech),
          move_mech(mech),
          mech.request_radar()
     ]
}

fn stay_with_friendly(mech: &impl MechInstruments, friendly: &RadarPing) -> Vec<MechCommand> {
     let mut moves = vec![];
     let amt = if friendly.distance >= 3 {
          3
     } else {
          friendly.distance - 1 
     };
     for _ in 0..amt {
          moves.push(mech.move_mech(mech.direction_to(&friendly.location)))
     }
     moves.push(mech.request_radar());
     moves
}

fn supporting_fire(mech: &impl MechInstruments, friendly: RadarPing) -> Vec<MechCommand> {
     if let Some(scan) = mech.last_radar_scan() {
          let shots = scan
                      .iter()
                      .filter(|p| p.foe)
                      .filter(|p| mech.direction_to(&p.location) != mech.direction_to(&friendly.location))
                      .filter(|p| p.distance <= mech.secondary_range() as usize)
                      .collect::<Vec<_>>();
          match shots.get(0) {
               Some(target) if target.distance <= mech.primary_range() as usize => {
                    vec![
                         mech.fire_primary(mech.direction_to(&target.location)),
                         mech.fire_primary(mech.direction_to(&target.location))
                    ]
               },
               Some(target) => {
                    vec![
                         mech.fire_secondary(mech.direction_to(&target.location))
                    ]
               },
               None => stay_with_friendly(mech, &friendly)
          }
     } else {
          vec![
               mech.request_radar()
          ]
     }
}

fn radar_ping_for_id(scan: Vec<RadarPing>, id: String) -> Option<RadarPing> {
     if let Some(enemy) = scan.iter().filter(|e| e.id == id).collect::<Vec<_>>().get(0) {
          let ping = RadarPing {
               id: enemy.id.to_string(),
               location: Point::new(enemy.location.x, enemy.location.y),
               foe: enemy.foe,
               distance: enemy.distance
          };
          Some(ping)
     } else {
          None
     }
}

fn move_mech(mech: &impl MechInstruments) -> MechCommand {
     mech.move_mech(move_direction(mech.position(), mech.world_size()))
}

// Sir Emony and Deeploy Jenkinns are great friends. They hope to meet up sometime
fn move_direction(pos: Point, gb: GameBoard) -> GridDirection {
     // Run in a quadrant square pattern
     let quarter_height = gb.height as i32 / 4;
     let quarter_width = gb.width as i32 / 4;

     if pos.x <= quarter_width && pos.y < 3 * quarter_height {
          GridDirection::North
     } else if pos.x < 3 * quarter_width && pos.y >= 3 * quarter_height {
          GridDirection::East
     } else if pos.x >= 3 * quarter_width && pos.y >= quarter_height {
          GridDirection::South
     } else {
          GridDirection::West
     }
}