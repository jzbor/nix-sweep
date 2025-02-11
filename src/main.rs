use std::fmt::Display;
use std::io::Write;
use std::{env, process};
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

fn user() -> Result<String, String> {
    env::var("USER")
        .map_err(|_| String::from("Unable to read $USER"))
}

fn mark(generations: &mut [generations::Generation], config: &config::Config) {
    // mark older generations
    for generation in generations.iter_mut() {
        if generation.age() > config.older {
            generation.mark();
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

fn list_generations(generations: &[Generation], user: Option<&str>) {
    match user {
        None => println!("{}", format!("=> Listing system generations").green()),
        Some(u) => println!("{}", format!("=> Listing profile generations for user {}", u).green()),
    };

    for gen in generations {
        let marker = if gen.marked() { "would remove".red() } else { "would keep".green() };
        let id_str = format!("[{}]", gen.number()).yellow();
        println!("{}\t {} days old, {}", id_str, gen.age(), marker);
    }
}

fn remove_generations(generations: &[Generation], user: Option<&str>) {
    match &user {
        None => println!("{}", format!("=> Removing old system generations").green()),
        Some(user) => println!("{}", format!("=> Removing old profile generations for user {}", user).green()),
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

    let user = if config.system { None } else { Some(resolve(user())) };

    let mut generations = match &user {
        None => resolve(generations::system_generations()),
        Some(user) => resolve(generations::user_generations(user)),
    };
    mark(&mut generations, &config);

    if config.list || (!config.list && !config.rm && !config.gc && !config.interactive) {
        // list generations
        list_generations(&generations, user.as_deref());
        process::exit(0);
    }

    if config.rm || config.interactive {
        if config.interactive {
            list_generations(&generations, user.as_deref());
            println!();
            let confirmation = resolve(ask("Do you want to proceed?"));
            if !confirmation {
                println!();
                println!("-> Not touching anything");
                process::exit(1);
            }
        }

        remove_generations(&generations, user.as_deref());
    }

    if config.gc {
        println!();
        println!("{}", "=> Running garbage collection".green());
        resolve(gc::gc());
    }
}
