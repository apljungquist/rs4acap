#FROM debian:bookworm-20240423
FROM rust:latest

# Install required packages
RUN apt-get update && \
    apt-get install -y \
    avahi-daemon \
    avahi-utils \
    dbus \
    procps \
    init-system-helpers && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# Create required directories
RUN mkdir -p /var/run/avahi-daemon && \
    mkdir -p /run/dbus

# Generate machine ID for dbus
RUN dbus-uuidgen --ensure

COPY test.sh / \
