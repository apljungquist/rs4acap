#!/usr/bin/env sh
set -eux
unset RUST_LOG

device-inventory help
device-inventory help login
device-inventory help add
device-inventory help import
device-inventory help list
device-inventory help export
device-inventory help remove
