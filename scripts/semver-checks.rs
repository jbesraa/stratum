#!/bin/bash

# Install dependencies
sudo apt-get update
sudo apt-get install -y cmake

# Install cargo-semver-checks
cargo install cargo-semver-checks --version 0.37.0 --locked

# Run semver checks (using a function for DRYness)
run_semver_checks() {
  # Make sure we are in project root
  pushd "$(git rev-parse --show-toplevel)" || exit 1
  # First input is the crate directory
  local dir="$1"
  echo "Running semver checks for: $dir"
  # Move to the crate directory
  pushd "$dir" || exit 1
  # Perform the semver checks
  # if second input is set, pass it to cargo-semver-checks
  if [[ -n "$2" ]]; then
    cargo semver-checks "$2"
  else
    cargo semver-checks
  fi
  if [[ $? -ne 0 ]]; then
    echo "Semver checks failed for: $dir"
    exit 1  # Exit script if any check fails
  fi
}

run_semver_checks "common"
run_semver_checks "utils/buffer"
run_semver_checks "protocols/v2/binary-sv2/codec"
run_semver_checks "protocols/v2/binary-sv2"
run_semver_checks "protocols/v2/const-sv2"
run_semver_checks "protocols/v2/framing-sv2"
run_semver_checks "protocols/v2/noise-sv2"
run_semver_checks "protocols/v2/codec-sv2"
run_semver_checks "protocols/v2/subprotocols/common-messages"
run_semver_checks "protocols/v2/subprotocols/job-declaration"
run_semver_checks "protocols/v2/subprotocols/mining"
run_semver_checks "protocols/v2/subprotocols/template-distribution"
run_semver_checks "protocols/v2/sv2-ffi"
run_semver_checks "protocols/v2/roles-logic-sv2" "--default-features"
run_semver_checks "protocols/v1"
run_semver_checks "utils/bip32-key-derivation"
run_semver_checks "utils/error-handling"
run_semver_checks "utils/key-utils"
run_semver_checks "roles/roles-utils/network-helpers"
run_semver_checks "roles/roles-utils/rpc"

echo "All semver checks completed."
