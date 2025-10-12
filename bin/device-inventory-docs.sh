#!/usr/bin/env sh
set -eux
unset RUST_LOG

# Special commands in alphabetical order
device-inventory help
device-inventory help completions
# Normal commands in alphabetical order
device-inventory help activate
device-inventory help add
device-inventory help deactivate
device-inventory help for-each
device-inventory help import
device-inventory help list
device-inventory help login
device-inventory help remove
device-inventory help return
