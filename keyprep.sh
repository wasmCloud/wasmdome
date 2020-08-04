# This script is executed by the Github workflow with the secrets injected
# as environment variables. You should not need to run this locally as you
# should have your own local keys (run `make keys` in each of the actor project folders)

# Official NPCs
mkdir -p ./NPC/turret/.keys && echo "$WASMDOME_ACCOUNT_KEY" > ./NPC/turret/.keys/account.nk
echo "$TURRET1_MODULE_KEY" > ./NPC/turret/.keys/module.nk 

mkdir -p ./NPC/corner-turret/.keys && echo "$WASMDOME_ACCOUNT_KEY" > ./NPC/corner-turret/.keys/account.nk
echo "$TURRET2_MODULE_KEY" > ./NPC/corner-turret/.keys/module.nk 

mkdir -p ./NPC/kode-frieze/.keys && echo "$WASMDOME_ACCOUNT_KEY" > ./NPC/kode-frieze/.keys/account.nk
echo "$KODEFRIEZE_MODULE_KEY" > ./NPC/kode-frieze/.keys/module.nk 
