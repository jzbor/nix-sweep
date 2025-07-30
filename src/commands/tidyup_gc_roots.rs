use std::cmp::Reverse;
use std::fs;
use std::time::Duration;

use colored::Colorize;
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
use rayon::slice::ParallelSliceMut;

use crate::interaction::*;
use crate::ordered_channel::OrderedChannel;
use crate::roots;
use crate::roots::GCRoot;


#[derive(clap::Args)]
pub struct TidyupGCRootsCommand {
    /// Delete all qualifying gc roots without asking for user confirmation
    #[clap(short, long)]
    force: bool,

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

impl super::Command for TidyupGCRootsCommand {
    fn run(self) -> Result<(), String> {
        let mut roots = roots::gc_roots(self.include_missing)?;
        let print_size = !(self.no_size || self.force);

        roots.par_sort_by_key(|r| r.link().clone());
        roots.par_sort_by_key(|r| Reverse(r.age().cloned().unwrap_or(Duration::MAX)));

        roots = GCRoot::filter_roots(roots, self.include_profiles, self.include_current,
            !self.exclude_inaccessible, self.older, self.newer);
        let nroots_listed = roots.len();

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
                if !self.force {
                    root.print_fancy(closure_size, !self.no_size);
                }

                if root.store_path().is_err() {
                    if self.force {
                        warn(&format!("Cannot remove as the path is inaccessible: {}", root.link().to_string_lossy()))
                    } else {
                        ack("Cannot remove as the path is inaccessible");
                    }
                } else if self.force || ask("Remove gc root?", false) {
                    println!("-> Removing gc root '{}'", root.link().to_string_lossy());
                    if let Err(e) =  fs::remove_file(root.link()) {
                        println!("{}", format!("Error: {}", e).red());
                    }
                }
            }
        });

        if !self.force {
            println!();
        }
        Ok(())
    }
}
