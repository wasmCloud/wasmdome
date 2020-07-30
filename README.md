# Assembly Mechs: Beyond WasmDome

_The year is 2020 and our containerized civilization is falling apart. A cruel and villainous DevOps demon named **Boylur Plait** has descended from the cloud to Earth to challenge mankind to a tournament._

## Building a Mech

Developers build robots or _mechs_ for competition in the wasmdome using the [wasmdome sdk](https://docs.rs/wasmdome-mech-sdk). There will be a screencast tutorial available walking developers through the process. Once you've created your mech, compiled it, and signed it (the new mech template comes with a `Makefile` pre-configured to generate your keys and sign your mech properly), you're ready to compete.

## Competing Offline

You'll need the following tools installed on your machine to host your mech and compete offline:

* **NATS Server** - You can use `brew` to install it on a Mac, or run `curl -sf https://gobinaries.com/nats-io/nats-server | sh` to install the NATS server on _any_ supported OS and architecture. Don't worry, the NATS binary is less than _20MB_
* **wascap** - You'll need the `wascap` tool so you can embed secure tokens in your WebAssembly modules. Use `cargo install wascap --features "cli"` to install it, optionally adding `--force` if you're upgrading to the latest version from a previous one.
* **nk** - You'll need the `nk` tool to generate the ed25519 keys required to sign secure tokens. Use `cargo install nkeys --features "cli"` to install the tool
* **wascc-host** - You'll need `wascc-host` to be able to host actors and capability providers. Use `cargo install wascc-host --features "bin lattice manifest"` to install the tool with all the options we need to compete in the arena
* **WasmDome Dev Kit** - Install the dev kit appropriate for your system. You can find the ZIP files in the [releases](https://github.com/wascc/wasmdome/releases) section of the WasmDome repository. These ZIP files contain a bundle of pre-signed NPCs that you can use in your offline matches, the `wasmdome` CLI tool, and the WasmDome Engine _capability provider_.

With all of this set up, and NATS server running (you can run `./nats-server -a 127.0.0.1` to start up an isolated server that supports [waSCC lattice](https://wascc.dev/docs/lattice/overview/)), you can use the `wasmdome` CLI to guide you through the process of running your own local WasmDome arena.

## Competing Online

To compete in the online WasmDome arena, you'll want to register at [wasmdome.dev](https://wasmdome.dev). Follow the instructions online and once a match is coming up soon, you'll be able to use the `wasmdome` CLI to claim a set of credentials that you can use to tell your NATS server to run as a _leaf node_ connected to the live, public lattice where the matches take place. You'll be able to see your mech appear in the lobby as soon as you join the public lattice.

