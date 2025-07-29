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

fn announce(s: String) {
    println!("{}", format!("=> {}", s).green());
}

pub fn announce_listing(profile: &Profile) {
    announce(format!("Listing generations for profile {}", profile.path().to_string_lossy()));
}

pub fn announce_removal(profile: &Profile) {
    announce(format!("Removing old generations for profile {}", profile.path().to_string_lossy()));
}

pub fn announce_gc_roots(nroots_total: usize, nroots_listed: usize) {
    announce(format!("Listing {} gc roots (out of {} total)", nroots_listed, nroots_total));
}

