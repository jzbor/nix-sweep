use std::fmt::Display;
use std::io::Write;
use std::process;

use colored::Colorize;

pub fn resolve<T, E: Display>(result: Result<T, E>) -> T {
    match result {
        Ok(t) => t,
        Err(e) => {
            eprintln!("{} {}", "Error:".red(), e);
            process::exit(1)
        },
    }
}

pub fn warn(warning: &str) {
    eprintln!("{} {}", "Warning:".yellow(), warning);
}

pub fn ask(question: &str, default: bool) -> bool {
    loop {
        match default {
            true => print!("{question} [Y/n] "),
            false => print!("{question} [y/N] "),
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

pub fn ack(question: &str) {
    loop {
        print!("{question} [enter] ");
        let _ = std::io::stdout().flush();

        let mut input = String::new();
        match std::io::stdin().read_line(&mut input) {
            Ok(_) => (),
            Err(_) => continue,
        };
        return;
    }
}

pub fn announce(s: String) {
    println!("\n{}", format!("=> {s}").green());
}
