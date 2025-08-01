use std::cmp::{self, Reverse};
use std::path::PathBuf;

use colored::Colorize;
use rayon::slice::ParallelSliceMut;

use crate::utils::{files, journal};
use crate::utils::fmt::*;
use crate::utils::interaction::{announce, resolve};
use crate::utils::journal::*;
use crate::nix::profiles::Profile;
use crate::nix::roots::GCRoot;
use crate::nix::store::{Store, StorePath, NIX_STORE};


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

struct StoreAnalysis {
    nstore_paths: usize,
    ndrv_paths: usize,
    ndrv_closure: usize,
    store_size_naive: u64,
    store_size_hl: u64,
    drv_size: u64,
    drv_closure_size: u64,
    journal_size: Option<u64>,
    blkdev_info: Option<(String, u64)>,
}

struct ProfileAnalysis {
    profiles: Vec<(PathBuf, Option<Profile>, Option<u64>)>,
    drained: usize,
}

struct GCRootsAnalysis {
    gc_roots: Vec<(GCRoot, Option<u64>)>,
    drained: usize,
}



impl StoreAnalysis {
    fn create(journal: bool) -> Result<Self, String> {
        let store_paths = Store::all_paths()?;
        let nstore_paths = store_paths.len();
        let drv_paths: Vec<_> = store_paths.into_iter().filter(StorePath::is_drv).collect();
        let ndrv_paths = drv_paths.len();

        let mut store_size_naive = 0;
        let mut store_size_hl = 0;
        let mut drv_size = 0;
        let mut drv_closure_size = 0;
        let mut journal_size = None;
        let mut ndrv_closure = 0;

        rayon::scope(|s| {
            s.spawn(|_| {
                store_size_naive = resolve(Store::size_naive());
            });

            s.spawn(|_| {
                store_size_hl = resolve(Store::size());
            });

            s.spawn(|_| {
                if journal && journal_exists() {
                    journal_size = Some(journal::journal_size());
                }
            });

            s.spawn(|_| {
                let paths: Vec<_> = drv_paths.iter().map(|sp| sp.path().clone()).collect();
                drv_size = files::dir_size_considering_hardlinks_all(&paths);
            });

            s.spawn(|_| {
                let drv_closure: Vec<_> = StorePath::full_closure(&drv_paths).into_iter().collect();
                ndrv_closure = drv_closure.len();
                let paths: Vec<_> = drv_closure.iter().map(|sp| sp.path().clone()).collect();
                drv_closure_size = files::dir_size_considering_hardlinks_all(&paths)
            });
        });

        let blkdev_info = Store::blkdev()
            .and_then(|d| files::get_blkdev_size(&d).map(|s| (d, s)))
            .ok();

        Ok(StoreAnalysis {
            nstore_paths, store_size_naive, store_size_hl,
            ndrv_paths, ndrv_closure, drv_size, drv_closure_size,
            blkdev_info, journal_size,
        })
    }

    fn store_size(&self) -> u64 {
        cmp::min(self.store_size_naive, self.store_size_hl)
    }

    fn report(&self) -> Result<(), String> {
        announce("System:".to_owned());

        print!("{:<20} {}", format!("{}:", NIX_STORE), FmtSize::new(self.store_size()).left_pad().yellow());
        if let Some((dev, dev_size)) = &self.blkdev_info {
            let percent_str = FmtPercentage::new(self.store_size(), *dev_size).left_pad();
            println!("\t({} of {} [{}])", percent_str, dev, size::Size::from_bytes(*dev_size));
        } else {
            println!();
        }

        if let Some(journal_size) = self.journal_size {
            print!("{:<20} {:>11}", format!("{}:", JOURNAL_PATH), FmtSize::new(journal_size).left_pad().yellow());

            if let Some((dev, size)) = &self.blkdev_info {
                let percent_str = FmtPercentage::new(journal_size, *size).left_pad();
                println!("\t({} of {} [{}])", percent_str, dev, FmtSize::new(*size));
            } else {
                println!();
            }
        }

        println!();
        println!("Number of store paths:           \t{}", self.nstore_paths.to_string().bright_blue());
        println!("Derivation (.drv) files in store:\t{}\t{} {}",
            self.ndrv_paths.to_string().cyan(),
            FmtSize::new(self.drv_size).left_pad().cyan(),
            FmtPercentage::new(self.drv_size, self.store_size_hl).bracketed().right_pad(),
        );
        println!("Closure of .drv files in store:  \t{}\t{} {}",
            self.ndrv_closure.to_string().bright_cyan(),
            FmtSize::new(self.drv_closure_size).left_pad().bright_cyan(),
            FmtPercentage::new(self.drv_closure_size, self.store_size_hl).bracketed().right_pad(),
        );

        if self.store_size_naive > self.store_size_hl {
            println!("Hardlinking currently saves:    \t{}", size::Size::from_bytes(self.store_size_naive - self.store_size_hl).to_string().green());
        }

        Ok(())
    }
}

impl ProfileAnalysis {
    fn create(all: bool, show: usize) -> Result<Self, String> {
        let profile_paths = GCRoot::profile_paths()?;

        let mut profiles = Vec::with_capacity(profile_paths.len());
        for path in profile_paths {
            let profile = Profile::from_path(path.clone()).ok();
            let size = profile.as_ref()
                .and_then(|p| Profile::full_closure_size(p).ok());
            profiles.push((path, profile, size));
        }

        profiles.par_sort_by_key(|(p, _, _)| p.clone());
        profiles.par_sort_by_key(|(_, _, s)| Reverse(*s));

        let drained = if !all {
            profiles.drain(cmp::min(show, profiles.len())..).count()
        } else {
            0
        };

        Ok(ProfileAnalysis { profiles, drained })
    }

    fn report(&self, full_paths: bool, store_size: u64) -> Result<(), String> {
        announce("Profiles:".to_owned());

        let max_path_len = self.profiles.iter()
            .map(|(p, _, _)| p.to_string_lossy().len())
            .max()
            .unwrap_or(0);

        for (path, profile, size) in &self.profiles {
            let path = path.to_string_lossy().to_string();
            let path_str = FmtWithEllipsis::fitting_terminal(path, max_path_len, 40)
                .truncate_if(!full_paths)
                .right_pad();
            let size_str = FmtOrNA::mapped(*size, FmtSize::new)
                .left_pad();
            let percentage_str = FmtOrNA::mapped(*size, |s| FmtPercentage::new(s, store_size)
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

        if self.drained != 0 {
            println!("...and {} more", self.drained);
        }

        Ok(())
    }
}

impl GCRootsAnalysis {
    fn create(all: bool, show: usize) -> Result<Self, String> {
        let mut gc_roots: Vec<_> = GCRoot::all(false, false, false)?
            .into_iter()
            .filter(|r| r.is_independent())
            .map(|r| match r.store_path().cloned() {
                Ok(path) => (r, Some(path.closure_size())),
                Err(_) => (r, None),
            })
            .collect();

        gc_roots.par_sort_by_key(|(p, _)| p.link().clone());
        gc_roots.par_sort_by_key(|(_, s)| Reverse(*s));

        let drained = if !all {
            gc_roots.drain(cmp::min(show, gc_roots.len())..).count()
        } else {
            0
        };

        Ok(GCRootsAnalysis { gc_roots, drained })
    }

    fn report(&self, full_paths: bool, store_size: u64) -> Result<(), String> {
        announce("GC Roots:".to_owned());

        let max_link_len = self.gc_roots.iter()
            .map(|(r, _)| r.link().to_string_lossy().len())
            .max()
            .unwrap_or(0);
        for (root, size) in &self.gc_roots {
            let link = root.link().to_string_lossy().to_string();
            let link_str = FmtWithEllipsis::fitting_terminal(link, max_link_len, 20)
                .truncate_if(!full_paths)
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
        if self.drained != 0 {
            println!("...and {} more", self.drained);
        }

        println!();
        let roots: Vec<_> = self.gc_roots.iter()
            .map(|tup| tup.0.clone())
            .collect();
        let total_size = GCRoot::full_closure_size(&roots)?;
        let size_str = FmtSize::new(total_size).to_string();
        let percentage_str = FmtPercentage::new(total_size, store_size)
            .bracketed()
            .right_pad();
        println!("Total closure size of independent GC Roots:\t{} {}", size_str.yellow(), percentage_str);

        Ok(())
    }
}


impl super::Command for AnalyzeCommand {
    fn run(self) -> Result<(), String> {
        let mut store_analysis = Err("Store indexing not completed yet".to_owned());
        let mut profile_analysis = Err("Profile indexing not completed yet".to_owned());
        let mut gc_roots_analysis = Err("Gc roots indexing not completed yet".to_owned());

        eprintln!("Indexing store, profiles and gc roots...");
        rayon::scope(|s| {
            s.spawn(|_| {
                store_analysis = StoreAnalysis::create(!self.no_journal);
                eprintln!("Finished store indexing");
            });

            s.spawn(|_| {
                profile_analysis = ProfileAnalysis::create(self.all, self.show);
                eprintln!("Finished profile indexing");
            });

            s.spawn(|_| {
                gc_roots_analysis = GCRootsAnalysis::create(self.all, self.show);
                eprintln!("Finished gc roots indexing");
            });
        });

        let store_analysis = store_analysis?;
        let profile_analysis = profile_analysis?;
        let gc_roots_analysis = gc_roots_analysis?;


        store_analysis.report()?;
        profile_analysis.report(self.full_paths, store_analysis.store_size())?;
        gc_roots_analysis.report(self.full_paths, store_analysis.store_size())?;

        println!();
        Ok(())
    }
}
