use std::fmt::Display;
use std::io::Write;
use std::str::FromStr;
use std::{path, process};
use colored::Colorize;

use clap::Parser;
use config::ConfigPreset;
use generations::Generation;

mod config;
mod gc;
mod generations;


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
    Cleanout {
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

        /// Profiles to clean out; valid values: system, user, home, <path>
        profiles: Vec<String>,
    },

    /// Run garbage collection (short for `nix-store --gc`)
    GC {
        /// Ask before running garbage collection
        #[clap(short, long)]
        interactive: bool,

        /// Don't actually run garbage collection
        #[clap(short, long)]
        dry_run: bool,
    }
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

fn ask(question: &str) -> Result<bool, String> {
    loop {
        print!("{} [y/n] ",question);
        let _ = std::io::stdout().flush();

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)
            .map_err(|e| format!("Unable to ask question ({})", e))?;

        match input.trim() {
            "y" | "Y" | "yes" | "Yes" | "YES" => return Ok(true),
            "n" | "N" | "no" | "No" | "NO" => return Ok(false),
            _ => continue,
        }
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

fn list_generations(generations: &[Generation], profile_type: &ProfileType) {
    announce_listing(profile_type);
    for gen in generations {
        let marker = if gen.marked() { "would remove".red() } else { "would keep".green() };
        let id_str = format!("[{}]", gen.number()).yellow();
        println!("{}\t {} days old, {}", id_str, gen.age(), marker);
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

fn run_gc(interactive: bool, dry_run: bool) -> Result<(), String> {
    if dry_run {
        println!("\n{}", "=> Skipping garbage collection (dry run)".green());
    } else {
        println!("\n{}", "=> Running garbage collection".green());
        if !interactive || ask("Do you want to perform garbage collection now?")? {
            gc::gc()?
        }
    }

    Ok(())
}

fn cleanout(preset: String, config_file: Option<path::PathBuf>, config_args: config::ConfigPreset, profiles: Vec<String>, dry_run: bool) -> Result<(), String> {
    config_args.validate()?;
    let config = ConfigPreset::load(&preset, config_file)?
        .override_with(&config_args);
    let interactive = config.interactive.is_none() || config.interactive == Some(true);

    // println!("{:#?}", config);

    for profile_str in profiles {
        let profile = ProfileType::from_str(&profile_str)?;
        let generations = get_generations(&profile, &config)?;

        if dry_run {
            list_generations(&generations, &profile);
        } else if interactive {
            list_generations(&generations, &profile);

            let confirmation = ask("Do you want to proceed?")?;
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
        run_gc(interactive, dry_run)?;
    }

    Ok(())
}

fn main() {
    let config = Args::parse();

    use Subcommand::*;
    let res = match config.subcommand {
        Cleanout { preset, config, cleanout_config, profiles, dry_run } => cleanout(preset, config, cleanout_config, profiles, dry_run),
        GC { interactive, dry_run } => run_gc(interactive, dry_run),
    };
    resolve(res);
}
