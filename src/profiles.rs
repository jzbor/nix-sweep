use std::env;
use std::fs;
use std::path;
use std::path::Component;
use std::process;
use std::str;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::Duration;
use std::time::SystemTime;

use colored::Colorize;
use rayon::iter::IndexedParallelIterator;
use rayon::iter::IntoParallelRefIterator;
use rayon::iter::ParallelIterator;
use rayon::slice::ParallelSliceMut;

use crate::config;
use crate::files::dir_size_considering_hardlinks_all;
use crate::fmt::FmtAge;
use crate::fmt::FmtSize;
use crate::fmt::Formattable;
use crate::interaction::announce_listing;
use crate::ordered_channel::OrderedChannel;
use crate::store::StorePath;
use crate::HashSet;


#[derive(Debug)]
pub struct Profile {
    parent: PathBuf,
    name: String,
    generations: Vec<Generation>,
}

#[derive(Eq, Debug)]
pub struct Generation {
    number: usize,
    path: PathBuf,
    profile_path: PathBuf,
    age: Duration,
    marker: bool,
}


impl Profile {
    pub fn new(parent: PathBuf, name: String) -> Result<Self, String> {
        let full_path = parent.clone().join(&name);
        if !fs::exists(&full_path)
            .map_err(|e| format!("Unable to check path {} ({})", full_path.to_string_lossy(), e))? {
            return Err(format!("Could not find profile '{}'", full_path.to_string_lossy()));
        }

        // discover generations
        let profile_prefix = format!("{}-", name);
        let mut generations: Vec<_> = fs::read_dir(&parent)
            .map_err(|e| format!("Unable to read directory {} ({})", parent.to_string_lossy(), e))?
            .flatten()
            .filter(|e| e.file_name().to_str().map(|n| n.starts_with(&profile_prefix)).unwrap_or(false))
            .map(|e| Generation::new_from_direntry(&name, &e))
            .map(|r| r.unwrap())
            .collect();
        generations.sort();

        Ok(Profile { parent, name, generations })
    }

    pub fn from_path(path: PathBuf) -> Result<Self, String> {
        // get parent and name
        let parent = path.parent()
            .ok_or(format!("Unable to get parent for profile '{}'", path.to_string_lossy()))?
            .to_path_buf();
        let name = match path.components().next_back() {
            Some(Component::Normal(s)) => s.to_str()
                .ok_or(format!("Cannot convert profile path '{}' to string", path.to_string_lossy()))?
                .to_owned(),
            _ => return Err(format!("Unable to retrieve profile name for profile '{}'", path.to_string_lossy())),
        };

        Profile::new(parent, name)
    }

    pub fn new_user_profile(name: String) -> Result<Self, String> {
        let check_path = |path: &str| fs::exists(format!("{}/{}", path, name))
                .map_err(|e| format!("Unable to check path {} ({})", path, e));
        let user = env::var("USER")
            .map_err(|_| String::from("Unable to read $USER"))?;

        let path = format!("/nix/var/nix/profiles/per-user/{}", user);
        if check_path(&path)? {
            return Self::new(PathBuf::from(path), name);
        }

        let home = env::var("HOME")
            .map_err(|_| String::from("Unable to read $USER"))?;

        let path = format!("{}/.local/state/nix/profiles", home);
        if check_path(&path)? {
            return Self::new(PathBuf::from(path), name);
        }

        Err("Could not find profile".to_owned())
    }

    pub fn system() -> Result<Self, String> {
        Self::new(PathBuf::from("/nix/var/nix/profiles/"), String::from("system"))
    }

    pub fn home() -> Result<Self, String> {
        Self::new_user_profile(String::from("home-manager"))
    }

    pub fn user() -> Result<Self, String> {
        Self::new_user_profile(String::from("profile"))
    }

    pub fn apply_markers(&mut self, config: &config::ConfigPreset) {
        // negative criteria are applied first

        // mark older generations
        if let Some(older) = config.remove_older {
            for generation in self.generations.iter_mut() {
                if generation.age() >= older {
                    generation.mark();
                }
            }
        }

        // mark superfluous generations
        if let Some(max) = config.keep_max {
            for (i, generation) in self.generations.iter_mut().rev().enumerate() {
                if i >= max {
                    generation.mark();
                }
            }
        }

        // unmark newer generations
        if let Some(newer) = config.keep_newer {
            for generation in self.generations.iter_mut() {
                if generation.age() < newer {
                    generation.unmark();
                }
            }
        }

        // unmark kept generations
        if let Some(min) = config.keep_min {
            for (i, generation) in self.generations.iter_mut().rev().enumerate() {
                if i < min {
                    generation.unmark();
                }
            }
        }

        // always unmark newest generation
        if let Some(newest) = self.generations.last_mut() {
            newest.unmark()
        }

        // always unmark currently active generation
        if let Ok(active) = self.active_generation_mut() {
            active.unmark()
        }
    }

    pub fn path(&self) -> PathBuf {
        self.parent.clone().join(&self.name)
    }

    pub fn generations(&self) -> &[Generation] {
        &self.generations
    }

    pub fn active_generation(&self) -> Result<&Generation, String> {
        let gen_name = fs::read_link(self.path())
            .map(|p| p.to_path_buf())
            .map_err(|e| e.to_string())?;
        let gen_path = self.parent.join(gen_name);

        self.generations.iter()
            .find(|g| g.path() == gen_path)
            .ok_or("Cannot find current generation".to_owned())
    }

    pub fn active_generation_mut(&mut self) -> Result<&mut Generation, String> {
        let gen_name = fs::read_link(self.path())
            .map(|p| p.to_path_buf())
            .map_err(|e| e.to_string())?;
        let gen_path = self.parent.join(gen_name);

        self.generations.iter_mut()
            .find(|g| g.path() == gen_path)
            .ok_or("Cannot find current generation".to_owned())
    }


    pub fn list_generations(&self, print_size: bool, print_markers: bool) {
        announce_listing(self);

        let store_paths: Vec<_> = self.generations().iter()
            .flat_map(|g| g.store_path())
            .collect();

        let ordered_channel: OrderedChannel<_> = OrderedChannel::new();
        let gens = self.generations();
        let ngens = gens.len();

        rayon::join( || {
            gens.par_iter()
                .enumerate()
                .map(|(i, gen)| {
                    let active = self.is_active_generation(gen);
                    let size = if print_size {
                        match gen.store_path() {
                            Ok(sp) => sp.closure_size(),
                            Err(_) => 0,
                        }
                    } else { 0 };
                    (i, active, size)
                })
                .for_each(|tup| ordered_channel.put(tup.0, tup));
        }, || {
                for (i, active, size) in ordered_channel.iter(ngens) {
                    gens[i].print_fancy(active, print_markers, Some(size));
                }
        });

        if print_size {
            let paths: HashSet<_> = store_paths.par_iter()
                .flat_map(|sp| sp.closure())
                .flatten()
                .collect();
            let kept_paths: HashSet<_> = self.generations().par_iter()
                .filter(|g| !g.marked())
                .flat_map(|g| g.store_path())
                .flat_map(|sp| sp.closure())
                .flatten()
                .collect();

            let dirs: Vec<_> = paths.iter().map(|sp| sp.path())
                .cloned()
                .collect();
            let kept_dirs: Vec<_> = kept_paths.iter().map(|sp| sp.path())
                .cloned()
                .collect();
            let size = dir_size_considering_hardlinks_all(&dirs);
            let kept_size = dir_size_considering_hardlinks_all(&kept_dirs);


            println!();
            println!("Estimated total size: {} ({} store paths)",
                FmtSize::new(size).to_string().yellow(), paths.len());
            if print_markers {
                println!("  -> after removal:   {} ({} store paths)",
                    FmtSize::new(kept_size).to_string().green(), kept_paths.len());
            }
        }

        println!();
    }


    pub fn is_active_generation(&self, generation: &Generation) -> bool {
        let active = match self.active_generation() {
            Ok(gen) => gen,
            Err(_) => return false,
        };
        active == generation
    }

    pub fn full_closure(&self) -> Result<HashSet<StorePath>, String> {
        let closures: Result<Vec<_>, _> = self.generations.par_iter()
            .map(|g| g.closure())
            .collect();
        let full_closure: HashSet<_> = closures?
            .into_iter()
            .flatten()
            .collect();

        Ok(full_closure)
    }

    pub fn full_closure_size(&self) -> Result<u64, String> {
        let full_closure: Vec<_> = self.full_closure()?
            .iter()
            .map(|sp| sp.path())
            .cloned()
            .collect();
        Ok(dir_size_considering_hardlinks_all(&full_closure))
    }
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
            .map_err(|e| format!("Unable to calculate generation age ({})", e))?;

        Ok(Generation {
            number, age,
            path: dirent.path(),
            profile_path,
            marker: false,
        })
    }



    pub fn path(&self) -> &Path {
        &self.path
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

    pub fn age(&self) -> Duration {
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

    pub fn closure(&self) -> Result<HashSet<StorePath>, String> {
        self.store_path().and_then(|sp| sp.closure())
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

    pub fn print_fancy(&self, active: bool, print_marker: bool, size: Option<u64>) {
        let marker = if self.marked() { "would remove".red() } else { "would keep".green() };
        let id_str = format!("[{}]", self.number()).bright_blue();

        print!("{}\t{}", id_str,
            FmtAge::new(self.age())
                .with_suffix::<4>(" old".to_owned())
                .left_pad());

        if print_marker {
            print!(", {}", marker);
        }

        if let Some(size) = size {
            let closure_size_str = FmtSize::new(size)
                .bracketed()
                .with_square_brackets()
                .right_pad();
            print!(" \t{}", closure_size_str.yellow());
        }

        if active {
            print!("\t<- active");
        }

        println!();
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

impl FromStr for Profile {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "user" => Profile::user(),
            "home" => Profile::home(),
            "system" => Profile::system(),
            other => {
                let path = path::PathBuf::from_str(other)
                    .map_err(|e| e.to_string())?;
                Profile::from_path(path)
            },
        }
    }
}
