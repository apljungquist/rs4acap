_Utilities for creating Embedded Application Packages (EAPs)_

This is a drop-replacement for the `acap-build` python script in the ACAP Native SDK.

This package improves on the upstream tool by:
1. Enabling `acap-build` to be installed without the rest of the SDK.
2. Enabling apps to be built with the SDK in a location different from `/opt/axis/`.

## What "drop-in replacement" means

Being a drop in replacement means essentially that users of the upstream `acap-build`
should be able to prepend the port to their `PATH` and expect it to work.

There are aspects in which the observable behavior is expected to differ, notably:
- The replacement may accept CLI flags that the upstream does not
- The stdout and stderr output will be different, but it should not be less helpful.

### How this is tested

In addition to the cargo integration tests and snapshot tests iin this repository
the bit-exactness has been manually verified on all reproducible apps from:
- `AxisCommunications/acap-rs@176d669ec37a5fe35764461d1f23494d3d3822b2`
- `AxisCommunications/acap-native-sdk-examples@36800ed4c28dd96a2b659db3cb2c8a937c61d6d0`

The non-reproducible examples are:
- curl-openssl
- using-opencv
- utility-libraries/openssl_curl_example: probably because `libcrypto.so` is not reproducible.

## Known issues

- The binary calls `tar`, which causes several issues:
  - Users who `cargo install` `acap-build` must remember to also install `tar`.
  - The system `tar` must have a compatible CLI and produce the same output
- The binary does not support `--build meson`
- The binary does not support `--meson-cross-files`
- The binary does not support `--disable-package-creation`
