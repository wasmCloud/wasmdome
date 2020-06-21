COLOR ?= always # Valid COLOR options: {always, auto, never}
CARGO = cargo --color $(COLOR)

.PHONY: all clean doc

test:
	cd mech-sdk && cargo test

release: test	
	#cd command-processor && make release
	cd engine-provider && make release
	cd historian && make release
	cd leaderboard && make release
	#cd match-coord && make release
#	cd wasmdome && cargo build --verbose --release
	
