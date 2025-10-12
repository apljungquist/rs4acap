use url::Host;

use crate::{
    db::{Database, Device},
    psst::Password,
};

#[derive(Clone, Debug, clap::Parser)]
pub struct AddCommand {
    /// An alias for the device unique within the inventory.
    #[arg()]
    alias: String,
    /// The IP address or hostname of the device
    #[arg(value_parser = url::Host::parse)]
    host: Host,
    /// The username of an administrator on the device, or root.
    #[arg()]
    username: String,
    /// The password of an administrator on the device, or of root.
    #[arg(value_parser = Password::parse)]
    password: Password,
    /// HTTP port to use, if different from default
    #[clap(long)]
    http_port: Option<u16>,
    /// HTTPS port to use, if different from default
    #[clap(long)]
    https_port: Option<u16>,
    /// SSH port to use, if different from default
    #[clap(long)]
    ssh_port: Option<u16>,
}

impl AddCommand {
    pub async fn exec(self, db: Database) -> anyhow::Result<()> {
        let Self {
            alias,
            host,
            username,
            password,
            http_port,
            https_port,
            ssh_port,
        } = self;
        let mut devices = db.read_devices()?;
        devices.insert(
            alias,
            Device {
                host,
                username,
                password,
                http_port,
                https_port,
                ssh_port,
                // TODO: Fetch from device
                model: None,
            },
        );
        db.write_devices(&devices)?;
        Ok(())
    }
}
