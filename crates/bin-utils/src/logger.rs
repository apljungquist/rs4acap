use std::{
    env,
    fs::File,
    path::{Path, PathBuf},
};

use log::{info, LevelFilter, Log, Metadata, Record};

struct Tee {
    loggers: Vec<env_logger::Logger>,
}

impl Log for Tee {
    fn enabled(&self, metadata: &Metadata) -> bool {
        self.loggers.iter().any(|logger| logger.enabled(metadata))
    }

    fn log(&self, record: &Record) {
        for logger in &self.loggers {
            if logger.enabled(record.metadata()) {
                logger.log(record);
            }
        }
    }

    fn flush(&self) {
        for logger in &self.loggers {
            logger.flush();
        }
    }
}

pub struct Guard {
    file: Option<PathBuf>,
}

impl Guard {
    pub fn disarm(&mut self) {
        if let Some(file) = self.file.take() {
            info!("Full log stored in {file:?}");
        }
    }
}

impl Drop for Guard {
    fn drop(&mut self) {
        if let Some(file) = self.file.as_ref() {
            eprintln!("Full log stored in {file:?}");
        }
    }
}

fn file_path() -> Option<PathBuf> {
    let current_exe = env::current_exe().ok()?;
    let name = current_exe.file_name()?.to_str()?;
    let path = env::temp_dir().join(name).with_extension("log");
    Some(path)
}

fn file_logger(path: &Path) -> Option<env_logger::Logger> {
    let target = env_logger::Target::Pipe(Box::new(File::create(path).ok()?));
    Some(
        env_logger::Builder::new()
            .filter_level(LevelFilter::Trace)
            .target(target)
            .build(),
    )
}

fn stderr_logger() -> env_logger::Logger {
    env_logger::Builder::new()
        .filter_level(LevelFilter::Warn)
        .parse_default_env()
        .build()
}

pub fn init() -> Guard {
    let file_path = file_path().unwrap();
    let file_logger = file_logger(&file_path).unwrap();
    let stderr_logger = stderr_logger();

    let max_level = file_logger.filter().max(stderr_logger.filter());

    log::set_boxed_logger(Box::new(Tee {
        loggers: vec![file_logger, stderr_logger],
    }))
    .unwrap();
    log::set_max_level(max_level);

    Guard {
        file: Some(file_path),
    }
}
