# Assembly Mechs: Beyond WasmDome

_The year is 2020 and our containerized civilization is falling apart. A cruel and villainous DevOps demon named **Boylur Plait** has descended from the cloud to Earth to challenge mankind to a tournament._

![Sample Match](https://github.com/wascc/wasmdome/blob/master/wasmdome_match.gif)

## Building a Mech

Developers build robots or _mechs_ for competition in the wasmdome using the [wasmdome sdk](https://docs.rs/wasmdome-mech-sdk). There will be a screencast tutorial available walking developers through the process. Once you've created your mech, compiled it, and signed it (the new mech template comes with a `Makefile` pre-configured to generate your keys and sign your mech properly), you're ready to compete.

## Pre-Requisites

Run the following commands to make sure that you have the latest versions of all the required tools:


#### Install Nats
```
curl -sf https://gobinaries.com/nats-io/nats-server | sh
```

#### Install Rust (Optional)
```
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
rustup target add wasm32-unknown-unknown
```

#### Install Required Rust Packages
```
cargo install cargo-generate
cargo install wascap --features "cli" --force
cargo install nkeys --features "cli" --force
cargo install wascc-host --features "bin lattice manifest" --force
```

#### Download and Install Wasmdome Dev Kit
Source code and precompiled binaries for multiple platforms are available under the [releases](https://github.com/wascc/wasmdome/releases) tab.

The following is a list of the tools you'll need to compete on or offline:

* **NATS Server** - You can use `brew` to install it on a Mac, or run `curl -sf https://gobinaries.com/nats-io/nats-server | sh` to install the NATS server on _any_ supported OS and architecture. Don't worry, the NATS binary is less than _20MB_
* **wascap** - You'll need the `wascap` tool so you can embed secure tokens in your WebAssembly modules.
* **nk** - You'll need the `nk` tool to generate the ed25519 keys required to sign secure tokens. 
* **wascc-host** - You'll need `wascc-host` to be able to host actors and capability providers. 
* **WasmDome Dev Kit** - Install the dev kit appropriate for your system. You can find the ZIP files in the [releases](https://github.com/wascc/wasmdome/releases) section of the WasmDome repository. These ZIP files contain a bundle of pre-signed NPCs that you can use in your offline matches, the `wasmdome` CLI tool, and the WasmDome Engine _capability provider_.

## Competing Offline

With all of the pre-requisites set up, and NATS server running (you can run `./nats-server -a 127.0.0.1` to start up an isolated server that supports [waSCC lattice](https://wascc.dev/docs/lattice/overview/)), you can use the `wasmdome` CLI to guide you through the process of running your own local WasmDome arena.

[![Assembly Mechs: Beyond Wasmdome Offline Tutorial](http://img.youtube.com/vi/xjy61n7frHo/0.jpg)](http://www.youtube.com/watch?v=xjy61n7frHo "Assembly Mechs: Beyond Wasmdome Offline Tutorial")

#### Competing Offline Text Instructions
These instructions are transcribed from the video, make sure to watch the video if you have time to get more detail!
Install dependencies with cargo, see the section Download and Install Wasmdome Dev Kit
Run your nats-server locally
```
nats-server -a 127.0.0.1
```

Use cargo generate to create your mech:
```
cargo generate --git https://github.com/wascc/new-mech-template.git
cd your-mech
cargo build
```

Use wascc-host to launch your engine-provider. The file extension will be dylib if you are on a mac.
```
# provider.yaml
---
actors: []
capabilities:
    - path: ./libengine_provider.so
bindings: []
wascc-host --manifest provider.yaml
```
You’re now ready to place your NPCs and mechs into the lattice. You can do this by using wascc-host to run a manifest that contains your mechs, like the example below:
```
# npc_only.yaml
---
actors:
  - ./turret_signed.wasm
bindings:
  - actor: "MCEWIJ5FUAOY2KKMQDQSP7QM4LS7TAU5CAPDQNG4AXJHLRPESSASB4HG"
   capability: "wasmdome:engine"
   values:
capabilities: []
```
And then run the following command to schedule your mechs
```
wascc-host --manifest npc_only.yaml
```
You’re now ready to run a match by using the wasmdome binary included with the GitHub release zip, and running the following, feel free to tweak the arguments. This will launch a match with a world size of 10x10, and a maximum turn limit of 100, and since the Turret is the only mech in the arena it will emerge victorious. 

```
./wasmdome run -h 10 -w 10 -t 100
```

Next you want to add your mech that you generated to this lattice. In the cargo project you generated:
```
make keys   # Generate signing keys for your account and module
make release # Build your mech for release, and sign it with wascap
wascap caps target/wasm32-unknown-unknown/release/your_mech_s.wasm # Inspect capabilities and attributes of the actor
```

Once you have your mech generated, modify the npc_only.yaml file to include your own mech, and create a new actor binding with the module key that is output from the above wascap command. Once you do that, you can schedule your mech by running:
```
# npc_only.yaml
---
actors:
  - ./turret_signed.wasm
  - ./your-mech/target/wasm32-unknown-unknown/release/your_mech_s.wasm
bindings:
  - actor: "MCEWIJ5FUAOY2KKMQDQSP7QM4LS7TAU5CAPDQNG4AXJHLRPESSASB4HG"
   capability: "wasmdome:engine"
   values:
  - actor: "YOUR_MECH_MODULE"
   capability: "wasmdome:engine"
   values:
capabilities: []
```
```
wascc-host --manifest npc_only.yaml
```
Then you can use the above wasmdome run command to run the match. See if your mech beats the turret! Since the autogenerated mech does not have much logic to play the game, go into src/lib.rs and make your mech logic so you can win your match.

## Competing Online

To compete in the online WasmDome arena, you'll want to register at [wasmdome.dev](https://wasmdome.dev). Follow the instructions online and once a match is coming up soon, you'll be able to use the `wasmdome` CLI to claim a set of credentials that you can use to tell your NATS server to run as a _leaf node_ connected to the live, public lattice where the matches take place. You'll be able to see your mech appear in the lobby as soon as you join the public lattice.

[![Assembly Mechs: Beyond Wasmdome Online Tutorial](http://img.youtube.com/vi/PBQ1tyeXrCA/0.jpg)](http://www.youtube.com/watch?v=PBQ1tyeXrCA "Assembly Mechs: Beyond Wasmdome Online Tutorial")
