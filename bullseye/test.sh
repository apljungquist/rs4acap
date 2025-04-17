#!/usr/bin/env sh
set -eux

set -eux
service dbus start
service avahi-daemon start
avahi-browse --all
