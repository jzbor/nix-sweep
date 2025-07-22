use std::fmt::Display;
use std::io::Write;
use std::process;

use colored::Colorize;

use crate::profiles::Profile;


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

pub fn ack(question: &str) {
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

pub fn announce_listing(profile: &Profile) {
    println!("{}", format!("=> Listing generations for profile {}", profile.path().to_string_lossy()).to_string().green());
}

pub fn announce_removal(profile: &Profile) {
    println!("{}", format!("=> Removing old generations for profile {}", profile.path().to_string_lossy()).to_string().green());
}

