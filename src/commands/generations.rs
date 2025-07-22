use std::str::FromStr;

use crate::profiles::Profile;


#[derive(clap::Args)]
pub struct GenerationsCommand {
    /// Only print the paths
    #[clap(long)]
    paths: bool,

    /// Present list as tsv
    #[clap(long)]
    tsv: bool,

    /// Do not calculate the size of generations
    #[clap(long)]
    no_size: bool,

    /// Profiles to list; valid values: system, user, home, <path_to_profile>
    #[clap(required = true)]
    profiles: Vec<String>,
}

impl super::Command for GenerationsCommand {
    fn run(self) -> Result<(), String> {
        for profile_str in self.profiles {
            let profile = Profile::from_str(&profile_str)?;

            if self.paths {
                for gen in profile.generations() {
                    println!("{}", gen.path().to_string_lossy());
                }
            } else {
                profile.list_generations(!self.no_size, false);
            }
        }

        Ok(())
    }
}
