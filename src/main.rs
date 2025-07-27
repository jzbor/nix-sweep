use std::cmp;
use std::{env, thread};

use clap::Parser;
use rayon::ThreadPoolBuilder;

use crate::commands::Command;
use crate::interaction::resolve;

mod config;
mod gc;
mod profiles;
mod store;
mod roots;
mod journal;
mod files;
mod caching;
mod fmt;
mod commands;
mod interaction;
mod ordered_channel;


const THREADS_ENV_VAR: &str = "NIX_SWEEP_NUM_THREADS";
const MAX_THREADS: usize = 4;


type HashMap<K, V> = rustc_hash::FxHashMap<K, V>;
type HashSet<V> = rustc_hash::FxHashSet<V>;

/// Utility to clean up old Nix profile generations and left-over garbage collection roots
///
/// You can adjust the number of worker threads this program uses with the `NIX_SWEEP_NUM_THREADS` env
/// variable.
#[derive(Parser)]
#[command(version, about, long_about)]
pub struct Args {
    #[clap(subcommand)]
    subcommand: Subcommand,
}

#[derive(clap::Subcommand)]
enum Subcommand {
    /// Analyze store usage
    Analyze(commands::analyze::AnalyzeCommand),

    /// Clean out old profile generations
    ///
    /// Positive criteria (e.g. --keep-min, --keep-newer) are prioritized over negative ones
    /// (e.g. --keep-max, --remove-older).
    /// Passing 0 on any cleanout criterion will reset it to the default behavior.
    ///
    /// The latest generation as well as the currently active one will not be removed.
    Cleanout(commands::cleanout::CleanoutCommand),

    /// Run garbage collection (short for `nix-store --gc`)
    GC(commands::gc::GCCommand),

    /// List garbage collection roots
    GCRoots(commands::gc_roots::GCRootsCommand),

    /// List profile generations
    Generations(commands::generations::GenerationsCommand),

    /// Show information on a path or a symlink to a path
    PathInfo(commands::path_info::PathInfoCommand),

    /// Selectively remove gc roots
    TidyupGCRoots(commands::tidyup_gc_roots::TidyupGCRootsCommand),

    /// Export manpage
    #[clap(hide(true))]
    Man(commands::man::ManCommand),
}

fn init_rayon() -> Result<(), String> {
    let nthreads: usize = match env::var(THREADS_ENV_VAR).ok() {
        Some(n) => n.parse()
            .map_err(|_| format!("Unable to parse {} environment variable", THREADS_ENV_VAR))?,
        None => match thread::available_parallelism().ok() {
            Some(avail) => cmp::min(avail.into(), MAX_THREADS),
            None => MAX_THREADS,
        },
    };

    ThreadPoolBuilder::new()
        .num_threads(nthreads)
        .build_global()
        .map_err(|e| e.to_string())
}

fn main() {
    let config = Args::parse();
    resolve(init_rayon());

    use Subcommand::*;
    let res = match config.subcommand {
        Analyze(cmd) => cmd.run(),
        Cleanout(cmd) => cmd.run(),
        GC(cmd) => cmd.run(),
        GCRoots(cmd) => cmd.run(),
        Generations(cmd) => cmd.run(),
        Man(cmd) => cmd.run(),
        PathInfo(cmd) => cmd.run(),
        TidyupGCRoots(cmd) => cmd.run(),
    };
    resolve(res);
}
