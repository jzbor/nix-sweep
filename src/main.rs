use std::fmt::Display;
use std::{env, process};
use colored::Colorize;

use clap::Parser;

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

fn main() {
    let config = config::Config::parse();

    let user = if config.system { None } else { Some(resolve(user())) };

    let mut generations = match &user {
        None => resolve(generations::system_generations()),
        Some(user) => resolve(generations::user_generations(user)),
    };
    mark(&mut generations, &config);

    if config.list {
        match user {
            None => println!("{}", format!("=> Listing system generations").green()),
            Some(u) => println!("{}", format!("=> Listing profile generations for user {}", u).green()),
        };

        for gen in generations {
            let marker = if gen.marked() { "remove".red() } else { "keep".green() };
            let id_str = format!("[{}]", gen.number()).yellow();
            let age_str = format!("{} {}d", "age:".bright_blue(), gen.age());
            let marked_str = format!("{} {}", "marked:".bright_blue(), marker);
            println!("{}\t{}\t{}", id_str, age_str, marked_str);
        }
        process::exit(0);
    }

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

    if config.gc {
        println!();
        println!("{}", "=> Running garbage collection".green());
        resolve(gc::gc());
    }
}
