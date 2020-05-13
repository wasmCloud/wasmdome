# This script is executed by the Github workflow with the secrets injected
# as environment variables. You should not need to run this locally as you
# should have your own local keys (run `make keys` in each of the actor project folders)

mkdir -p ./command-processor/.keys && echo "$WASMDOME_ACCOUNT_KEY" > ./command-processor/.keys/account.nk
echo "$CMDPROC_MODULE_KEY" > ./command-processor/.keys/module.nk