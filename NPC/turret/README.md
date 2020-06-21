# Turret Mech

This turret is designed to stay in the location in which it was spawned. It will request a radar scan every turn. If it has a successful radar scan available, it will pick the first _enemy_ target from that scan and fire in that enemy's direction with its primary weapon. This effectively means the turret could fire its primary weapon once per turn throughout the entire match.

**NOTE** by default this turret is tagged as an **npc** and as such will fire upon your mechs.

## Running

To ensure this actor is included in your local **Wasmdome** matches:

1. Make sure you've got a NATS instance running either on loopback or 0.0.0.0 (and configure your lattice environment variable accordingly)
1. Make sure you've got the [provider](../engine-provider) running.
1. Start up the host for your actor by running `wascc-host ./turret_host.yaml`
1. Your turret is ready to compete!
