use std::{fmt::Write, fs::File, io::BufReader, path::PathBuf};

use anyhow::Context;
use chrono::SecondsFormat;

use crate::{
    archive::read_info_json,
    info::{ImageInfo, Product},
};

#[derive(Clone, Debug, clap::Args)]
pub struct InspectCommand {
    /// Path to a firmware image, i.e. a `.bin` file.
    image: PathBuf,
    /// Print the metadata as JSON.
    #[arg(long)]
    json: bool,
}

fn render(info: &ImageInfo) -> anyhow::Result<String> {
    let build_time = info.try_build_time()?;
    let ImageInfo {
        release,
        build_nbr,
        part_nbr,
        build_time: _,
        signing_domain,
        track,
        upgradeable_from,
        products,
    } = info;
    let mut out = String::new();
    writeln!(out, "Release: {release}")?;
    writeln!(out, "Build number: {build_nbr}")?;
    writeln!(out, "Part number: {part_nbr}")?;
    writeln!(
        out,
        "Build time: {}",
        build_time.to_rfc3339_opts(SecondsFormat::Secs, true)
    )?;
    writeln!(out, "Signing domain: {signing_domain}")?;
    if let Some(track) = track {
        writeln!(out, "Track: {track}")?;
    }
    if let Some(upgradeable_from) = upgradeable_from {
        writeln!(out, "Upgradeable from: {}", upgradeable_from.join(", "))?;
    }
    for Product {
        prod_full_name,
        hardware_id,
        prod_variant,
        ..
    } in products
    {
        match prod_variant {
            Some(prod_variant) => writeln!(
                out,
                "Product: {prod_full_name} {prod_variant} ({hardware_id})"
            )?,
            None => writeln!(out, "Product: {prod_full_name} ({hardware_id})")?,
        }
    }
    Ok(out)
}

impl InspectCommand {
    pub fn exec(self) -> anyhow::Result<String> {
        let Self { image, json } = self;
        let file =
            File::open(&image).with_context(|| format!("Failed to open {}", image.display()))?;
        let info: ImageInfo = read_info_json(BufReader::new(file))?
            .parse()
            .context("Failed to parse the image info")?;
        if json {
            Ok(format!("{}\n", serde_json::to_string_pretty(&info)?))
        } else {
            render(&info)
        }
    }
}
