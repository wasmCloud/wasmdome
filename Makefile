COLOR ?= always # Valid COLOR options: {always, auto, never}
CARGO = cargo --color $(COLOR)

.PHONY: all clean doc

test:
	cd mech-sdk && cargo test

release: test		
	cd wasmdome && cargo build --release --verbose
	cd engine-provider && cargo build --verbose --release

	cd NPC/turret && make release
	cd NPC/corner-turret && make release
	cd NPC/kode-frieze && make release
	cd NPC/sir-emony && make release 
	cd NPC/deploy-jenkins && make release
	cd NPC/boylur-plait && make release
