#!/bin/bash

workspaces=("benches" "common" "roles" "protocols" "utils")

for workspace in "${workspaces[@]}"; do
  echo "Running fmt for workspace: $workspace"
  cargo fmt --all --manifest-path="$workspace/Cargo.toml" -- --check
  if [[ $? -ne 0 ]]; then
    echo "Fmt failed for workspace: $workspace"
    exit 1
  else
    echo "Fmt passed for workspace: $workspace"
  fi
done

echo "Fmt success!"
exit 0
