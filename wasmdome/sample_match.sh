#!/bin/bash

# Create Match
export match='{"actors": ["MB6ZF3LZ7X3EODFK3ZNIA7EU42EF6N434I43WI7DW5H4TCJB7S2X25BZ"],"match_id": "test_match","board_height": 3,"board_width": 3,"max_turns": 50, "aps_per_turn": 4}'
go run ~/go/src/github.com/nats-io/nats.go/examples/nats-pub/main.go -reply "foo" "wasmdome.matches.create" "$match"
