FROM debian:bullseye-20250407

# Install required packages
RUN apt-get update \
 && apt-get install -y \
    avahi-discover \
    avahi-utils \
    libnss-mdns \
    mdns-scan \
    systemd \
    systemd-sysv \
    avahi-daemon \
    dbus \
    sysvinit-utils

# Create required directories
RUN mkdir -p /var/run/avahi-daemon && \
    mkdir -p /run/dbus

# Generate machine ID for dbus
RUN dbus-uuidgen --ensure

COPY test.sh / \
