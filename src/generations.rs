use std::env;
use std::fs;
use std::path::Component;
use std::process;
use std::str;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::store_paths::StorePath;

#[derive(Eq, Debug)]
pub struct Generation {
    number: usize,
    path: PathBuf,
    profile_path: PathBuf,
    age: u64,
    marker: bool,
}

impl Generation {
    fn new_from_direntry(name: &str, dirent: &fs::DirEntry) -> Result<Self, String> {
        let file_name = dirent.file_name();
        let file_name = file_name.to_string_lossy();
        let suffix = file_name.strip_prefix(name)
            .ok_or("Cannot create generation representation (missing profile prefix)")?;
        let tokens: Vec<_> = suffix.split('-').collect();
        if tokens.len() != 3 || tokens[2] != "link" {
            return Err(format!("Cannot create generation representation ({:?})", tokens))
        }

        let profile_path = dirent.path().parent().unwrap()
            .join(name);

        let number = str::parse::<usize>(tokens[1])
            .map_err(|_| format!("Cannot parse \"{}\" as generation number", tokens[1]))?;

        let last_modified = fs::symlink_metadata(dirent.path())
            .map_err(|e| format!("Unable to get metadata for path {} ({})", dirent.path().to_string_lossy(), e))?
            .modified()
            .map_err(|e| format!("Unable to get metadata for path {} ({})", dirent.path().to_string_lossy(), e))?;
        let now = SystemTime::now();
        let age = now.duration_since(last_modified)
            .map_err(|e| format!("Unable to calculate generation age ({})", e))?
            .as_secs() / 60 / 60 / 24;

        Ok(Generation {
            number, age,
            path: dirent.path(),
            profile_path,
            marker: false,
        })
    }

    pub fn store_path(&self) -> Result<StorePath, String> {
        StorePath::from_symlink(&self.path)
    }

    pub fn number(&self) -> usize {
        self.number
    }

    pub fn profile_path(&self) -> &Path {
        &self.profile_path
    }

    pub fn age(&self) -> u64 {
        self.age
    }

    pub fn mark(&mut self) {
        self.marker = true;
    }

    pub fn unmark(&mut self) {
        self.marker = false;
    }

    pub fn marked(&self) -> bool{
        self.marker
    }

    pub fn remove(&self) -> Result<(), String> {
        let result = process::Command::new("nix-env")
            .args(["-p", self.profile_path().to_str().unwrap()])
            .args(["--delete-generations", &self.number().to_string()])
            .stdin(process::Stdio::inherit())
            .stdout(process::Stdio::inherit())
            .stderr(process::Stdio::inherit())
            .status();

        match result {
            Ok(status) => if status.success() {
                Ok(())
            } else {
                Err(format!("Removal of generation {} failed", self.number()))
            },
            Err(e) => Err(format!("Removal of generation {} failed ({})", self.number(), e)),
        }
    }
}

impl Ord for Generation {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.number.cmp(&other.number)
    }
}

impl PartialOrd for Generation {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Generation {
    fn eq(&self, other: &Self) -> bool {
        self.path.eq(&other.path)
    }
}

pub fn user_generations() -> Result<Vec<Generation>, String> {
    named_user_generations("profile")
}

pub fn home_generations() -> Result<Vec<Generation>, String> {
    named_user_generations("home-manager")
}

pub fn named_user_generations(profile_name: &str) -> Result<Vec<Generation>, String> {
    let check_path = |path: &str| format!("{}/{}", path, profile_name);
    let user = env::var("USER")
        .map_err(|_| String::from("Unable to read $USER"))?;

    let path = format!("/nix/var/nix/profiles/per-user/{}", user);
    if fs::exists(check_path(&path))
            .map_err(|e| format!("Unable to check path {} ({})", path, e))? {
        return generations(&path, profile_name);
    }

    let home = env::var("HOME")
        .map_err(|_| String::from("Unable to read $USER"))?;

    let path = format!("{}/.local/state/nix/profiles", home);
    if fs::exists(check_path(&path))
            .map_err(|e| format!("Unable to check path {} ({})", path, e))? {
        return generations(&path, profile_name);
    }

    Err("Could not find profile".to_owned())
}

pub fn generations_from_path(path: &Path) -> Result<Vec<Generation>, String> {
    if fs::exists(path)
        .map_err(|e| format!("Unable to check path {} ({})", path.to_string_lossy(), e))? {
        let parent = path.parent()
            .ok_or(format!("Unable to get parent for profile '{}'", path.to_string_lossy()))?
            .to_str()
            .ok_or(format!("Cannot convert profile path '{}' to string", path.to_string_lossy()))?
            .to_owned();
        let profile_name = match path.components().next_back() {
            Some(Component::Normal(s)) => s.to_str()
                .ok_or(format!("Cannot convert profile path '{}' to string", path.to_string_lossy()))?,
            _ => return Err(format!("Unable to retrieve profile name for profile '{}'", path.to_string_lossy())),
        };
        generations(&parent, profile_name)
    } else {
        Err(format!("Could not find profile '{}'", path.to_string_lossy()))
    }
}

pub fn system_generations() -> Result<Vec<Generation>, String> {
    generations("/nix/var/nix/profiles/", "system")
}

fn generations(path: &str, profile_name: &str) -> Result<Vec<Generation>, String> {
    let profile_prefix = format!("{}-", profile_name);
    let mut generations: Vec<_> = fs::read_dir(path)
        .map_err(|e| format!("Unable to read directory {} ({})", path, e))?
        .flatten()
        .filter(|e| e.file_name().to_str().map(|n| n.starts_with(&profile_prefix)).unwrap_or(false))
        .map(|e| Generation::new_from_direntry(profile_name, &e))
        .map(|r| r.unwrap())
        .collect();
    generations.sort();
    Ok(generations)
}

