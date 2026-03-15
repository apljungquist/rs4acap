use std::{collections::BTreeMap, fs, path::PathBuf};

use anyhow::Context;
use log::debug;
use rs4a_authentication::{CookieStore, SessionCookie};

const INDEX_FILE_NAME: &str = "index.json";

pub struct Database {
    dir: PathBuf,
    cookie_store: CookieStore,
}

impl Database {
    pub fn open_or_create(data_dir: Option<PathBuf>) -> anyhow::Result<Self> {
        let (db_dir, cookie_store) = match data_dir {
            None => {
                let dir = dirs::data_dir()
                    .context("Could not infer a data directory")?
                    .join("rs4a-firmware-inventory");
                (dir, CookieStore::open_default()?)
            }
            Some(custom) => {
                let cookie_store = CookieStore::new(custom.clone());
                (custom, cookie_store)
            }
        };
        fs::create_dir_all(&db_dir).context("Failed to create the data directory")?;
        Ok(Self {
            dir: db_dir,
            cookie_store,
        })
    }

    pub fn read_cookie(&self) -> anyhow::Result<Option<SessionCookie>> {
        self.cookie_store.read()
    }

    pub fn write_cookie(&self, cookie: &SessionCookie) -> anyhow::Result<()> {
        self.cookie_store.write(cookie)
    }

    pub fn read_index(&self) -> anyhow::Result<BTreeMap<String, Vec<String>>> {
        let file = self.dir.join(INDEX_FILE_NAME);
        match fs::read_to_string(&file) {
            Ok(t) => serde_json::from_str(&t)
                .context("Failed to deserialize index")
                .with_context(|| format!("Consider removing {file:?}")),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                debug!("{INDEX_FILE_NAME} not found, returning an empty index");
                Ok(BTreeMap::new())
            }
            Err(e) => Err(e).context("Failed to read index")?,
        }
    }

    pub fn write_index(&self, index: &BTreeMap<String, Vec<String>>) -> anyhow::Result<()> {
        let index = serde_json::to_string_pretty(index).context("Failed to serialize index")?;
        fs::write(self.dir.join(INDEX_FILE_NAME), index)
            .context("Failed to write index")
            .map(|_| ())
    }

    pub fn firmware_path(&self, product: &str, version: &str) -> PathBuf {
        self.dir
            .join("firmware")
            .join(product)
            .join(version)
            .join(format!("{product}_{version}.bin"))
    }
}
