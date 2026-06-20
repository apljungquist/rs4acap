A replacement for the `acap-build` python script in the ACAP SDK.

This package improves on the upstream tool by:
1. Enabling `acap-build` to be installed without the rest of the SDK.
2. Enabling apps to be built with the SDK in a location different from `/opt/axis/`.
3. Enabling other development tools to use `acap-build` as a library.

<!--
These requirements are driven by the work on `cargo-acap-build` in AxisCommunications/acap-rs.
-->

To that end this package provides both:
- A binary crate designed as a bit-exact drop-in replacement for the upstream script.
- A library crate designed for use by other development tools.

<!--
The binary serves one more purpose;
it pressures the library to support everything users of the upstream tool are accustomed to.
-->

The bit-exactness works on all reproducible apps tested from:
- `AxisCommunications/acap-rs@176d669ec37a5fe35764461d1f23494d3d3822b2`
- `AxisCommunications/acap-native-sdk-examples@36800ed4c28dd96a2b659db3cb2c8a937c61d6d0`

The non-reproducible examples are:
- curl-openssl
- using-opencv
- utility-libraries/openssl_curl_example: probably because `libcrypto.so` is not reproducible.
