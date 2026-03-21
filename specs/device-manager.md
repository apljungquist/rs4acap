# device-manager

Provision and manage AXIS OS devices over the network.

## Commands

### `reinit`

Restore a device to factory defaults, then initialize it to a known, useful state.

```
device-manager reinit [OPTIONS]
```

Equivalent to running `restore` followed by `init`.

Refuse to run if `--user` is not `root`, since `init` always creates a `root` user. The initial user is hardcoded to `root` because older firmware requires it, and using the same user across all versions keeps development simple.

### `restore`

Restore a device to factory defaults.

```
device-manager restore [OPTIONS]
```

1. Connect to the device using the provided credentials.
2. If the device is already in factory-default state, return early.
3. Request a factory default.
4. Wait for the device to restart.

### `init`

Initialize a device that is in setup mode.

```
device-manager init [OPTIONS]
```

Fails if the device is not in factory-default state.

Refuse to run if `--user` is not `root`, since the initial user is always `root`.

1. Create a `root` user with the password from `--pass` and full admin permissions.
2. Enable SSH.
3. Remove the device from the local SSH known hosts file. Non-fatal on failure.
4. On AXIS OS 11.2 -- 12.x, allow unsigned ACAP applications. On other versions, skip this step.

### `completions`

Generate shell completions.

```
device-manager completions <SHELL>
```

## Common options

All device-targeting commands accept:

| Flag | Env | Default | Description |
|------|-----|---------|-------------|
| `--host` | `AXIS_DEVICE_IP` | required | Device hostname or IP |
| `--http-port` | `AXIS_DEVICE_HTTP_PORT` | | HTTP port override |
| `--https-port` | `AXIS_DEVICE_HTTPS_PORT` | | HTTPS port override |
| `-u, --user` | `AXIS_DEVICE_USER` | `root` | Username |
| `-p, --pass` | `AXIS_DEVICE_PASS` | `pass` | Password |

Scheme detection is automatic: try HTTPS first, fall back to HTTP.

## Non-fatal operations

Some operations are expected to fail depending on the environment. These log a warning and continue:

- Removing the device from known hosts (`ssh-keygen` may not be available)
