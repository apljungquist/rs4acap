use std::io::{self, Read};

use anyhow::Context;

use crate::vlt;

#[derive(clap::Parser, Debug, Clone)]
pub struct ExportLoansCommand;

impl ExportLoansCommand {
    pub async fn exec(self) -> anyhow::Result<()> {
        let mut content = String::new();
        io::stdin()
            .read_to_string(&mut content)
            .context("Failed to read from stdin")?;

        let loan = vlt::parse(&content)?;
        println!("export AXIS_DEVICE_IP={}", loan.effective_ip()?);
        println!("export AXIS_DEVICE_USER={}", loan.username());
        println!(
            "export AXIS_DEVICE_PASS={}",
            loan.password().dangerous_reveal()
        );
        println!("export AXIS_DEVICE_HTTP_PORT={}", loan.http_port());
        println!("export AXIS_DEVICE_HTTPS_PORT={}", loan.https_port());
        println!("export AXIS_DEVICE_SSH_PORT={}", loan.ssh_port());
        Ok(())
    }
}
