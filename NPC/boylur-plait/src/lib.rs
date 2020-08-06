extern crate wasmdome_mech_sdk as mech;

use mech::*;

mech_handler!(handler);

pub fn handler(mech: impl MechInstruments) -> Vec<MechCommand> {
     vec![
          mech.request_radar(),
          mech.move_mech(GridDirection::North),
          mech.fire_primary(GridDirection::South)
     ]
}