#!/usr/bin/env sh
set -eu

unset ACAP_SDK_LOCATION
unset SOURCE_DATE_EPOCH

set -x

acap-build -h
