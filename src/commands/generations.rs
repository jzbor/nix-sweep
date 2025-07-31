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
            } else if self.tsv {
                for gen in profile.generations() {
                    let num = gen.number();
                    let path = gen.path().to_string_lossy();
                    let store_path = gen.store_path()
                        .map(|sp| sp.path().to_string_lossy().to_string())
                        .unwrap_or_default();
                    if self.no_size {
                        println!("{num}\t{path}\t{store_path}");
                    } else  {
                        let size = gen.store_path()
                            .map(|sp| sp.closure_size().to_string())
                            .unwrap_or_default();
                        println!("{num}\t{path}\t{store_path}\t{size}");

                    }
                }
            } else {
                profile.list_generations(!self.no_size, false);
            }
        }

        Ok(())
    }
}
