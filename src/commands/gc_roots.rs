use std::cmp::Reverse;
use std::time::Duration;

use colored::Colorize;
use rayon::slice::ParallelSliceMut;

use crate::fmt::*;
use crate::interaction::{announce_gc_roots, resolve};
use crate::roots::GCRoot;
use crate::{roots, HashSet};

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

    /// Present the long, verbose form
    #[clap(short, long)]
    long: bool,
}

impl super::Command for GCRootsCommand {
    fn run(self) -> Result<(), String> {
        let calc_size = !(self.no_size || self.paths);
        let mut roots = roots::gc_roots(self.include_missing)?;
        roots.par_sort_by_key(|r| r.link().clone());
        roots.par_sort_by_key(|r| Reverse(r.age().cloned().unwrap_or(Duration::MAX)));

        let nroots_total = roots.len();

        roots = roots.into_iter()
            .filter(|r| self.include_profiles || !r.is_profile() )
            .filter(|r| self.include_current || !r.is_current() )
            .filter(|r| !self.exclude_inaccessible || r.is_accessible())
            .collect();

        let nroots_listed = roots.len();

        if !self.tsv && !self.paths {
            announce_gc_roots(nroots_total, nroots_listed);
        }

        for root in &roots {
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

            let closure_size = if calc_size {
                root.closure_size().ok()
            } else {
                None
            };

            if self.paths {
                println!("{}", root.link().to_string_lossy());
            } else if self.tsv {
                let path = root.store_path().as_ref().map(|p| p.path().to_string_lossy().to_string())
                    .unwrap_or(String::from("n/a"));
                println!("{}\t{}", root.link().to_string_lossy(), path);
            } else if self.long {
                root.print_fancy(closure_size);
            } else {
                root.print_concise(closure_size);
            }
        }

        if !self.paths && !self.tsv {
            println!();
            let full_closure = resolve(GCRoot::full_closure(&roots));
            let total_size = resolve(GCRoot::full_closure_size(&roots));
            println!("Estimated total size: {} ({} store paths)",
                FmtSize::new(total_size).to_string().yellow(), full_closure.len());
            println!();
        }

        Ok(())
    }
}
