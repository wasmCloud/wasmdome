#!/bin/bash

# Create Match
export match='{"actors": ["MBQEEEEKX6HCJQBBTCDAUQWRHCG45CQJ543C46ILN4ASVRJ2R7J2E2NK", "MDB3X2OJECJ3F23SYW727ACEVKCXUAR4QQE5GXWA3HTNEHDRATO4P6P4"],"match_id": "test_match","board_height": 3,"board_width": 3,"max_turns": 50, "aps_per_turn": 4}'
go run ~/go/src/github.com/nats-io/nats.go/examples/nats-pub/main.go -reply "foo" "wasmdome.matches.create" "$match"
