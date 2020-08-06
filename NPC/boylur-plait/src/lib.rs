extern crate wasmdome_mech_sdk as mech;

use mech::*;

mech_handler!(handler);

pub fn handler(mech: impl MechInstruments) -> Vec<MechCommand> {
     match mech.register_get(EAX) {
          Some(RegisterValue::Number(n)) => {
               let curr_health = u64::from(mech.hull_integrity());
               if curr_health >= n * (2 / 3) {
                    phase_1(&mech)
               } else if curr_health >= n / 3 {
                    phase_2(&mech)
               } else {
                    final_phase(&mech)
               }
          },
          //Storing initial health
          _ => {
               vec![
                    mech.register_set(EAX, RegisterValue::Number(mech.hull_integrity().into())),
                    mech.request_radar(),
                    intimidate(&mech)
               ]
          },
     }
}

// Lurk around, if there's an enemy in secondary range, secondary fire, else if primary range primary fire + move once
fn phase_1(mech: &impl MechInstruments) -> Vec<MechCommand> {

     let last_scan = mech.last_radar_scan().unwrap_or(vec![]);
     let within_secondary = last_scan.iter().filter(|e| enemy_within_range(mech.secondary_range(), &mech.position(), &e.location)).collect::<Vec<_>>();
     let within_primary = within_secondary.iter().filter(|e| enemy_within_range(mech.primary_range(),  &mech.position(), &e.location)).map(|e| *e).collect::<Vec<_>>();

     if let Some(enemy) = find_enemy(within_secondary) {
          vec![
               mech.fire_secondary(mech.direction_to(&enemy.location))
          ]
     } else if let Some(enemy) = find_enemy( within_primary) {
          vec![
               mech.fire_primary(mech.direction_to(&enemy.location)),
               mech.request_radar()
          ]
     } else {
          let mut move_mech = evasive_maneuvers(mech, mech.world_size());
          move_mech.append(&mut vec![mech.request_radar()]);
          move_mech
     }
}

// Secondary fire if no mechs are within primary range, primary if there is an enemy within range and then evasive maneuvers
fn phase_2(mech: &impl MechInstruments) -> Vec<MechCommand> {

     let last_scan = mech.last_radar_scan().unwrap_or(vec![]);
     let within_secondary = last_scan.iter().filter(|e| enemy_within_range(mech.secondary_range(), &mech.position(), &e.location)).collect::<Vec<_>>();
     let within_primary = within_secondary.iter().filter(|e| enemy_within_range(mech.primary_range(),  &mech.position(), &e.location)).map(|e| *e).collect::<Vec<_>>();

     if let Some(enemy) = find_enemy(within_primary) {
               let mut fire = vec![
                    mech.fire_primary(mech.direction_to(&enemy.location)),
               ];
               fire.append(&mut evasive_maneuvers(mech, mech.world_size()));
               fire
     } else if let Some(enemy) = find_enemy(within_secondary) {
          vec![
               mech.fire_secondary(mech.direction_to(&enemy.location))
          ]
     } else {
          let mut move_mech = evasive_maneuvers(mech, mech.world_size());
          move_mech.append(&mut vec![mech.request_radar()]);
          move_mech
     }
}

// This is where I'd normally say, break the rules as boylur plait, 8 aps per turn
// Fight for your life mode, closest foe is gonna die
fn final_phase(mech: &impl MechInstruments) -> Vec<MechCommand> {
     let last_scan = mech.last_radar_scan().unwrap_or(vec![]);
     let within_secondary = last_scan.iter().filter(|e| enemy_within_range(mech.secondary_range(), &mech.position(), &e.location)).collect::<Vec<_>>();
     let within_primary = within_secondary.iter().filter(|e| enemy_within_range(mech.primary_range(),  &mech.position(), &e.location)).map(|e| *e).collect::<Vec<_>>();

     if let Some(enemy) = find_closest_enemy(mech.position(), within_primary) {
               let mut fire = vec![
                    mech.fire_primary(mech.direction_to(&enemy.location)),
               ];
               fire.append(&mut evasive_maneuvers(mech, mech.world_size()));
               fire
     } else if let Some(enemy) = find_closest_enemy(mech.position(), within_secondary) {
          vec![
               mech.fire_secondary(mech.direction_to(&enemy.location))
          ]
     } else {
          let mut move_mech = evasive_maneuvers(mech, mech.world_size());
          move_mech.append(&mut vec![mech.request_radar()]);
          move_mech
     }
}

fn evasive_maneuvers(mech: &impl MechInstruments, gb: GameBoard) -> Vec<MechCommand> {
     let quarter_height = gb.height as i32 / 4;
     let quarter_width = gb.width as i32 / 4;
     let pos = mech.position();

     if pos.x <= quarter_width && pos.y < 3 * quarter_height {
          vec![
               mech.move_mech(GridDirection::North),
               mech.move_mech(GridDirection::North),
               mech.move_mech(GridDirection::East)
          ]
     } else if pos.x < 3 * quarter_width && pos.y >= 3 * quarter_height {
          vec![
               mech.move_mech(GridDirection::East),
               mech.move_mech(GridDirection::East),
               mech.move_mech(GridDirection::South)
          ]
     } else if pos.x >= 3 * quarter_width && pos.y >= quarter_height {
          vec![
               mech.move_mech(GridDirection::South),
               mech.move_mech(GridDirection::South),
               mech.move_mech(GridDirection::West)
          ]
     } else {
          vec![
               mech.move_mech(GridDirection::West),
               mech.move_mech(GridDirection::West),
               mech.move_mech(GridDirection::North)
          ]
     }
}

fn intimidate(mech: &impl MechInstruments) -> MechCommand {
     let d = mech.random_number(0, 7);
     use GridDirection::*;
     match d {
          0 => mech.fire_primary(North),
          1 => mech.fire_primary(NorthEast),
          2 => mech.fire_primary(East),
          3 => mech.fire_primary(SouthEast),
          4 => mech.fire_primary(South),
          5 => mech.fire_primary(SouthWest),
          6 => mech.fire_primary(West),
          _ => mech.fire_primary(NorthWest),
     }
}

// Hey there coder! Here's your boilerplate find enemy function! Use it wisely!
fn find_enemy(scan: Vec<&RadarPing>) -> Option<RadarPing> {
     if let Some(enemy) = scan.iter().filter(|e| e.foe).collect::<Vec<_>>().get(0) {
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

fn find_closest_enemy(mech_position: Point, mut scan: Vec<&RadarPing>) -> Option<RadarPing> {
     //TODO: Ensure this is the correct sort order
     scan.sort_by(|a, b| distance_to(&mech_position, &a.location).cmp(&distance_to(&mech_position, &b.location)));
     if let Some(enemy) = scan.get(0) {
          let ping = RadarPing {
               id: enemy.id.to_string(),
               location: Point::new(enemy.location.x, enemy.location.y),
               foe: enemy.foe,
               distance: enemy.distance
          };
          Some(ping)
     } else {
          None
     }}

fn enemy_within_range(range: u32, friendly_pos: &Point, enemy_pos: &Point) -> bool {
     distance_to(friendly_pos, enemy_pos) <= range
}

// Hey there coder! Seen this before? Who cares, it's useful
fn distance_to(a: &Point, b: &Point) -> u32 {
     ((b.x - a.x).pow(2) as f64 + (b.y - a.y).pow(2) as f64).sqrt() as u32
}