use std::io::{self, IsTerminal};

use anyhow::Context;
use rs4a_vlt::{authentication, authentication::AxisConnectSessionSID};

use crate::{db::Database, db_vlt};

#[derive(Clone, Debug, clap::Args)]
#[group(required = true, multiple = false)]
pub struct LoginCommand {
    /// Authenticate with a username and password.
    #[arg(long, short)]
    username: Option<String>,
    /// Store an existing session.
    #[arg(long, short)]
    cookie: bool,
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
    input("Enter the cookie, including the 'axis_connect_session_sid=' prefix:")
}

impl LoginCommand {
    pub async fn exec(self, db: Database, offline: bool) -> anyhow::Result<()> {
        let Self { username, cookie } = self;
        assert_eq!(username.is_none(), cookie);
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
