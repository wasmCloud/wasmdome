#!/bin/bash

echo ""
echo "This script will build all submodules of this crate, and will copy the necessary actors into /wasmdome/ in order to prepare you for offline CLI"
echo "Please ensure all necessary keys are in place"
echo ""

cd ../command-processor
make release

cd ../domaincommon
cargo build

cd ../historian
make release

cd ../hosts
cargo build

cd ../leaderboard
make build

cd ../match-coord
make release

cd ../mech-sdk
cargo build

cd ../NPC/turret
make release

cd ../../wasmdome
cargo build

echo ""
echo "Ready for wasmdome"
echo ""
