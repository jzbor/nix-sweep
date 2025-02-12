use std::fmt::Display;
use std::io::Write;
use std::process;
use colored::Colorize;

use clap::Parser;
use generations::Generation;

mod config;
mod gc;
mod generations;

fn resolve<T, E: Display>(result: Result<T, E>) -> T {
    match result {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(1)
        },
    }
}

fn mark(generations: &mut [generations::Generation], config: &config::Config) {
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

fn list_generations(generations: &[Generation], user: bool) {
    match user {
        true => println!("{}", "=> Listing system generations".to_string().green()),
        false => println!("{}", "=> Listing user profile generations".to_string().green()),
    };

    for gen in generations {
        let marker = if gen.marked() { "would remove".red() } else { "would keep".green() };
        let id_str = format!("[{}]", gen.number()).yellow();
        println!("{}\t {} days old, {}", id_str, gen.age(), marker);
    }
}

fn remove_generations(generations: &[Generation], user: bool) {
    match user {
        true => println!("{}", "=> Removing old system generations".to_string().green()),
        false => println!("{}", "=> Removing old user profile generations".to_string().green()),
    };

    for gen in generations {
        if gen.marked() {
            println!("{}", format!("-> Removing generation {} ({} days old)", gen.number(), gen.age()).bright_blue());
            resolve(gen.remove());
        } else {
            println!("{}", format!("-> Keeping generation {} ({} days old)", gen.number(), gen.age()).bright_black());
        }
    }
}

fn main() {
    let config = config::Config::parse();

    let user = !config.system;

    let mut generations = match user {
        true => resolve(generations::system_generations()),
        false => resolve(generations::user_generations()),
    };
    mark(&mut generations, &config);

    let no_action_given = !config.list && !config.rm && !config.gc && !config.interactive;
    let interactive = config.interactive || no_action_given;
    let gc = config.gc || no_action_given;

    if config.list {
        // list generations
        list_generations(&generations, user);
        process::exit(0);
    }

    if interactive {
        list_generations(&generations, user);

        println!();
        let confirmation = resolve(ask("Do you want to proceed?"));
        println!();
        if !confirmation {
            println!("-> Not touching anything");
            process::exit(1);
        }

        remove_generations(&generations, user);
    } else if config.rm {
        remove_generations(&generations, user);
    }

    if gc {
        println!();
        println!("{}", "=> Running garbage collection".green());
        resolve(gc::gc());
    }
}
