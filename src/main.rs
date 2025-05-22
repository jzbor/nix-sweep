use std::collections::HashMap;
use std::fmt::Display;
use std::io::Write;
use std::str::FromStr;
use std::time::Duration;
use std::{fs, path, process};
use colored::Colorize;

use clap::{CommandFactory, Parser};
use config::ConfigPreset;
use profiles::{Generation, Profile};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use roots::GCRoot;
use store_paths::StorePath;

mod config;
mod gc;
mod profiles;
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
    ///
    /// Positive criteria (e.g. --keep-min, --keep-newer) are prioritized over negative ones
    /// (e.g. --keep-max, --remove-older).
    /// Passing 0 on any cleanout criterion will reset it to the default behavior.
    Cleanout(CleanoutArgs),

    /// Run garbage collection (short for `nix-store --gc`)
    GC(GCArgs),

    /// List garbage collection roots
    GCRoots(GCRootsArgs),

    /// List profile generations
    Generations(GenerationsArgs),

    /// Generate a TOML preset config to use with `nix-sweep cleanout`
    #[clap(hide(true))]
    GeneratePreset(GeneratePresetArgs),

    /// Selectively remove gc roots
    RemoveGCRoots(RemoveGCRootsArgs),

    /// Export manpage
    #[clap(hide(true))]
    Man(ManArgs),
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

    /// Profiles to clean out; valid values: system, user, home, <path_to_profile>
    #[clap(required = true)]
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
struct GenerationsArgs {
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

    /// Do not calculate the size of generations
    #[clap(long)]
    no_size: bool,
}

#[derive(Clone, Debug, clap::Args)]
struct GeneratePresetArgs {
    /// Name of the preset that is generated
    #[clap(short, long, default_value_t = config::DEFAULT_PRESET.to_owned())]
    preset: String,

    #[clap(flatten)]
    cleanout_config: config::ConfigPreset,
}

#[derive(Clone, Debug, clap::Args)]
struct ManArgs {
    directory: path::PathBuf,
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

fn format_duration(d: &Duration) -> String {
    let seconds = d.as_secs();
    let minutes = seconds / 60;
    let hours = minutes / 60;
    let days = hours / 24;
    let weeks = days / 7;
    let years = days / 365;

    if minutes < 1 {
        format!("{} sec", seconds)
    } else if hours < 1 {
        format!("{} min", minutes)
    } else if days < 1 {
        if hours == 1 {
            String::from("1 hour")
        } else {
            format!("{} hours", hours)
        }
    } else if years < 1 {
        if days == 1 {
            String::from("1 day")
        } else {
            format!("{} days", days)
        }
    } else if years < 3 {
        if weeks == 1 {
            String::from("1 week")
        } else {
            format!("{} weeks", weeks)
        }
    } else {
        if years == 1 {
            String::from("1 year")
        } else {
            format!("{} years", years)
        }
    }
}

fn fancy_print_generation(generation: &Generation, print_marker: bool, print_size: bool, added_size_lookup: Option<&HashMap<StorePath, usize>>) {
    let marker = if generation.marked() { "would remove".red() } else { "would keep".green() };
    let id_str = format!("[{}]", generation.number()).bright_blue();

    print!("{}\t {} old", id_str, format_duration(&generation.age()));

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

fn fancy_print_gc_root(root: &GCRoot, print_size: bool) {
    let attributes = match (root.is_profile(), root.is_current()) {
        (true, true) => "(profile, current)",
        (true, false) => "(profile)",
        (false, true) => "(current)",
        (false, false) => "(other)",
    };

    let age = root.age()
        .ok()
        .map(format_duration);

    let (store_path, size) = if let Ok(store_path) = root.store_path() {
        let store_path_str = store_path.path().to_string_lossy().into();
        if print_size {
            let closure_size = size::Size::from_bytes(store_path.closure_size());
            (store_path_str, Some(closure_size))
        } else {
            (store_path_str, None)
        }
    } else {
        (String::from("<not accessible>"), None)
    };

    // let size = if size != "".into() { format!(" {}", size).into() } else { size };

    println!("\n{}", root.link().to_string_lossy());
    println!("{}", format!("  -> {}", store_path).bright_black());
    print!("  ");
    if let Some(age) = age {
        print!("age: {}, ", age.bright_blue());
    }
    if print_size {
        match size {
            Some(size) => print!("size: {}, ", size.to_string().yellow()),
            None => print!("size: {}, ", "n/a".to_string().yellow()),
        }
    }
    println!("type: {}", attributes.blue());
}

fn announce_listing(profile: &Profile) {
    println!("{}", format!("=> Listing generations for profile {}", profile.path().to_string_lossy()).to_string().green());
}

fn announce_removal(profile: &Profile) {
    format!("=> Removing old generations for profile {}", profile.path().to_string_lossy()).to_string().green();
}

fn list_generations(profile: &Profile, print_size: bool, print_markers: bool) {
    announce_listing(profile);

    let store_paths: Vec<_> = profile.generations().iter()
        .flat_map(|g| g.store_path())
        .collect();
    let added_size_lookup = store_paths::count_closure_paths(&store_paths);

    for gen in profile.generations() {
        fancy_print_generation(gen, print_markers, print_size, Some(&added_size_lookup));
    }

    if print_size {
        let mut paths: Vec<_> = store_paths.par_iter()
            .flat_map(|sp| sp.closure())
            .flatten()
            .collect();
        let mut kept_paths: Vec<_> = profile.generations().par_iter()
            .filter(|g| !g.marked())
            .flat_map(|g| g.store_path())
            .flat_map(|sp| sp.closure())
            .flatten()
            .collect();

        paths.sort_by_key(|p| p.path().clone());
        paths.dedup_by_key(|p| p.path().clone());

        kept_paths.sort_by_key(|p| p.path().clone());
        kept_paths.dedup_by_key(|p| p.path().clone());

        let size: u64 = paths.iter()
            .map(|c| c.size())
            .sum();

        let kept_size: u64 = kept_paths.iter()
            .map(|c| c.size())
            .sum();

        println!();
        println!("Estimated total size: {} ({} store paths)",
            size::Size::from_bytes(size).to_string().yellow(), paths.len());
        if print_markers {
            println!("  -> after removal:   {} ({} store paths)",
                size::Size::from_bytes(kept_size).to_string().green(), kept_paths.len());
        }
    }

    println!();
}

fn remove_generations(profile: &Profile) {
    announce_removal(profile);
    for gen in profile.generations() {
        let age_str = format_duration(&gen.age());
        if gen.marked() {
            println!("{}", format!("-> Removing generation {} ({} old)", gen.number(), age_str).bright_blue());
            resolve(gen.remove());
        } else {
            println!("{}", format!("-> Keeping generation {} ({} old)", gen.number(), age_str).bright_black());
        }
    }
    println!();
}

fn get_profile(profile_type: &ProfileType) -> Result<Profile, String> {
    use ProfileType::*;
    match profile_type {
        Home() => Profile::home(),
        User() => Profile::user(),
        System() => Profile::system(),
        Custom(path) => Profile::from_path(path.to_owned()),
    }
}

fn cmd_run_gc(args: GCArgs) -> Result<(), String> {
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

fn cmd_cleanout(args: CleanoutArgs) -> Result<(), String> {
    args.cleanout_config.validate()?;
    let config = ConfigPreset::load(&args.preset, args.config)?
        .override_with(&args.cleanout_config);
    let interactive = config.interactive.is_none() || config.interactive == Some(true);

    // println!("{:#?}", config);

    for profile_str in args.profiles {
        let mut profile = get_profile(&ProfileType::from_str(&profile_str)?)?;
        profile.mark(&config);

        if args.dry_run {
            list_generations(&profile, !args.no_size, true);
        } else if interactive {
            list_generations(&profile, !args.no_size, true);

            let confirmation = ask("Do you want to delete the marked generations?", false);
            if confirmation {
            remove_generations(&profile);
            } else {
                println!("-> Not touching profile\n");
            }
        } else {
            remove_generations(&profile);
        }
    }

    if config.gc == Some(true) {
        let gc_args = GCArgs { interactive, _non_interactive: !interactive, dry_run: args.dry_run };
        cmd_run_gc(gc_args)?;
    }

    Ok(())
}

fn cmd_gc_roots(args: GCRootsArgs) -> Result<(), String> {
    let mut roots = roots::gc_roots(args.include_missing)?;

    roots.sort_by_key(|r| r.link().clone());

    for root in roots {
        if !args.include_profiles && root.is_profile() {
            continue
        }
        if !args.include_current && root.is_current() {
            continue
        }

        if args.paths {
            println!("{}", root.link().to_string_lossy());
        } else if args.tsv {
            let path = root.store_path().as_ref().map(|p| p.path().to_string_lossy().to_string())
                .unwrap_or(String::from("n/a"));
            println!("{}\t{}", root.link().to_string_lossy(), path);
        } else {
            fancy_print_gc_root(&root, !args.no_size);
        }
    }

    if !args.paths && !args.tsv {
        println!();
    }
    Ok(())
}

fn cmd_remove_gc_roots(args: RemoveGCRootsArgs) -> Result<(), String> {
    let roots = roots::gc_roots(args.include_missing)?;

    let mut roots: Vec<_> = roots.into_iter().collect();
    roots.sort_by_key(|r| r.link().clone());

    for root in roots {
        if !args.include_profiles && root.is_profile() {
            continue
        }
        if !args.include_current && root.is_current() {
            continue
        }

        if !args.force {
            fancy_print_gc_root(&root, !args.no_size);
        }

        if root.store_path().is_err() {
            ack("Cannot remove as the path is inaccessible");
        } else if args.force || ask("Remove gc root?", false) {
            println!("-> Removing gc root '{}'", root.link().to_string_lossy());
            if let Err(e) =  fs::remove_file(root.link()) {
                println!("{}", format!("Error: {}", e).red());
            }
        }
    }

    if !args.force {
        println!();
    }
    Ok(())
}

fn cmd_generations(args: GenerationsArgs) -> Result<(), String> {
    for profile_str in args.profiles {
        let profile = get_profile(&ProfileType::from_str(&profile_str)?)?;

        if args.paths {
            for gen in profile.generations() {
                println!("{}", gen.path().to_string_lossy());
            }
        } else {
            list_generations(&profile, !args.no_size, false);
        }
    }

    Ok(())
}

fn cmd_generate_preset(args: GeneratePresetArgs) -> Result<(), String> {
    let mut presets = HashMap::new();
    presets.insert(args.preset, args.cleanout_config);
    let s = toml::to_string(&presets)
        .map_err(|e| e.to_string())?;
    println!("{}", s);
    Ok(())
}

fn cmd_man(args: ManArgs) -> Result<(), String> {
    // export main
    let man = clap_mangen::Man::new(Args::command());
    let mut buffer: Vec<u8> = Default::default();
    man.render(&mut buffer)
        .map_err(|e| e.to_string())?;
    let file = args.directory.join("nix-sweep.1");
    fs::write(&file, buffer)
        .map_err(|e| e.to_string())?;
    println!("Written {}", file.to_string_lossy());

    for subcommand in Args::command().get_subcommands() {
        let man = clap_mangen::Man::new(subcommand.clone());
        let mut buffer: Vec<u8> = Default::default();
        man.render(&mut buffer)
            .map_err(|e| e.to_string())?;
        let file = args.directory.join(format!("nix-sweep-{}.1", subcommand));
        fs::write(&file, buffer)
            .map_err(|e| e.to_string())?;
        println!("Written {}", file.to_string_lossy());
    }

    Ok(())
}

fn main() {
    let config = Args::parse();

    use Subcommand::*;
    let res = match config.subcommand {
        Cleanout(args) => cmd_cleanout(args),
        GC(args) => cmd_run_gc(args),
        GCRoots(args) => cmd_gc_roots(args),
        RemoveGCRoots(args) => cmd_remove_gc_roots(args),
        Generations(args) => cmd_generations(args),
        GeneratePreset(args) => cmd_generate_preset(args),
        Man(args) => cmd_man(args),
    };
    resolve(res);
}
