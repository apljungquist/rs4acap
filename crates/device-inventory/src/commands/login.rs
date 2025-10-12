use std::io::{self, IsTerminal};

use anyhow::Context;
use rs4a_vlt::{authentication, authentication::AxisConnectSessionSID};

use crate::{db::Database, db_vlt};

#[derive(Clone, Debug, clap::Parser)]
pub struct LoginCommand {
    /// The username to authenticate as, if not using a session id.
    #[arg(long, short)]
    pub username: Option<String>,
}

fn input(prompt: &str) -> anyhow::Result<String> {
    if io::stdin().is_terminal() {
        println!("{prompt}");
    }
    let mut buf = String::new();
    io::stdin()
        .read_line(&mut buf)
        .context("Failed to read from stdin")?;
    Ok(buf)
}

async fn username_password_flow(username: String) -> anyhow::Result<AxisConnectSessionSID> {
    let auth_flow = authentication::AuthenticationFlow::start().await?;
    let password = input("Enter password:")?.trim().to_string();
    let auth_flow = auth_flow.submit(&username, &password).await?;
    let otp = input("Enter OTP code:")?.trim().to_string();
    auth_flow.submit(&otp).await
}

fn direct_input_flow() -> anyhow::Result<String> {
    input("Enter the cookie:")
}

impl LoginCommand {
    pub async fn exec(self, db: Database, offline: bool) -> anyhow::Result<()> {
        let Self { username } = self;
        let cookie = match username {
            None => direct_input_flow()?,
            Some(username) => {
                assert!(!offline);
                username_password_flow(username).await?.to_string()
            }
        };
        db.write_cookie(&cookie)?;
        if !offline {
            db_vlt::import(&db, offline).await?;
        }
        Ok(())
    }
}
