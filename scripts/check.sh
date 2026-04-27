#!/bin/sh
set -eu

cd "$(dirname "$0")/.."

cargo fmt -- --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build

state_file="${TMPDIR:-/tmp}/our-cli-check-memory.txt"
target/debug/our-cli --help >/dev/null
OUR_CLI_STATE="$state_file" target/debug/our-cli reset >/dev/null
OUR_CLI_STATE="$state_file" OUR_CLI_MOCK_RESPONSE="offline helper ok" OUR_CLI_MOCK_TOTAL_TOKENS=42 target/debug/our-cli test >/dev/null
OUR_CLI_STATE="$state_file" target/debug/our-cli history | grep "offline helper ok" >/dev/null
