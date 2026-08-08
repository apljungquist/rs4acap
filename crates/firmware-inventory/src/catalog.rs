use anyhow::Context;
use log::debug;
use quick_xml::{events::Event, Reader};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogEntry {
    /// Product directory name.
    pub product: String,
    /// Dotted version string.
    pub revision: String,
    /// Path to the fimage, relative to the software root.
    pub fileurl: String,
}

fn parse_product(fileurl: &str) -> Option<String> {
    let mut parts = fileurl.trim_start_matches('/').splitn(3, '/');
    parts.next()?;
    let product = parts.next()?;
    parts.next()?;
    Some(product.to_string())
}

pub fn parse_catalog(xml: &str) -> anyhow::Result<Vec<CatalogEntry>> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut entries = Vec::new();
    let mut buf = Vec::new();
    loop {
        buf.clear();
        let event = reader
            .read_event_into(&mut buf)
            .context("Failed to parse catalog XML")?;
        match event {
            Event::Eof => break,
            Event::Start(tag) | Event::Empty(tag) if tag.name().as_ref() == b"software" => {
                let mut fileurl = None;
                let mut revision = None;
                for attr in tag.attributes() {
                    let attr = attr.context("Failed to parse a <software> attribute")?;
                    let value = attr
                        .decode_and_unescape_value(reader.decoder())
                        .context("Failed to decode a <software> attribute")?
                        .into_owned();
                    match attr.key.as_ref() {
                        b"fileurl" => fileurl = Some(value),
                        b"revision" => revision = Some(value),
                        _ => {}
                    }
                }
                let Some(fileurl) = fileurl else {
                    debug!("Skipping <software> element with no fileurl attribute");
                    continue;
                };
                let Some(revision) = revision else {
                    debug!("Skipping <software fileurl={fileurl:?}> with no revision attribute");
                    continue;
                };
                let Some(product) = parse_product(&fileurl) else {
                    debug!(
                        "Skipping <software> element with an unexpected fileurl shape: {fileurl:?}"
                    );
                    continue;
                };
                entries.push(CatalogEntry {
                    product,
                    revision,
                    fileurl,
                });
            }
            _ => {}
        }
    }
    Ok(entries)
}
