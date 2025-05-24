use crate::db::Device;

pub fn envs(device: &Device) -> Vec<(String, String)> {
    let mut envs = Vec::new();

    // TODO: Consider resolving to IPv4 if possible.
    envs.push(("AXIS_DEVICE_IP".to_string(), device.host.to_string()));
    envs.push(("AXIS_DEVICE_USER".to_string(), device.username.to_string()));
    envs.push((
        "AXIS_DEVICE_PASS".to_string(),
        device.password.dangerous_reveal().to_string(),
    ));
    if let Some(p) = device.ssh_port {
        envs.push(("AXIS_DEVICE_SSH_PORT".to_string(), p.to_string()));
    }
    if let Some(p) = device.http_port {
        envs.push(("AXIS_DEVICE_HTTP_PORT".to_string(), p.to_string()));
    }
    if let Some(p) = device.https_port {
        envs.push(("AXIS_DEVICE_HTTPS_PORT".to_string(), p.to_string()));
    }
    envs.push(("AXIS_DEVICE_HTTPS_SELF_SIGNED".to_string(), "1".to_string()));

    envs
}
