use std::fs;
use std::path;

use clap::CommandFactory;
use clap_complete::Shell;


#[derive(clap::Args)]
pub struct CompletionsCommand {
    directory: path::PathBuf,
}

impl super::Command for CompletionsCommand {
    fn run(self) -> Result<(), String> {
        let mut command = crate::Args::command();
        let shells = &[
            (Shell::Bash, "bash"),
            (Shell::Zsh, "zsh"),
            (Shell::Fish, "fish"),
            (Shell::PowerShell, "ps1"),
            (Shell::Elvish, "elv"),
        ];

        for (shell, ending) in shells {
            let mut file = fs::File::create(self.directory.join(format!("nix-sweep.{}", ending)))
                .map_err(|e| e.to_string())?;
            clap_complete::aot::generate(*shell, &mut command, "nix-sweep", &mut file);
        }

        Ok(())
    }
}
