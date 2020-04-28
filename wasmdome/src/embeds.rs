pub(crate) fn command_processor_actor() -> Vec<u8> {
    include_bytes!("../../command-processor/target/wasm32-unknown-unknown/release/command_processor_signed.wasm")
        .to_vec()
}

pub(crate) fn match_coord_actor() -> Vec<u8> {
    include_bytes!(
        "../../match-coord/target/wasm32-unknown-unknown/release/match_coord_signed.wasm"
    )
    .to_vec()
}

pub(crate) fn historian_actor() -> Vec<u8> {
    include_bytes!("../../historian/target/wasm32-unknown-unknown/release/historian_signed.wasm")
        .to_vec()
}
