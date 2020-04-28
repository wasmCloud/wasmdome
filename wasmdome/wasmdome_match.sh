#!/bin/bash

# Create Match
export match='{"actors": ["MBQEEEEKX6HCJQBBTCDAUQWRHCG45CQJ543C46ILN4ASVRJ2R7J2E2NK", "MDSZZ7WV4YJQOIXC4CZ5SDKOO5STWD4RMHTLVQZPHF27WJKV4DKVJUOJ"],"match_id": "test_match","board_height": 12,"board_width": 12,"max_turns": 10}'
go run ~/go/src/github.com/nats-io/nats.go/examples/nats-pub/main.go -reply "foo" "wasmdome.matches.create" "$match"
