use std::{
    path::{Path, PathBuf},
    process::Command,
};

use libtest_mimic::{Arguments, Failed, Trial};

fn main() -> std::process::ExitCode {
    libtest_mimic::run(&Arguments::from_args(), trials()).exit_code()
}

fn trials() -> Vec<Trial> {
    let file = Path::new(file!());
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join(file.file_stem().expect("this file has a stem"));
    let pattern = format!("{}/**/*.bin", dir.display());
    glob::glob(&pattern)
        .expect("the pattern is valid")
        .map(|image| {
            let image = image.expect("the images are readable");
            let name = image
                .strip_prefix(&dir)
                .expect("the images are in the searched directory")
                .display()
                .to_string();
            Trial::test(name, move || inspect(&image))
        })
        .collect()
}

fn inspect(image: &PathBuf) -> Result<(), Failed> {
    let output = Command::new(env!("CARGO_BIN_EXE_fimage"))
        .arg("inspect")
        .arg(image)
        .output()
        .map_err(|e| format!("could not run fimage: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "fimage inspect exited with {}; stderr:\n{}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    if output.stdout.is_empty() {
        return Err("expected metadata on stdout, got nothing".into());
    }
    Ok(())
}
