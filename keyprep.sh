# This script is executed by the Github workflow with the secrets injected
# as environment variables. You should not need to run this locally as you
# should have your own local keys (run `make keys` in each of the actor project folders)

mkdir -p ./command-processor/.keys && echo "$WASMDOME_ACCOUNT_KEY" > ./command-processor/.keys/account.nk
echo "$CMDPROC_MODULE_KEY" > ./command-processor/.keys/module.nk

mkdir -p ./historian/.keys && echo "$WASMDOME_ACCOUNT_KEY" > ./historian/.keys/account.nk
echo "$HISTORIAN_MODULE_KEY" > ./historian/.keys/module.nk

mkdir -p ./leaderboard/.keys && echo "$WASMDOME_ACCOUNT_KEY" > ./leaderboard/.keys/account.nk
echo "$LEADERBOARD_MODULE_KEY" > ./leaderboard/.keys/module.nk 

mkdir -p ./match-coord/.keys && echo "$WASMDOME_ACCOUNT_KEY" > ./match-coord/.keys/account.nk
echo "$MATCHCOORD_MODULE_KEY" > ./match-coord/.keys/module.nk

# Official NPCs
mkdir -p ./NPC/turret/.keys && echo "$WASMDOME_ACCOUNT_KEY" > ./NPC/turret/.keys/account.nk
echo "$TURRET1_MODULE_KEY" > ./NPC/turret/.keys/module.nk 

# Leaf Node credentials for NATS
echo "$GANTRY_NATS_CREDS" > ./.nats/GANTRY.creds