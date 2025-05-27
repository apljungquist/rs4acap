#!/usr/bin/env sh
set -eu

unset RUST_LOG
export DEVICE_INVENTORY_LOCATION=$(mktemp -d)
export DEVICE_INVENTORY_OFFLINE=true

set -x
device-inventory import --source=json < crates/device-inventory/test-data/get-loans-response.json
device-inventory add local 192.168.0.90 root pass
device-inventory list
device-inventory export
device-inventory for-each sh -- -c 'echo $AXIS_DEVICE_IP'
device-inventory remove --alias 'vlt-*'
device-inventory list
