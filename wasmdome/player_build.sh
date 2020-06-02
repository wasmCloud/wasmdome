#!/bin/bash

cd ../NPC/dumb-player;
make release;
cp target/wasm32-unknown-unknown/release/dumb_player_signed.wasm ../../wasmdome/;
cd -;
wascap caps dumb_player_signed.wasm;

