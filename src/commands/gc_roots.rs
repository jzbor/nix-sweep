use std::cmp::Reverse;
use std::time::Duration;

use rayon::slice::ParallelSliceMut;

use crate::roots;

#[derive(clap::Args)]
pub struct GCRootsCommand {
    /// Only print the paths
    #[clap(long)]
    paths: bool,

    /// Present list as tsv
    #[clap(long)]
    tsv: bool,

    /// Include profiles
    #[clap(short('p'), long)]
    include_profiles: bool,

    /// Include current
    #[clap(short('c'), long)]
    include_current: bool,

    /// Include gc roots that are referenced, but could not be found
    #[clap(long)]
    include_missing: bool,

    /// Exclude gc roots, whose store path is not accessible
    #[clap(short, long)]
    exclude_inaccessible: bool,

    #[clap(long, value_parser = |s: &str| duration_str::parse_std(s))]
    older: Option<Duration>,

    #[clap(long, value_parser = |s: &str| duration_str::parse_std(s))]
    newer: Option<Duration>,

    /// Do not calculate the size of generations
    #[clap(long)]
    no_size: bool,
}

impl super::Command for GCRootsCommand {
    async fn run(self) -> Result<(), String> {
        let mut roots = roots::gc_roots(self.include_missing)?;
        roots.par_sort_by_key(|r| r.link().clone());
        roots.par_sort_by_key(|r| Reverse(r.age().cloned().unwrap_or(Duration::MAX)));

        for root in roots {
            if !self.include_profiles && root.is_profile() {
                continue
            }
            if !self.include_current && root.is_current() {
                continue
            }
            if self.exclude_inaccessible && !root.is_accessible() {
                continue
            }
            if let Some(older) = &self.older {
                if let Ok(age) = root.age() {
                    if age <= older {
                        continue
                    }
                }
            }
            if let Some(newer) = &self.newer {
                if let Ok(age) = root.age() {
                    if age >= newer {
                        continue
                    }
                }
            }

            if self.paths {
                println!("{}", root.link().to_string_lossy());
            } else if self.tsv {
                let path = root.store_path().as_ref().map(|p| p.path().to_string_lossy().to_string())
                    .unwrap_or(String::from("n/a"));
                println!("{}\t{}", root.link().to_string_lossy(), path);
            } else {
                root.print_fancy(!self.no_size).await;
            }
        }

        if !self.paths && !self.tsv {
            println!();
        }
        Ok(())
    }
}
