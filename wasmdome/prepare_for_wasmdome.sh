#!/bin/bash

echo ""
echo "This script will build all submodules of this crate, and will copy the necessary actors into /wasmdome/ in order to prepare you for offline CLI"
echo "Please ensure all necessary keys are in place"
echo ""

cd ../command-processor
make release
cp target/wasm32-unknown-unknown/release/command_processor_signed.wasm ../wasmdome/

cd ../domaincommon
cargo build

cd ../historian
make release
cp target/wasm32-unknown-unknown/release/historian_signed.wasm ../wasmdome/

cd ../hosts
cargo build

cd ../leaderboard
make build

cd ../match-coord
make release
cp target/wasm32-unknown-unknown/release/match_coord_signed.wasm ../wasmdome/

cd ../mech-sdk
cargo build

cd ../NPC
cd corner-turret
make release

cd ../turret
make release

cd ../../wasmdome
cargo build

echo ""
echo "Ready for wasmdome"
echo ""
