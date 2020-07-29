COLOR ?= always # Valid COLOR options: {always, auto, never}
CARGO = cargo --color $(COLOR)

.PHONY: all clean doc

test:
	cd mech-sdk && cargo test

release: test		
	cd engine-provider && cargo build --verbose --release
#	cd historian && make release
#	cd leaderboard && make release	
	cd NPC/turret && make release
	cd NPC/corner-turret && make release
