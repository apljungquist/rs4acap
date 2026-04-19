use std::{
    io::{self, IsTerminal},
    str::FromStr,
};

use anyhow::Context;
use rs4a_authentication::{AuthenticationFlow, SessionCookie};

use crate::db::Database;

#[derive(Clone, Debug, clap::Args)]
#[group(required = true, multiple = false)]
pub struct LoginCommand {
    /// Authenticate with a username and password.
    #[arg(long, short)]
    pub username: Option<String>,
    /// Store an existing session.
    #[arg(long, short)]
    pub cookie: bool,
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

async fn username_password_flow(username: String) -> anyhow::Result<SessionCookie> {
    let auth_flow = AuthenticationFlow::start().await?;
    let password = input("Enter password:")?.trim().to_string();
    let auth_flow = auth_flow.submit(&username, &password).await?;
    let otp = input("Enter OTP code:")?.trim().to_string();
    auth_flow.submit(&otp).await
}

fn direct_input_flow() -> anyhow::Result<String> {
    input("Enter the cookie, including the 'axis_connect_session_sid=' prefix:")
}

impl LoginCommand {
    pub(crate) async fn exec(self, db: Database) -> anyhow::Result<String> {
        let Self { username, cookie } = self;
        assert_eq!(username.is_none(), cookie);
        let cookie = match username {
            None => SessionCookie::from_str(direct_input_flow()?.as_str())?,
            Some(username) => username_password_flow(username).await?,
        };
        db.write_cookie(&cookie)?;
        Ok(String::new())
    }
}
