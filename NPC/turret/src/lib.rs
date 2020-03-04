extern crate wasmdome_mech_sdk as mech;

use mech::*;

mech_handler!(handler);

pub fn handler(mech: impl MechInstruments) -> Vec<MechCommand> {
    let mut cmds = Vec::new();
    if let Some(scan) = mech.last_radar_scan() {
        cmds.extend_from_slice(
            scan.iter()
                .filter_map(|ping| {
                    if ping.foe {
                        Some(mech.fire_primary(mech.direction_to(&ping.location)))
                    } else {
                        None
                    }
                })
                .take(1)
                .collect::<Vec<_>>()
                .as_slice(),
        );
    };
    cmds.push(mech.request_radar());
    cmds
}