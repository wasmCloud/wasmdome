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

mkdir -p ./NPC/boylur-plait/.keys && echo "$WASMDOME_ACCOUNT_KEY" > ./NPC/boylur-plait/.keys/account.nk
echo "$BOYLUR_MODULE_KEY" > ./NPC/boylur-plait/.keys/module.nk 

mkdir -p ./NPC/sir-emony/.keys && echo "$WASMDOME_ACCOUNT_KEY" > ./NPC/sir-emony/.keys/account.nk
echo "$SIREMONY_MODULE_KEY" > ./NPC/sir-emony/.keys/module.nk 

mkdir -p ./NPC/deploy-jenkins/.keys && echo "$WASMDOME_ACCOUNT_KEY" > ./NPC/deploy-jenkins/.keys/account.nk
echo "$DEPLOY_MODULE_KEY" > ./NPC/deploy-jenkins/.keys/module.nk 


