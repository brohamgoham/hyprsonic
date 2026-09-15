#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.."

started=$SECONDS
cargo build --offline --locked --target-dir target/phase0
mkdir -p artifacts
binary=target/phase0/debug/hyprsonic

"$binary" demo | tee artifacts/phase0-demo.log

# Exit 2 is part of the contract: an infeasible plan must fail the command.
expect_infeasible() {
  local output=$1
  shift
  if "$binary" replay --scenario delayed --plan wallet "$@" > "$output"; then
    echo "ERROR: wallet plan unexpectedly succeeded" >&2
    exit 1
  else
    local status=$?
    if [[ $status != 2 ]]; then
      echo "ERROR: expected infeasible exit 2, received $status" >&2
      exit 1
    fi
  fi
}

expect_infeasible artifacts/wallet-failure.log
expect_infeasible artifacts/wallet-failure.json --json
echo ""
echo "Demo completed in $((SECONDS - started)) seconds. Infeasible replay exit=2 verified."
echo "Full fail log: artifacts/wallet-failure.log"
echo "Machine-readable ledger + exact fixture: artifacts/wallet-failure.json"
