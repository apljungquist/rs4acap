#!/usr/bin/env sh
set -eu

base_cmd="device-inventory --offline"
export RUST_LOG=debug

for cmd in \
  "$base_cmd help" \
  "$base_cmd help login" \
  "$base_cmd help add" \
  "$base_cmd help import" \
  "$base_cmd help list" \
  "$base_cmd help export" \
  "$base_cmd help remove"
do
  echo "$ $cmd"
  $cmd
  echo
done

echo "$ $base_cmd import --source=json < crates/device-inventory/test-data/get-loans-response.json"
$base_cmd import --source=json < crates/device-inventory/test-data/get-loans-response.json
echo

for cmd in \
  "$base_cmd list" \
  "$base_cmd export" \
  "$base_cmd remove --alias vlt-12345"
do
  echo "$ $cmd"
  $cmd
  echo
done

echo "$ $base_cmd list"
$base_cmd list
