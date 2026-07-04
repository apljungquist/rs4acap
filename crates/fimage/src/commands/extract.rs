use std::{
    fs::File,
    io::BufReader,
    path::{Path, PathBuf},
};

use anyhow::Context;

use crate::archive;

#[derive(Clone, Debug, clap::Args)]
pub struct ExtractCommand {
    /// Path to a firmware image, i.e. a `.bin` file.
    image: PathBuf,
    /// Directory to extract into.
    ///
    /// Defaults to a directory in the current working directory named after the image.
    directory: Option<PathBuf>,
}

fn default_directory(image: &Path) -> anyhow::Result<PathBuf> {
    let stem = image.file_stem().context("Image path has no file name")?;
    Ok(PathBuf::from(stem))
}

impl ExtractCommand {
    pub fn exec(self) -> anyhow::Result<String> {
        let Self { image, directory } = self;
        let directory = match directory {
            Some(directory) => directory,
            None => default_directory(&image)?,
        };
        let file =
            File::open(&image).with_context(|| format!("Failed to open {}", image.display()))?;
        archive::unpack(BufReader::new(file), &directory)?;
        Ok(format!("{}\n", directory.display()))
    }
}
