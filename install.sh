#!/bin/sh

PROGRAM_SIZE=$(ls -l $HOME/projects/solana/scalar/tokenizer/target/deploy/sclr_token.so | awk '{print int($5 * 1.1)}')
PROGRAM_ID="path/to/desired/program/id/keypair"

echo "Compiling whitelist program..."
cd ./program
cargo build-bpf

echo "Deploying tokenizer..."
solana program deploy ./target/deploy/stuk_wl.so --program-id $PROGRAM_ID --max-len $PROGRAM_SIZE

echo "Compiling cli..."
cd ../cli
cargo build --release

echo "Installing cli..."
cargo install --path .

echo "Setup complete"
