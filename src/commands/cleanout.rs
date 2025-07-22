use std::path;
use std::str::FromStr;

use colored::Colorize;

use crate::config::{self, ConfigPreset};
use crate::interaction::*;
use crate::fmt::FmtAge;
use crate::profiles::Profile;

use super::gc::GCCommand;


#[derive(clap::Args)]
pub struct CleanoutCommand {
    /// Settings for clean out criteria
    #[clap(short, long, default_value_t = config::DEFAULT_PRESET.to_owned())]
    preset: String,

    /// Alternative config file
    #[clap(short('C'), long)]
    config: Option<path::PathBuf>,

    #[clap(flatten)]
    cleanout_config: config::ConfigPreset,

    /// List, but do not actually delete old generations
    #[clap(short, long)]
    dry_run: bool,

    /// Do not calculate the size of generations
    #[clap(long)]
    no_size: bool,

    /// Profiles to clean out; valid values: system, user, home, <path_to_profile>
    #[clap(required = true)]
    profiles: Vec<String>,
}

impl super::Command for CleanoutCommand {
    fn run(self) -> Result<(), String> {
        self.cleanout_config.validate()?;
        let config = ConfigPreset::load(&self.preset, self.config)?
            .override_with(&self.cleanout_config);
        let interactive = config.interactive.is_none() || config.interactive == Some(true);

        // println!("{:#?}", config);

        for profile_str in self.profiles {
            let mut profile = Profile::from_str(&profile_str)?;
            profile.apply_markers(&config);

            if self.dry_run {
                profile.list_generations(!self.no_size, true);
            } else if interactive {
                profile.list_generations(!self.no_size, true);

                let confirmation = ask("Do you want to delete the marked generations?", false);
                println!();
                if confirmation {
                    remove_generations(&profile);
                } else {
                    println!("-> Not touching profile\n");
                }
            } else {
                remove_generations(&profile);
            }
        }

        if config.gc == Some(true) {
            let gc_cmd = GCCommand::new(interactive, self.dry_run);
            gc_cmd.run()?;
        }

        Ok(())
    }
}

fn remove_generations(profile: &Profile) {
    announce_removal(profile);
    for gen in profile.generations() {
        let age_str = FmtAge::new(gen.age()).to_string();
        if gen.marked() {
            println!("{}", format!("-> Removing generation {} ({} old)", gen.number(), age_str).bright_blue());
            resolve(gen.remove());
        } else {
            println!("{}", format!("-> Keeping generation {} ({} old)", gen.number(), age_str).bright_black());
        }
    }
    println!();
}

