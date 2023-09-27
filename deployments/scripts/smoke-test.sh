#!/bin/bash
# Wrapper script to bottle up logic for running "smoke tests" in CI,
# supporting backgrounding tasks and checking on their status later.
# The execution plan is:
#
#   1. Start the network
#   2. Wait ~10s
#   3. Run integration tests (fail here if non-zero)
#   4. Continue running network ~5m
#
# The goal is to fail fast if an integration test exits, but permit
# a slightly longer runtime for the suite to find more errors.
set -euo pipefail


# Fail fast if testnet dir exists, otherwise `cargo run ...` will block
# for a while, masking the error.
if [[ -d ~/.penumbra/testnet_data ]] ; then
    >&2 echo "ERROR: testnet data directory exists at ~/.penumbra/testnet_data"
    >&2 echo "Not removing this directory automatically; to remove, run: pd testnet unsafe-reset-all"
    exit 1
fi

if ! hash cometbft > /dev/null 2>&1 ; then
    >&2 echo "ERROR: cometbft not found in PATH"
    >&2 echo "See install guide: https://guide.penumbra.zone/main/pd/build.html"
    exit 1
fi

export RUST_LOG="pclientd=info,pcli=info,pd=info,penumbra=info"

# Duration that the network will be left running before script exits.
TESTNET_RUNTIME="${TESTNET_RUNTIME:-120}"
# Duration that the network will run before integration tests are run.
TESTNET_BOOTTIME="${TESTNET_BOOTTIME:-20}"

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
trap 'kill -9 "$cometbft_pid" "$pd_pid"' EXIT

echo "Waiting $TESTNET_BOOTTIME seconds for network to boot..."
sleep "$TESTNET_BOOTTIME"

echo "Running pclientd integration tests against network"
PENUMBRA_NODE_PD_URL="http://127.0.0.1:8080" \
    PCLI_UNLEASH_DANGER="yes" \
    cargo test --quiet --release --features sct-divergence-check --package pclientd -- --ignored --test-threads 1 --nocapture

echo "Running pcli integration tests against network"
PENUMBRA_NODE_PD_URL="http://127.0.0.1:8080" \
    PCLI_UNLEASH_DANGER="yes" \
    cargo test --quiet --release --features sct-divergence-check,download-proving-keys --package pcli -- --ignored --test-threads 1 --nocapture

echo "Waiting another $TESTNET_RUNTIME seconds while network runs..."
sleep "$TESTNET_RUNTIME"
# `kill -0` checks existence of pid, i.e. whether the process is still running.
# It doesn't inspect errors, but the only reason the process would be stopped
# is if it failed, so it's good enough for our needs.
if ! kill -0 "$cometbft_pid" || ! kill -0 "$pd_pid" ; then
    >&2 echo "ERROR: smoke test process exited early"
    exit 1
else
    echo "SUCCESS! Smoke test complete. Ran for $TESTNET_RUNTIME, found no errors."
fi
exit 0
