use clap_complete::{generate, Shell};

/// Print a completion file for the given shell.
///
/// Example: `cargo-acap-sdk completions zsh | source /dev/stdin`.
#[derive(Debug, clap::Parser)]
pub struct CompletionsCommand {
    shell: Shell,
}

impl CompletionsCommand {
    pub fn exec<T: clap::Parser>(self) -> anyhow::Result<()> {
        let Self { shell } = self;
        let mut cmd = T::command();
        let name = cmd.get_name().to_string();
        generate(shell, &mut cmd, name, &mut std::io::stdout());
        Ok(())
    }
}
