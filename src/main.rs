use std::fmt::Display;
use std::io::Write;
use std::process;
use colored::Colorize;

use clap::Parser;
use generations::Generation;

mod config;
mod gc;
mod generations;

#[derive(Clone, Copy, Debug)]
enum ProfileType {
    User,
    Home,
    System,
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

fn mark(mut generations: Vec<Generation>, config: &config::Config) -> Vec<Generation>{
    // mark older generations
    for generation in generations.iter_mut() {
        if generation.age() > config.older {
            generation.mark();
        }
    }

    // limit to max generations
    if let Some(max) = config.max {
        for (i, generation) in generations.iter_mut().rev().enumerate() {
            if i >= max {
                generation.mark();
            }
        }
    }

    // unmark kept generations
    for (i, generation) in generations.iter_mut().rev().enumerate() {
        if i < config.keep {
            generation.unmark();
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

fn announce_listing(profile_type: ProfileType) {
    use ProfileType::*;
    match profile_type {
        User => println!("{}", "=> Listing user profile generations".to_string().green()),
        Home => println!("{}", "=> Listing home-manager generations".to_string().green()),
        System => println!("{}", "=> Listing system generations".to_string().green()),
    }
}

fn announce_removal(profile_type: ProfileType) {
    use ProfileType::*;
    match profile_type {
        User => println!("{}", "=> Removing old user profile generations".to_string().green()),
        Home => println!("{}", "=> Removing old home-manager generations".to_string().green()),
        System => println!("{}", "=> Removing old system generations".to_string().green()),
    }
}

fn list_generations(generations: &[Generation], profile_type: ProfileType) {
    announce_listing(profile_type);
    for gen in generations {
        let marker = if gen.marked() { "would remove".red() } else { "would keep".green() };
        let id_str = format!("[{}]", gen.number()).yellow();
        println!("{}\t {} days old, {}", id_str, gen.age(), marker);
    }
    println!();
}

fn remove_generations(generations: &[Generation], profile_type: ProfileType) {
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

fn get_generations(profile_type: ProfileType, config: &config::Config) -> Result<Vec<Generation>, String> {
    use ProfileType::*;
    match profile_type {
        Home => generations::home_generations(),
        User => generations::user_generations(),
        System => generations::system_generations(),
    }.map(|gens| mark(gens, config))
}

fn main() {
    let config = config::Config::parse();
    let mut profile_types = Vec::new();

    if config.home {
        profile_types.push(ProfileType::Home);
    }
    if config.user {
        profile_types.push(ProfileType::User);
    }
    if config.system {
        profile_types.push(ProfileType::System);
    }
    if profile_types.is_empty() {
        use ProfileType::*;
        profile_types = vec![User];
    }


    let no_action_given = !config.list && !config.rm && !config.gc && !config.interactive;
    let interactive = config.interactive || no_action_given;
    let gc = config.gc;

    if config.list {
        // list generations
        for profile_type in profile_types {
            let generations = resolve(get_generations(profile_type, &config));
            list_generations(&generations, profile_type);
        }
        return;
    }

    if interactive {
        for profile_type in profile_types {
            let generations = resolve(get_generations(profile_type, &config));
            list_generations(&generations, profile_type);

            let confirmation = resolve(ask("Do you want to proceed?"));
            println!();
            if !confirmation {
                println!("-> Not touching anything");
                process::exit(1);
            }

            remove_generations(&generations, profile_type);
        }
    } else if config.rm {
        for profile_type in profile_types {
            let generations = resolve(get_generations(profile_type, &config));
            remove_generations(&generations, profile_type);
        }
    }

    if gc {
        println!();
        println!("{}", "=> Running garbage collection".green());
        resolve(gc::gc());
    }
}
