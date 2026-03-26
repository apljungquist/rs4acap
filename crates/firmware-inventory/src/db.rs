use std::{collections::BTreeMap, fmt, fs, path::PathBuf};

use anyhow::Context;
use chrono::NaiveDateTime;
use log::debug;
use rs4a_authentication::{CookieStore, SessionCookie};
use serde::{Deserialize, Serialize};

use crate::version::FirmwareVersion;

const INDEX_FILE_NAME: &str = "index.json";

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProductName(String);

impl ProductName {
    pub fn new(name: String) -> Self {
        Self(name)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ProductName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProductEntry {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_modified: Option<NaiveDateTime>,
    pub versions: Vec<FirmwareVersion>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Index(BTreeMap<ProductName, ProductEntry>);

impl Index {
    pub fn get(&self, key: &ProductName) -> Option<&ProductEntry> {
        self.0.get(key)
    }

    pub fn products(&self) -> impl Iterator<Item = (&ProductName, &ProductEntry)> {
        self.0.iter()
    }

    pub fn insert(&mut self, key: ProductName, value: ProductEntry) {
        self.0.insert(key, value);
    }
}

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

    pub fn read_index(&self) -> anyhow::Result<Index> {
        let file = self.dir.join(INDEX_FILE_NAME);
        match fs::read_to_string(&file) {
            Ok(t) => serde_json::from_str(&t)
                .context("Failed to deserialize index")
                .with_context(|| format!("Consider removing {file:?}")),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                debug!("{INDEX_FILE_NAME} not found, returning an empty index");
                Ok(Index::default())
            }
            Err(e) => Err(e).context("Failed to read index")?,
        }
    }

    pub fn write_index(&self, index: &Index) -> anyhow::Result<()> {
        let index = serde_json::to_string_pretty(index).context("Failed to serialize index")?;
        fs::write(self.dir.join(INDEX_FILE_NAME), index)
            .context("Failed to write index")
            .map(|_| ())
    }

    pub fn firmware_path(&self, product: &ProductName, version: &FirmwareVersion) -> PathBuf {
        let dir_name = version.to_dir_name();
        self.dir
            .join("firmware")
            .join(product.as_str())
            .join(&dir_name)
            .join(format!("{product}_{dir_name}.bin"))
    }
}
