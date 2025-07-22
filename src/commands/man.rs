use std::{fs, path};

use clap::CommandFactory;


#[derive(clap::Args)]
pub struct ManCommand {
    directory: path::PathBuf,
}

impl super::Command for ManCommand {
    fn run(self) -> Result<(), String> {
        // export main
        let man = clap_mangen::Man::new(crate::Args::command());
        let mut buffer: Vec<u8> = Default::default();
        man.render(&mut buffer)
            .map_err(|e| e.to_string())?;
        let file = self.directory.join("nix-sweep.1");
        fs::write(&file, buffer)
            .map_err(|e| e.to_string())?;
        println!("Written {}", file.to_string_lossy());

        for subcommand in crate::Args::command().get_subcommands() {
            let man = clap_mangen::Man::new(subcommand.clone());
            let mut buffer: Vec<u8> = Default::default();
            man.render(&mut buffer)
                .map_err(|e| e.to_string())?;
            let file = self.directory.join(format!("nix-sweep-{}.1", subcommand));
            fs::write(&file, buffer)
                .map_err(|e| e.to_string())?;
            println!("Written {}", file.to_string_lossy());
        }

        Ok(())
    }
}
