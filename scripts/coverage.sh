#!/bin/sh
set -eu

cd "$(dirname "$0")/.."

CARGO_HOME="${CARGO_HOME:-${HOME:-}/.cargo}"
if [ -d "$CARGO_HOME/bin" ]; then
  PATH="$CARGO_HOME/bin:$PATH"
fi

if ! command -v cargo-llvm-cov >/dev/null 2>&1; then
  echo "cargo-llvm-cov is required for coverage."
  echo "Install it with: cargo install cargo-llvm-cov"
  exit 1
fi

if command -v xcrun >/dev/null 2>&1; then
  if [ -z "${LLVM_COV:-}" ]; then
    llvm_cov_path="$(xcrun -f llvm-cov 2>/dev/null || true)"
    if [ -n "$llvm_cov_path" ]; then
      LLVM_COV="$llvm_cov_path"
      export LLVM_COV
    fi
  fi

  if [ -z "${LLVM_PROFDATA:-}" ]; then
    llvm_profdata_path="$(xcrun -f llvm-profdata 2>/dev/null || true)"
    if [ -n "$llvm_profdata_path" ]; then
      LLVM_PROFDATA="$llvm_profdata_path"
      export LLVM_PROFDATA
    fi
  fi
fi

cargo llvm-cov \
  --workspace \
  --all-targets \
  --summary-only \
  --ignore-filename-regex '(^|/)src/chat/(terminal|interactive)\.rs$' \
  --fail-under-lines 85
