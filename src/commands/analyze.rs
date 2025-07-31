use std::cmp::{self, Reverse};
use std::path;

use colored::Colorize;
use rayon::slice::ParallelSliceMut;

use crate::files;
use crate::fmt::*;
use crate::journal::*;
use crate::profiles::Profile;
use crate::roots::GCRoot;
use crate::roots;
use crate::store::{Store, NIX_STORE};

#[derive(clap::Args)]
pub struct AnalyzeCommand {
    /// Don't analyze system journal
    #[clap(long)]
    no_journal: bool,

    /// Show all gc roots and profiles
    #[clap(short, long)]
    all: bool,

    /// Show the full path for gc roots and profiles
    #[clap(short, long)]
    full_paths: bool,

    /// Show n gc-roots and profiles
    #[clap(long, default_value_t = 5)]
    show: usize,
}

impl super::Command for AnalyzeCommand {
    fn run(self) -> Result<(), String> {
        eprintln!("Indexing store...");
        let nstore_paths = Store::all_paths()?.len();
        let (store_size_naive, store_size_hl) = rayon::join(
            Store::size_naive,
            Store::size
        );
        let store_size_naive = store_size_naive?;
        let store_size_hl = store_size_hl?;
        let store_size = cmp::min(store_size_naive, store_size_hl);


        let journal_size = if !self.no_journal && journal_exists() {
            eprintln!("Indexing system journal...");
            Some(journal_size())
        } else { None };


        eprintln!("Indexing profiles...");
        let profile_paths = GCRoot::profile_paths()?;
        let mut sorted_profiles = Vec::with_capacity(profile_paths.len());
        for path in profile_paths {
            let profile = Profile::from_path(path.clone()).ok();
            let size = profile.as_ref()
                .and_then(|p| Profile::full_closure_size(p).ok());
            sorted_profiles.push((path, profile, size));
        }
        sorted_profiles.par_sort_by_key(|(p, _, _)| p.clone());
        sorted_profiles.par_sort_by_key(|(_, _, s)| Reverse(*s));
        let drained_profiles = if !self.all {
            sorted_profiles.drain(cmp::min(self.show, sorted_profiles.len())..).count()
        } else {
            0
        };

        eprintln!("Indexing gc roots...");
        let gc_roots: Vec<_> = roots::gc_roots(false)?
            .into_iter()
            .filter(|r| !r.is_profile() && !r.is_current())
            .collect();
        let mut sorted_gc_roots = Vec::with_capacity(gc_roots.len());
        for root in gc_roots {
            let item = match root.store_path().cloned() {
                Ok(path) => (root, Some(path.closure_size())),
                Err(_) => (root, None),
            };
            sorted_gc_roots.push(item);
        }
        sorted_gc_roots.par_sort_by_key(|(p, _)| p.link().clone());
        sorted_gc_roots.par_sort_by_key(|(_, s)| Reverse(*s));
        let drained_gc_roots = if !self.all {
            sorted_gc_roots.drain(cmp::min(self.show, sorted_gc_roots.len())..).count()
        } else {
            0
        };


        eprintln!();
        println!("{}", "=> System:".green());

        print!("{:<20} {}", format!("{}:", NIX_STORE), FmtSize::new(store_size).left_pad().yellow());
        let blkdev_info = Store::blkdev()
            .and_then(|d| files::get_blkdev_size(&d).map(|s| (d, s)));
        if let Ok((dev, size)) = blkdev_info {
            let percent_str = FmtPercentage::new(store_size, size).left_pad();
            println!("\t({} of {} [{}])", percent_str, dev, size::Size::from_bytes(size));
        } else {
            println!();
        }

        if let Some(journal_size) = journal_size {
            print!("{:<20} {:>11}", format!("{}:", JOURNAL_PATH), FmtSize::new(journal_size).left_pad().yellow());

            let blkdev_info = files::blkdev_of_path(&path::PathBuf::from(JOURNAL_PATH))
                .and_then(|d| files::get_blkdev_size(&d).map(|s| (d, s)));
            if let Ok((dev, size)) = blkdev_info {
                let percent_str = FmtPercentage::new(journal_size, size).left_pad();
                println!("\t({} of {} [{}])", percent_str, dev, FmtSize::new(size));
            } else {
                println!();
            }
        }

        println!();
        println!("Number of store paths:      \t{}", nstore_paths.to_string().bright_blue());

        if store_size_naive > store_size_hl {
            println!("Hardlinking currently saves:\t{}", size::Size::from_bytes(store_size_naive - store_size_hl).to_string().green());
        }


        println!();
        println!("{}", "=> Profiles:".green());
        let max_path_len = sorted_profiles.iter()
            .map(|(p, _, _)| p.to_string_lossy().len())
            .max()
            .unwrap_or(0);
        for (path, profile, size) in sorted_profiles {
            let path = path.to_string_lossy().to_string();
            let path_str = FmtWithEllipsis::fitting_terminal(path, max_path_len, 40)
                .truncate(!self.full_paths)
                .right_pad();
            let size_str = FmtOrNA::mapped(size, FmtSize::new)
                .left_pad();
            let percentage_str = FmtOrNA::mapped(size, |s| FmtPercentage::new(s, store_size)
                .bracketed())
                .or_empty()
                .right_pad();
            let generations_str = match profile {
                Some(profile) => format!("[{} generations]", profile.generations().len()),
                None => "n/a".to_owned(),
            };

            println!("{}  {} {}    {}",
                path_str,
                size_str.yellow(),
                percentage_str,
                generations_str.bright_blue(),
            );
        }
        if drained_profiles != 0 {
            println!("...and {drained_profiles} more");
        }


        println!();
        println!("{}", "=> GC Roots:".green());

        let max_link_len = sorted_gc_roots.iter()
            .map(|(r, _)| r.link().to_string_lossy().len())
            .max()
            .unwrap_or(0);
        for (root, size) in &sorted_gc_roots {
            let link = root.link().to_string_lossy().to_string();
            let link_str = FmtWithEllipsis::fitting_terminal(link, max_link_len, 20)
                .truncate(!self.full_paths)
                .right_pad();
            let size_str = FmtOrNA::mapped(*size, FmtSize::new)
                .left_pad();
            let percentage_str = FmtOrNA::mapped(*size, |s| FmtPercentage::new(s, store_size).bracketed())
                .or_empty()
                .right_pad();

            println!("{}  {} {}",
                link_str,
                size_str.yellow(),
                percentage_str,
            );
        }
        if drained_gc_roots != 0 {
            println!("...and {drained_gc_roots} more");
        }

        println!();
        let roots: Vec<_> = sorted_gc_roots.iter().
            map(|tup| tup.0.clone())
            .collect();
        let total_size = GCRoot::full_closure_size(&roots)?;
        let size_str = FmtSize::new(total_size).to_string();
        let percentage_str = FmtPercentage::new(total_size, store_size)
            .bracketed()
            .right_pad();
        println!("Total closure size of independent GC Roots:\t{} {}", size_str.yellow(), percentage_str);

        println!();
        Ok(())
    }
}
