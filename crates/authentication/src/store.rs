use std::{fs, path::PathBuf, str::FromStr};

use anyhow::Context;
use log::debug;

use crate::SessionCookie;

const COOKIE_FILE_NAME: &str = "cookie";

#[derive(Debug)]
pub struct CookieStore(PathBuf);

impl CookieStore {
    pub fn new(dir: PathBuf) -> Self {
        Self(dir)
    }

    /// Open the default shared cookie store at `<data_dir>/rs4a/`.
    pub fn open_default() -> anyhow::Result<Self> {
        let dir = dirs::data_dir()
            .context("Could not infer a data directory")?
            .join("rs4a");
        fs::create_dir_all(&dir).context("Failed to create the cookie store directory")?;
        Ok(Self(dir))
    }

    pub fn read(&self) -> anyhow::Result<Option<SessionCookie>> {
        match fs::read_to_string(self.0.join(COOKIE_FILE_NAME)) {
            Ok(t) => Some(SessionCookie::from_str(&t)).transpose(),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                debug!("{COOKIE_FILE_NAME} not found, returning None");
                Ok(None)
            }
            Err(e) => Err(e).context("Failed to read cookie"),
        }
    }

    pub fn write(&self, cookie: &SessionCookie) -> anyhow::Result<()> {
        fs::write(self.0.join(COOKIE_FILE_NAME), cookie.to_string().trim())
            .context("Failed to write cookie")
            .map(|_| ())
    }
}
