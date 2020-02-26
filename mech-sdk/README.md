# Assembly Mechs: Beyond WasmDome - Mech SDK

This is the SDK used by developers for building mechs to compete in the _[WasmDome](https://wasmdome.dev)_. Developers building mechs that will compete for nothing less than the survival of the planet will do so by responding to turn events delivered by the game engine and return a vector of mech commands..

Here's an example of a mech that simply moves north and fires every turn (obviously you will want a more clever strategy if you want to win):

```rust
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
```
