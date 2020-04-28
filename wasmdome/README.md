# Wasmdome Local CLI

This project is designed to allow for a fully offline developer experience of wasmdome by loading all required capability providers as static providers, and adding local wasm files in order to fill the roles of the `command-processor`, `match-coordinator`, and `historian`

Once you have corresponding `.keys` directories in each of the individual modules of this project that come with a `Makefile`, you can run the script `prepare_for_wasmdome.sh` which will build and copy all required `wasm` files into this directory. It will also build the domain, leaderboard, hosts, and protocol in case you want to do any local development.

You can always skip the `prepare_for_wasmdome.sh` script and build them manually, if you'd prefer. At the moment the script is very fragile and has no error checking.