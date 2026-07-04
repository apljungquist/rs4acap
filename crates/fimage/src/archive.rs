//! Read AXIS OS firmware image (`.bin`) archives.

use std::{
    io::{Cursor, Read},
    path::Path,
};

use anyhow::{bail, Context};
use flate2::read::GzDecoder;

// Images for AXIS OS before 13 are compressed with gzip, images for AXIS OS 13 and later with zstd.
const GZIP_MAGIC: [u8; 2] = [0x1f, 0x8b];
const ZSTD_MAGIC: [u8; 4] = [0x28, 0xb5, 0x2f, 0xfd];
const INFO_FILE_NAME: &str = "info.json";

/// Wrap an image in a decompressor appropriate for its magic numbers.
fn decompress<'a>(mut image: impl Read + 'a) -> anyhow::Result<Box<dyn Read + 'a>> {
    let mut magic = [0u8; 4];
    image
        .read_exact(&mut magic)
        .context("Failed to read the first bytes of the image")?;
    let image = Cursor::new(magic).chain(image);
    if magic == ZSTD_MAGIC {
        Ok(Box::new(
            ruzstd::StreamingDecoder::new(image)
                .context("Failed to start decompressing the image")?,
        ))
    } else if magic.starts_with(&GZIP_MAGIC) {
        Ok(Box::new(GzDecoder::new(image)))
    } else {
        bail!("unrecognized magic numbers: {magic:?} (expected gzip or zstd)");
    }
}

/// Extract the content of the `info.json` member from an AXIS OS firmware image.
///
/// Returns as soon as the member is found; in the images seen so far it is
/// the second member, before the large ones.
pub fn read_info_json(image: impl Read) -> anyhow::Result<String> {
    let mut archive = tar::Archive::new(decompress(image)?);
    for member in archive
        .entries()
        .context("Failed to read the archive inside the image")?
    {
        let mut member = member.context("Failed to read a member of the archive")?;
        if member.path()?.to_str() == Some(INFO_FILE_NAME) {
            let mut text = String::new();
            member
                .read_to_string(&mut text)
                .with_context(|| format!("Failed to read {INFO_FILE_NAME}"))?;
            return Ok(text);
        }
    }
    bail!("{INFO_FILE_NAME} not found in the archive");
}

/// Extract all members of an AXIS OS firmware image into a directory.
pub fn unpack(image: impl Read, directory: &Path) -> anyhow::Result<()> {
    let mut archive = tar::Archive::new(decompress(image)?);
    archive
        .unpack(directory)
        .with_context(|| format!("Failed to extract the archive into {}", directory.display()))
}
