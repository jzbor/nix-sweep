use std::collections::HashMap;
use std::fmt::Display;
use std::io::Write;
use std::path::Path;
use std::str::FromStr;
use std::{fs, path, process};
use colored::Colorize;

use clap::Parser;
use config::ConfigPreset;
use generations::Generation;
use rayon::prelude::*;
use roots::{gc_root_is_current, gc_root_is_profile};
use store_paths::StorePath;

mod config;
mod gc;
mod generations;
mod store_paths;
mod roots;


#[derive(Clone, Debug)]
enum ProfileType {
    User(),
    Home(),
    System(),
    Custom(path::PathBuf),
}

#[derive(Parser, Debug)]
#[command(version, about, long_about)]
pub struct Args {
    #[clap(subcommand)]
    subcommand: Subcommand,
}

#[derive(Clone, Debug, clap::Subcommand)]
enum Subcommand {
    /// Clean out old profiles
    Cleanout(CleanoutArgs),

    /// Selectively remove gc roots
    RemoveGCRoots(RemoveGCRootsArgs),

    /// Run garbage collection (short for `nix-store --gc`)
    GC(GCArgs),

    /// Print out gc roots
    GCRoots(GCRootsArgs),
}

#[derive(Clone, Debug, clap::Args)]
struct CleanoutArgs {
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

    /// Profiles to clean out; valid values: system, user, home, <path>
    profiles: Vec<String>,
}

#[derive(Clone, Debug, clap::Args)]
struct GCArgs {
    /// Ask before running garbage collection
    #[clap(short('n'), long("non-interactive"), action = clap::ArgAction::SetFalse)]  // this is very confusing, but works
    interactive: bool,

    /// Do not ask before running garbage collection
    #[clap(short('i'), long("interactive"), overrides_with = "interactive")]
    _non_interactive: bool,

    /// Don't actually run garbage collection
    #[clap(short, long)]
    dry_run: bool,
}

#[derive(Clone, Debug, clap::Args)]
struct GCRootsArgs {
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

    /// Do not calculate the size of generations
    #[clap(long)]
    no_size: bool,
}

#[derive(Clone, Debug, clap::Args)]
struct RemoveGCRootsArgs {
    /// Include profiles
    #[clap(short('p'), long)]
    include_profiles: bool,

    /// Include current
    #[clap(short('c'), long)]
    include_current: bool,

    /// Include gc roots that are referenced, but could not be found
    #[clap(long)]
    include_missing: bool,

    /// Do not calculate the size of generations
    #[clap(long)]
    no_size: bool,
}

impl FromStr for ProfileType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use ProfileType::*;
        match s {
            "user" => Ok(User()),
            "home" => Ok(Home()),
            "system" => Ok(System()),
            other => {
                let path = path::PathBuf::from_str(other)
                    .map_err(|e| e.to_string())?;
                Ok(Custom(path))
            },
        }
    }
}


fn resolve<T, E: Display>(result: Result<T, E>) -> T {
    match result {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(1)
        },
    }
}

fn mark(mut generations: Vec<Generation>, config: &config::ConfigPreset) -> Vec<Generation>{
    // negative criteria are applied first

    // mark older generations
    if let Some(older_days) = config.remove_older {
        for generation in generations.iter_mut() {
            if generation.age() >= older_days {
                generation.mark();
            }
        }
    }

    // mark superfluous generations
    if let Some(max) = config.keep_max {
        for (i, generation) in generations.iter_mut().rev().enumerate() {
            if i >= max {
                generation.mark();
            }
        }
    }

    // unmark newer generations
    if let Some(newer_days) = config.keep_newer {
        for generation in generations.iter_mut() {
            if generation.age() < newer_days {
                generation.unmark();
            }
        }
    }

    // unmark kept generations
    if let Some(min) = config.keep_min {
        for (i, generation) in generations.iter_mut().rev().enumerate() {
            if i < min {
                generation.unmark();
            }
        }
    }

    generations
}

fn ask(question: &str, default: bool) -> bool {
    loop {
        match default {
            true => print!("{} [Y/n] ",question),
            false => print!("{} [y/N] ",question),
        }
        let _ = std::io::stdout().flush();

        let mut input = String::new();
        match std::io::stdin().read_line(&mut input) {
            Ok(_) => (),
            Err(_) => continue,
        };

        match input.trim() {
            "y" | "Y" | "yes" | "Yes" | "YES" => return true,
            "n" | "N" | "no" | "No" | "NO" => return false,
            "" => return default,
            _ => continue,
        }
    }
}

fn ack(question: &str) {
    loop {
        print!("{} [enter] ",question);
        let _ = std::io::stdout().flush();

        let mut input = String::new();
        match std::io::stdin().read_line(&mut input) {
            Ok(_) => (),
            Err(_) => continue,
        };
        return;
    }
}

fn fancy_print_generation(generation: &Generation, print_marker: bool, print_size: bool, added_size_lookup: Option<&HashMap<StorePath, usize>>) {
    let marker = if generation.marked() { "would remove".red() } else { "would keep".green() };
    let id_str = format!("[{}]", generation.number()).bright_blue();

    print!("{}\t {} days old", id_str, generation.age());

    if print_marker {
        print!(", {}", marker);
    }

    if print_size {
        if let Ok(path) = generation.store_path() {
            let closure_size = size::Size::from_bytes(path.closure_size());
            let size = if let Some(occurences) = added_size_lookup {
                let added_size = size::Size::from_bytes(path.added_closure_size(occurences));
                format!("[{} / {}]", closure_size, added_size).yellow()
            } else {
                format!("[{}]", closure_size).yellow()
            };
            print!(" \t{}", size);
        }
    }
    println!();
}

fn fancy_print_gc_root(link: &Path, store_path_result: &Result<StorePath, String>, print_size: bool, added_size_lookup: Option<&HashMap<StorePath, usize>>) {
    let is_profile = gc_root_is_profile(link);
    let is_current = gc_root_is_current(link);
    let attributes = match (is_profile, is_current) {
        (true, true) => "(profile, current)",
        (true, false) => "(profile)",
        (false, true) => "(current)",
        (false, false) => "",
    };

    if let Ok(store_path) = store_path_result {
        let size = if print_size {
            let closure_size = size::Size::from_bytes(store_path.closure_size());

            if let Some(occurences) = added_size_lookup {
                let added_size = size::Size::from_bytes(store_path.added_closure_size(occurences));
                format!(" [{} / {}]", closure_size, added_size).yellow()
            } else {
                format!(" [{}]", closure_size).yellow()
            }
        } else {
            "".to_owned().into()
        };

        println!("{}{} {}", link.to_string_lossy(), size, attributes.blue());
        println!("{}", format!("  -> {}", store_path.path().to_string_lossy()).bright_black());
    } else {
        let size = if print_size { " [???]".yellow() } else { "".to_owned().into() };
        println!("{}{} {}", link.to_string_lossy(), size, attributes.blue());
        println!("{}", "  -> <not accessible>".to_string().bright_black());
    }

}

fn announce_listing(profile_type: &ProfileType) {
    use ProfileType::*;
    match profile_type {
        User() => println!("{}", "=> Listing user profile generations".to_string().green()),
        Home() => println!("{}", "=> Listing home-manager generations".to_string().green()),
        System() => println!("{}", "=> Listing system generations".to_string().green()),
        Custom(path) => println!("{}", format!("=> Listing generations for profile {}", path.to_string_lossy()).to_string().green()),
    }
}

fn announce_removal(profile_type: &ProfileType) {
    use ProfileType::*;
    match profile_type {
        User() => println!("\n{}", "=> Removing old user profile generations".to_string().green()),
        Home() => println!("\n{}", "=> Removing old home-manager generations".to_string().green()),
        System() => println!("\n{}", "=> Removing old system generations".to_string().green()),
        Custom(path) => println!("\n{}", format!("=> Removing old generations for profile {}", path.to_string_lossy()).to_string().green()),
    }
}

fn list_generations(generations: &[Generation], profile_type: &ProfileType, print_size: bool) {
    announce_listing(profile_type);

    let added_size_lookup = generations.par_iter()
        .flat_map(|g| g.store_path())
        .flat_map(|p| p.closure())
        .flatten()
        .fold(HashMap::new, |mut acc, v| {
            if let Some(existing) = acc.get_mut(&v) {
                *existing += 1;
            } else {
                acc.insert(v.clone(), 1);
            }
            acc
        })
        .reduce_with(|mut m1, m2| {
            for (k, v) in m2 {
                *m1.entry(k).or_default() += v;
            }
            m1
        }).unwrap_or(HashMap::new());


    for gen in generations {
        fancy_print_generation(gen, true, print_size, Some(&added_size_lookup));
    }
    println!();
}

fn remove_generations(generations: &[Generation], profile_type: &ProfileType) {
    announce_removal(profile_type);
    for gen in generations {
        let age_str = if gen.age() == 1 { "1 day old".to_owned() } else { format!("{} days old", gen.age()) };
        if gen.marked() {
            println!("{}", format!("-> Removing generation {} ({})", gen.number(), age_str).bright_blue());
            resolve(gen.remove());
        } else {
            println!("{}", format!("-> Keeping generation {} ({})", gen.number(), age_str).bright_black());
        }
    }
    println!();
}

fn get_generations(profile_type: &ProfileType, config: &config::ConfigPreset) -> Result<Vec<Generation>, String> {
    use ProfileType::*;
    match profile_type {
        Home() => generations::home_generations(),
        User() => generations::user_generations(),
        System() => generations::system_generations(),
        Custom(path) => generations::generations_from_path(path),
    }.map(|gens| mark(gens, config))
}

fn run_gc(args: GCArgs) -> Result<(), String> {
    if args.dry_run {
        println!("\n{}", "=> Skipping garbage collection (dry run)".green());
    } else {
        println!("\n{}", "=> Running garbage collection".green());
        if !args.interactive || ask("Do you want to perform garbage collection now?", false) {
            gc::gc()?
        }
    }

    Ok(())
}

fn cleanout(args: CleanoutArgs) -> Result<(), String> {
    args.cleanout_config.validate()?;
    let config = ConfigPreset::load(&args.preset, args.config)?
        .override_with(&args.cleanout_config);
    let interactive = config.interactive.is_none() || config.interactive == Some(true);

    // println!("{:#?}", config);

    for profile_str in args.profiles {
        let profile = ProfileType::from_str(&profile_str)?;
        let generations = get_generations(&profile, &config)?;

        if args.dry_run {
            list_generations(&generations, &profile, !args.no_size);
        } else if interactive {
            list_generations(&generations, &profile, !args.no_size);

            let confirmation = ask("Do you want to proceed?", false);
            if confirmation {
            remove_generations(&generations, &profile);
            } else {
                println!("-> Not touching profile\n");
            }
        } else {
            remove_generations(&generations, &profile);
        }
    }

    if config.gc == Some(true) {
        let gc_args = GCArgs { interactive, _non_interactive: !interactive, dry_run: args.dry_run };
        run_gc(gc_args)?;
    }

    Ok(())
}

fn list_gc_roots(args: GCRootsArgs) -> Result<(), String> {
    let roots = roots::gc_roots(args.include_missing)?;
    let added_size_lookup = roots::count_gc_deps(&roots);

    let mut sorted_roots: Vec<_> = roots.into_iter().collect();
    sorted_roots.sort_by_key(|e| e.0.clone());

    for (link, result) in sorted_roots {
        if !args.include_profiles && gc_root_is_profile(&link) {
            continue
        }
        if !args.include_current && gc_root_is_current(&link) {
            continue
        }

        if args.paths {
            println!("{}", link.to_string_lossy());
        } else if args.tsv {
            let path = result.as_ref().map(|p| p.path().to_string_lossy().to_string())
                .unwrap_or(String::from("na"));
            println!("{}\t{}", link.to_string_lossy(), path);
        } else {
            fancy_print_gc_root(&link, &result, !args.no_size, Some(&added_size_lookup));
            println!()
        }
    }

    Ok(())
}

fn remove_gc_roots(args: RemoveGCRootsArgs) -> Result<(), String> {
    let roots = roots::gc_roots(args.include_missing)?;
    let added_size_lookup = roots::count_gc_deps(&roots);

    let mut sorted_roots: Vec<_> = roots.into_iter().collect();
    sorted_roots.sort_by_key(|e| e.0.clone());

    for (link, result) in sorted_roots {
        if !args.include_profiles && gc_root_is_profile(&link) {
            continue
        }
        if !args.include_current && gc_root_is_current(&link) {
            continue
        }

        fancy_print_gc_root(&link, &result, args.no_size, Some(&added_size_lookup));

        if result.is_err() {
            ack("Cannot remove as the path is inaccessible");
        } else if ask("Remove gc root?", false) {
            println!("-> Removing gc root '{}'", link.to_string_lossy());
            if let Err(e) =  fs::remove_file(&link) {
                println!("{}", format!("Error: {}", e).red());
            }
        }
        println!();
    }

    Ok(())
}

fn main() {
    let config = Args::parse();

    use Subcommand::*;
    let res = match config.subcommand {
        Cleanout(args) => cleanout(args),
        GC(args) => run_gc(args),
        GCRoots(args) => list_gc_roots(args),
        RemoveGCRoots(args) => remove_gc_roots(args),
    };
    resolve(res);
}
