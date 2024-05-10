#!/usr/bin/env bash

# This script runs an end-to-end test of the summoner ceremony in CI.

set -euo pipefail

# This script also runs the devnet. The reason for this is that if testnet
# preview is redeployed during the run of this script, the script will fail
# as the chain ID will be different.

# If `SUMMONER_SMOKE_RESET=1` is set, automatically reset the testnet data
# directory, and clear the summonerd directory. This is helpful if running this
# test in a loop with e.g. `entr` or `cargo watch`.
if [[ "${SUMMONER_SMOKE_RESET:-0}" -eq '1' ]] ; then
    cargo run --release --bin pd -- testnet unsafe-reset-all
    rm -rf /tmp/summonerd
    rm -rf /tmp/account1
fi

# Fail fast if testnet dir exists, otherwise `cargo run ...` will block
# for a while, masking the error.
if [[ -d ~/.penumbra/testnet_data ]] ; then
    >&2 echo "ERROR: testnet data directory exists at ~/.penumbra/testnet_data"
    >&2 echo "Not removing this directory automatically; to remove, run: pd testnet unsafe-reset-all"
    exit 1
fi

# Fail fast if `/tmp/` directories created by this test already exist.
# Otherwise, this test may be inheriting stale state from a previous run.
if [[ -d /tmp/summonerd ]] ; then
    >&2 echo "ERROR: summonerd directory exists at /tmp/summonerd"
    >&2 echo "Not removing this directory automatically; to remove, run: rm -rf /tmp/summonerd/"
    exit 1
fi
if [[ -d /tmp/account1 ]] ; then
    >&2 echo "ERROR: account1 directory exists at /tmp/account1"
    >&2 echo "Not removing this directory automatically; to remove, run: rm -rf /tmp/account1/"
    exit 1
fi

# Fail fast if `cometbft` is not in the $PATH, we are missing software to
# run this smoke test.
if ! hash cometbft > /dev/null 2>&1 ; then
    >&2 echo "ERROR: cometbft not found in PATH"
    >&2 echo "See install guide: https://guide.penumbra.zone/main/pd/build.html"
    exit 1
fi

export RUST_LOG="summonerd=info,pd=info,penumbra=info"

# Duration that the network will run before we start the ceremony.
TESTNET_BOOTTIME="${TESTNET_BOOTTIME:-20}"

echo "Building latest version of pd from source..."
cargo build --quiet --release --bin pd

echo "Generating testnet config..."
EPOCH_DURATION="${EPOCH_DURATION:-100}"
cargo run --quiet --release --bin pd -- testnet generate --epoch-duration "$EPOCH_DURATION" --timeout-commit 500ms

echo "Starting CometBFT..."
cometbft start --log_level=error --home "${HOME}/.penumbra/testnet_data/node0/cometbft" &
cometbft_pid="$!"

echo "Starting pd..."
cargo run --quiet --release --bin pd -- start --home "${HOME}/.penumbra/testnet_data/node0/pd" &
pd_pid="$!"

# Ensure processes are cleaned up after script exits, regardless of status.
trap 'kill -9 "$cometbft_pid" "$pd_pid"' EXIT INT

echo "Waiting $TESTNET_BOOTTIME seconds for network to boot..."
sleep "$TESTNET_BOOTTIME"

# This is not a secret, it is a test account seed phrase, used for integration tests like this one only.
export SEED_PHRASE="comfort ten front cycle churn burger oak absent rice ice urge result art couple benefit cabbage frequent obscure hurry trick segment cool job debate"

echo "Building latest version of summonerd from source..."
cargo build --quiet --release --bin summonerd

echo "Generating phase 1 root..."
cargo run --quiet --release --bin summonerd -- generate-phase1 --output phase1.bin

echo "Setting up storage directory..."
mkdir /tmp/summonerd
cargo run --quiet --release --bin pcli -- --home /tmp/summonerd init --grpc-url http://127.0.0.1:8080 soft-kms generate
export SUMMONER_ADDRESS=$(PCLI_UNLEASH_DANGER="yes" cargo run --quiet --release --bin pcli -- --home /tmp/summonerd view address 0 2>&1)
echo $SUMMONER_ADDRESS
export SUMMONER_FVK=$(grep "full_viewing_key" /tmp/summonerd/config.toml | cut -d= -f2 | tr -d ' "')
cargo run --quiet --release --bin summonerd -- init --storage-dir /tmp/summonerd --phase1-root phase1.bin

echo "Starting phase 1 run..."
cargo run --quiet --release --bin summonerd -- start --phase 1 --storage-dir /tmp/summonerd --fvk $SUMMONER_FVK --node http://127.0.0.1:8080 --bind-addr 127.0.0.1:8082 &
phase1_pid="$!"
# If script ends early, ensure phase 1 is halted.
trap 'kill -9 "$cometbft_pid" "$pd_pid" "$phase1_pid"' EXIT INT

echo "Setting up test accounts..."
# We are returning 0 always here because the backup wallet file does not respect the location of
# the home directory, and so if there is already a backup wallet, we refuse to overwrite it,
# and will exit non-zero. We don't care about the backup wallet for this test, so we ignore the
# exit code.
echo $SEED_PHRASE | cargo run --quiet --release --bin pcli -- --home /tmp/account1 init --grpc-url http://127.0.0.1:8080 soft-kms import-phrase || true
export ACCOUNT1_ADDRESS=$(PCLI_UNLEASH_DANGER="yes" cargo run --quiet --release --bin pcli -- --home /tmp/account1 view address 0 2>&1)
echo $ACCOUNT1_ADDRESS

echo "Phase 1 contributions..."
cargo run --quiet --release --bin pcli -- --home /tmp/account1 ceremony contribute --coordinator-url http://127.0.0.1:8082 --coordinator-address $SUMMONER_ADDRESS --phase 1 --bid 10penumbra

echo "Stopping phase 1 run..."
if ! kill -0 "$phase1_pid" ; then
    >&2 echo "ERROR: phase 1 exited early"
    exit 1
else
    echo "Phase 1 complete. Stopping phase 1 run..."
    kill -9 "$phase1_pid"
fi

echo "Transitioning..."
cargo run --quiet --release --bin summonerd -- transition --storage-dir /tmp/summonerd

echo "Starting phase 2 run..."
cargo run --quiet --release --bin summonerd -- start --phase 2 --storage-dir /tmp/summonerd --fvk $SUMMONER_FVK --node http://127.0.0.1:8080 --bind-addr 127.0.0.1:8082 &
phase2_pid="$!"
# If script ends early, ensure phase 2 is halted.
trap 'kill -9 "$cometbft_pid" "$pd_pid" "$phase1_pid" "$phase2_pid' EXIT INT

echo "Phase 2 contributions..."
cargo run --quiet --release --bin pcli -- --home /tmp/account1 ceremony contribute --coordinator-url http://127.0.0.1:8082 --coordinator-address $SUMMONER_ADDRESS --phase 2 --bid 10penumbra

echo "Exporting keys..."
cargo run --quiet --release --bin summonerd -- export --storage-dir /tmp/summonerd --target-dir ./crates/crypto/proof-params/src/gen

# We have a set of tests in the pcli crate that generate and verify proofs using the
# proving keys present in the `penumbra-proof-params` crate.
echo "Check tests pass using the new proving keys..."
cargo test -p pcli

echo "Stopping phase 2 run..."
if ! kill -0 "$phase2_pid" ; then
    >&2 echo "ERROR: phase 2 exited early"
    exit 1
else
    echo "Phase 2 complete. Stopping phase 2 run..."
    kill -9 "$phase2_pid"
fi

rm -rf /tmp/summonerd
rm -rf /tmp/account1
exit 0
