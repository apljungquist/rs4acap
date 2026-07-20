use std::io::{BufRead, BufReader};

use anyhow::Context;
use log::debug;

pub trait RunWith {
    fn run_with_processed_output(
        self,
        stdout_func: impl FnMut(std::io::Result<String>) -> anyhow::Result<()>,
        stderr_func: impl FnMut(std::io::Result<String>) -> anyhow::Result<()> + Send,
    ) -> anyhow::Result<()>;
}

fn spawn(mut cmd: std::process::Command) -> anyhow::Result<std::process::Child> {
    match cmd.spawn() {
        Ok(t) => Ok(t),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            let program = cmd.get_program().to_string_lossy().to_string();
            Err(e).context(format!(
                "{program} not found, perhaps it must be installed."
            ))
        }
        Err(e) => Err(e.into()),
    }
}

impl RunWith for std::process::Command {
    fn run_with_processed_output(
        mut self,
        mut stdout_func: impl FnMut(std::io::Result<String>) -> anyhow::Result<()>,
        mut stderr_func: impl FnMut(std::io::Result<String>) -> anyhow::Result<()> + Send,
    ) -> anyhow::Result<()> {
        self.stdout(std::process::Stdio::piped());
        self.stderr(std::process::Stdio::piped());
        debug!("Spawning child {self:#?}...");
        let mut child = spawn(self)?;
        // PANICS:
        // `expect` will never panic because we configured `Stdio::piped()` above, so the child
        // has the handles, and this function is the only place that takes them.
        let stdout = child
            .stdout
            .take()
            .expect("not previously taken by this function");
        let stderr = child
            .stderr
            .take()
            .expect("not previously taken by this function");

        let (stdout_result, stderr_result) = std::thread::scope(|scope| {
            let stderr_thread = scope.spawn(move || -> anyhow::Result<()> {
                for line in BufReader::new(stderr).lines() {
                    stderr_func(line)?;
                }
                Ok(())
            });
            let stdout_result: anyhow::Result<()> = (|| {
                for line in BufReader::new(stdout).lines() {
                    stdout_func(line)?;
                }
                Ok(())
            })();
            let stderr_result = stderr_thread
                .join()
                .expect("the stderr thread does not panic");
            (stdout_result, stderr_result)
        });
        let () = stdout_result?;
        let () = stderr_result?;

        debug!("Waiting for child...");
        let status = child.wait()?;
        if !status.success() {
            anyhow::bail!("Child failed: {status}");
        }
        Ok(())
    }
}
