use std::cmp::Reverse;
use std::time::Duration;

use colored::Colorize;
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
use rayon::slice::ParallelSliceMut;

use crate::utils::fmt::*;
use crate::utils::interaction::announce;
use crate::utils::ordered_channel::OrderedChannel;
use crate::nix::roots::GCRoot;

#[derive(clap::Args)]
pub struct GCRootsCommand {
    /// Present the long, verbose form
    #[clap(short, long)]
    long: bool,

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

    /// Include gc roots from running processes
    #[clap(long)]
    include_proc: bool,

    /// Exclude gc roots, whose store path is not accessible
    #[clap(short, long)]
    exclude_inaccessible: bool,

    /// Only show gc roots older than OLDER
    #[clap(long, value_parser = |s: &str| duration_str::parse_std(s))]
    older: Option<Duration>,

    /// Only show gc roots newer than NEWER
    #[clap(long, value_parser = |s: &str| duration_str::parse_std(s))]
    newer: Option<Duration>,

    /// Do not calculate the size of generations
    #[clap(long)]
    no_size: bool,

    /// Query Nix for gc roots instead of enumerating the directory
    #[clap(long)]
    query_nix: bool,
}

impl super::Command for GCRootsCommand {
    fn run(self) -> Result<(), String> {
        let print_size = !(self.no_size || self.paths);
        let mut roots = GCRoot::all(self.query_nix, self.include_proc, self.include_missing)?;
        let nroots_total = roots.len();
        roots.par_sort_by_key(|r| r.link().clone());
        roots.par_sort_by_key(|r| Reverse(r.age().cloned().unwrap_or(Duration::MAX)));

        roots = GCRoot::filter_roots(roots, self.include_profiles, self.include_current,
            !self.exclude_inaccessible, self.older, self.newer);
        let nroots_listed = roots.len();

        if !self.tsv && !self.paths {
            announce(format!("Listing {nroots_listed} gc roots (out of {nroots_total} total)"));
        }

        let max_link_len = roots.iter()
            .map(|r| r.link().to_string_lossy().len())
            .max()
            .unwrap_or(0);

        let ordered_channel: OrderedChannel<_> = OrderedChannel::new();
        rayon::join( || {
            roots.par_iter()
                .enumerate()
                .map(|(i, root)| match print_size {
                    true => (i, (root, root.closure_size().ok())),
                    false => (i, (root, None)),
                })
                .for_each(|(i, tup)| ordered_channel.put(i, tup));
        }, || {
            for (root, closure_size) in ordered_channel.iter(nroots_listed) {
                if self.paths {
                    println!("{}", root.link().to_string_lossy());
                } else if self.tsv {
                    let path = root.store_path().as_ref().map(|p| p.path().to_string_lossy().to_string())
                        .unwrap_or_default();
                    if self.no_size {
                        println!("{}\t{}", root.link().to_string_lossy(), path);
                    } else {
                        let size = closure_size.as_ref().map(|s| s.to_string())
                            .unwrap_or(String::from("n/a"));
                        println!("{}\t{}\t{}", root.link().to_string_lossy(), path, size);
                    }
                } else if self.long {
                    root.print_fancy(closure_size, !self.no_size);
                } else {
                    root.print_concise(closure_size, !self.no_size, max_link_len);
                }
            }
        });

        if !self.paths && !self.tsv && !self.no_size {
            println!();
            let full_closure = GCRoot::full_closure(&roots);
            let total_size = GCRoot::full_closure_size(&roots)?;
            println!("Estimated total size: {} ({} store paths)",
                FmtSize::new(total_size).to_string().yellow(), full_closure.len());
        }

        if !self.paths && !self.tsv {
            println!();
        }

        Ok(())
    }
}
