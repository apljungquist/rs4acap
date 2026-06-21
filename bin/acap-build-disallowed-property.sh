#!/bin/sh
set -eu

REPO_ROOT=$(cd $(dirname "$0")/..; pwd)
export OECORE_TARGET_ARCH=aarch64
cd "${REPO_ROOT}/bin/acap-build-disallowed-property"

set -x

acap-build --build no-build . ||:
