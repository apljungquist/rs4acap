# device-manager — implementation notes

## VAPIX endpoints used

| Endpoint | Protocol | Used by |
|----------|----------|---------|
| `/axis-cgi/systemready.cgi` | JSON-RPC | restore, init |
| `/axis-cgi/firmwaremanagement.cgi` | JSON-RPC | restore |
| `/axis-cgi/param.cgi` | query string | init |
| `/axis-cgi/pwdgrp.cgi` | query string | init |
| `/axis-cgi/applications/config.cgi` | query string | init (11.2 -- 12.x only) |

## Restore

### Factory-default detection

Query the `systemready` API. The device is in factory-default state when `needsetup` is true.

### Factory default request

Use `firmwaremanagement` with soft mode.

### Restart detection

After requesting a factory default, poll `systemready` at 1-second intervals to detect that the device has restarted. Use one of three strategies, chosen based on which fields the initial `systemready` response includes:

- **Boot-ID-based** (preferred): Detect when `bootid` changes. Available on modern firmware.
- **Uptime-based**: Detect when `uptime` decreases. Fallback when `bootid` is absent.
- **State-transition-based**: Detect when the device transitions from not-ready back to ready. Last resort when neither `bootid` nor `uptime` is available.

## Init

### Anonymous connection

The device is in setup mode and requires no authentication. Connect without credentials, then verify `needsetup` is true before proceeding.

### User creation

Call `pwdgrp` to create a `root` user with group `Root` and role `AdminOperatorViewerPtz`.

### SSH setup

After reconnecting with the newly created credentials:

1. Set parameter `root.Network.SSH.Enabled` to `yes` via `param.cgi`.
2. Run `ssh-keygen -R <host>` to remove stale host keys. Log a warning on failure.

### Setup profiles

The firmware version reported by the device after authentication determines which post-setup steps to apply.

#### AXIS OS < 11.2

No additional setup. Unsigned ACAP applications are always accepted; there is no toggle.

#### AXIS OS 11.2 -- 12.x

Set `AllowUnsigned=true` via `GET /axis-cgi/applications/config.cgi?action=set&name=AllowUnsigned&value=true`.

The toggle was introduced in 11.2 (default `true`). In 12.0 the default changed to `false`. Applying it unconditionally within this range is harmless.

#### AXIS OS >= 13.0

No action for unsigned apps. The `AllowUnsigned` toggle is removed in 13.0; calling the endpoint would fail.

## Client construction

Use `rs4a_vapix::ClientBuilder`:

- Set host, optional port overrides, and optional basic authentication.
- Call `.danger_accept_invalid_certs(true)` (devices use self-signed certificates).
- Use `.build_with_automatic_scheme()` to probe HTTPS then HTTP.
