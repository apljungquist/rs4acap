#!/usr/bin/env sh
set -eux

rm -f /run/dbus/pid /var/run/avahi-daemon/pid
dbus-daemon --system --nofork --nopidfile &
sleep 1
avahi-daemon --no-chroot -D
avahi-browse --all