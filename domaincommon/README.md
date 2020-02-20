# Assembly Mechs: Beyond WasmDome

This crate contains the common _domain logic_ required to perform the match processing. This logic is expressed through _event sourcing_, where commands are handled by an aggregate that results in 1 or more events. Those events are then applied to the aggregate that result in state changes.

This pattern embraces immutability and enables things like replay and incredibly easy testing because everything should be deterministic and predictable.
