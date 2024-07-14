#!/bin/sh

PROGRAM_SIZE=$(ls -l $HOME/projects/superteam/stuk-whitelist/program/target/deploy/stuk_wl.so | awk '{print int($5 * 1.1)}')
PROGRAM_ID=$HOME/projects/superteam/stuk-whitelist/test-pid.json

echo "Compiling whitelist program..."
cd ./program
cargo build-bpf

echo "Deploying whitelist program..."
solana program deploy ./target/deploy/stuk_wl.so --program-id $PROGRAM_ID --max-len $PROGRAM_SIZE

echo "Compiling cli..."
cd ../cli
cargo build --release

echo "Installing cli..."
cargo install --path .

echo "Setup complete"
