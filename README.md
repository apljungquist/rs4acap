# rs4acap

_A collection of language agnostic tools that facilitate development for the AXIS Camera Application Platform (ACAP)._

## Table of Contents

- [Preview](#preview)
  - [device-finder](#device-finder)
  - [device-inventory](#device-inventory)
- [Related projects](#related-projects)

## Preview

The following tools are available:

### `device-finder`

```console
$ device-finder help
Usage: device-finder [COMMAND]

Commands:
  discover-devices  Discover devices on the local network
  completions       Print a completion file for the given shell
  help              Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

### `device-inventory`

```console
$ device-inventory help
Usage: device-inventory [OPTIONS] <COMMAND>

Commands:
  login        Login to a pool of shared devices
  add          Add a device
  import       Import devices
  for-each     Run a command with environment variables set for each device
  list         List available devices
  export       Print export statements for a device
  remove       Remove a device
  completions  Print a completion file for the given shell
  help         Print this message or the help of the given subcommand(s)

Options:
      --inventory <INVENTORY>  Location of the application data [env: DEVICE_INVENTORY_LOCATION=]
      --offline                [env: DEVICE_INVENTORY_OFFLINE=]
  -h, --help                   Print help
```

## Related projects

- [acap-rs](https://github.com/AxisCommunications/acap-rs) - though focused on facilitating developing ACAP apps in Rust, this project does provide a few language agnostic tools too.
- [awesome-acap](https://github.com/apljungquist/awesome-acap) - a list of free resources related to ACAP development.
