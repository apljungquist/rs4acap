#!/bin/sh
# Install the ACAP Native SDK to /opt/axis, where the upstream acap-build expects it.
#
# The upstream acap-build hardcodes /opt/axis for its manifest tools, so the SDK must live there
# rather than at an arbitrary location. This extracts /opt/axis from the official SDK image for the
# architecture the fuzzer targets. Requires docker and a writable /opt/axis.
#
# acap-build additionally needs the python package jsonschema, which is not under /opt/axis;
# install it separately, e.g. with `apt-get install python3-jsonschema`.
#
# The version and Ubuntu base default to the same values as nix/acap-native-sdk.nix so that the
# reference matches the schema the binary under test is validated against.
set -eu

SDK_IMAGE="axisecp/acap-native-sdk"
SDK_VERSION="${SDK_VERSION:-12.1.0}"
SDK_UBUNTU="${SDK_UBUNTU:-ubuntu24.04}"
SDK_ARCH="${SDK_ARCH:-aarch64}"

image="$SDK_IMAGE:$SDK_VERSION-$SDK_ARCH-$SDK_UBUNTU"
docker pull --quiet "$image"

# Stream /opt/axis out of the image and unpack it into the (already writable) /opt/axis. Archiving
# from inside the container avoids `docker cp`, which cannot populate the SDK's read-only
# directories; `--no-same-permissions` keeps the unpacked directories writable for the same reason.
mkdir -p /opt/axis
docker run --rm "$image" tar -cC /opt/axis . | tar -xC /opt/axis --no-same-permissions
