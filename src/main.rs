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

fn main() {
    // TODO remove with nix-env --delete-generations -p {profile_path()} {no}
    let args = config::Config::parse();
    let user = resolve(user());

    if args.list {
        println!("{}", format!("=> Listing profile generations for user {}", user).green());
        for gen in resolve(generations::user_generations(&user)) {
            println!("no {}, age: {:?}d, path: {}", gen.number(), gen.age(), gen.path().to_string_lossy());
        }
        process::exit(0);
    }

    if args.gc {
        println!();
        println!("{}", "=> Running garbage collection".green());
        resolve(gc::gc());
    }
}
