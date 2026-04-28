#!/bin/sh
set -eu

cd "$(dirname "$0")/.."

CARGO_HOME="${CARGO_HOME:-${HOME:-}/.cargo}"
if [ -d "$CARGO_HOME/bin" ]; then
  PATH="$CARGO_HOME/bin:$PATH"
fi

cargo fmt -- --check
cargo clippy --all-targets -- -D warnings
cargo test
if command -v cargo-llvm-cov >/dev/null 2>&1; then
  sh scripts/coverage.sh
else
  echo "coverage skipped: install cargo-llvm-cov to run scripts/coverage.sh"
fi
cargo build

state_file="${TMPDIR:-/tmp}/our-cli-check-memory.txt"
target/debug/our-cli --help >/dev/null
OUR_CLI_STATE="$state_file" target/debug/our-cli reset >/dev/null
OUR_CLI_STATE="$state_file" OUR_CLI_MOCK_RESPONSE="offline helper ok" OUR_CLI_MOCK_TOTAL_TOKENS=42 target/debug/our-cli test >/dev/null
OUR_CLI_STATE="$state_file" target/debug/our-cli history | grep "offline helper ok" >/dev/null
