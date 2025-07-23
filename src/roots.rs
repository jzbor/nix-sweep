use std::time::Duration;
use std::time::SystemTime;
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;

use colored::Colorize;

use crate::fmt::FmtAge;
use crate::fmt::FmtSize;
use crate::store::StorePath;


const GC_ROOTS_DIR: &str = "/nix/var/nix/gcroots";


pub struct GCRoot {
    link: PathBuf,
    age: Result<Duration, String>,
    store_path: Result<StorePath, String>,
}

impl GCRoot {
    fn new(link: PathBuf) -> Result<Self, String> {
        let store_path = StorePath::from_symlink(&link);
        let last_modified = fs::symlink_metadata(&link)
            .and_then(|m| m.modified())
            .map_err(|e| format!("Unable to get metadata for path {} ({})", link.to_string_lossy(), e));
        let now = SystemTime::now();
        let age = match last_modified {
            Ok(m) => now.duration_since(m)
                .map_err(|e| format!("Unable to calculate generation age ({})", e)),
            Err(e) => Err(e),
        };

        Ok(GCRoot { link, age, store_path })
    }

    pub fn link(&self) -> &PathBuf {
        &self.link
    }

    pub fn store_path(&self) -> Result<&StorePath, &String> {
        self.store_path.as_ref()
    }

    pub fn is_profile(&self) -> bool {
        let parent = self.link.parent().unwrap();
        parent.starts_with("/nix/var/nix/profiles")
        || parent.ends_with(".local/state/nix/profiles")
    }

    pub fn is_current(&self) -> bool {
        self.link.starts_with("/run/current-system")
        || self.link.starts_with("/run/booted-system")
        || self.link.ends_with("home-manager/gcroots/current-home")
        || self.link.ends_with("nix/flake-registry.json")
    }

    pub fn is_accessible(&self) ->bool {
        self.store_path().is_ok()
    }

    pub fn age(&self) -> Result<&Duration, &String> {
        self.age.as_ref()
    }

    pub fn profile_paths() -> Result<Vec<PathBuf>, String> {
        let links: Option<Vec<_>> = gc_roots(false)?.into_iter()
            .filter(|r| r.is_profile())
            .map(|r| r.link().to_str().map(|s| s.to_owned()))
            .collect();
        let mut paths: Vec<_> = links.ok_or(String::from("Unable to format gc root link"))?
            .iter()
            .flat_map(|l| {
                let mut s = match l.strip_suffix("-link") {
                    Some(rem) => rem.to_string(),
                    None => return None,
                };

                while let Some(last) = s.pop() {
                    if !last.is_numeric() {
                        match last {
                            '-' => return Some(PathBuf::from(s)),
                            _ => return None,
                        }
                    }
                }
                None
            }).collect();

        paths.sort();
        paths.dedup();

        Ok(paths)
    }


    pub async fn print_fancy(&self, print_size: bool) {
        let attributes = match (self.is_profile(), self.is_current()) {
            (true, true) => "(profile, current)",
            (true, false) => "(profile)",
            (false, true) => "(current)",
            (false, false) => "(other)",
        };

        let age_str = self.age()
            .ok()
            .map(|a| FmtAge::new(*a).to_string());

        let (store_path, size) = if let Ok(store_path) = self.store_path() {
            let store_path_str = store_path.path().to_string_lossy().into();
            if print_size {
                let closure_size = FmtSize::new(store_path.closure_size().await);
                (store_path_str, Some(closure_size))
            } else {
                (store_path_str, None)
            }
        } else {
            (String::from("<not accessible>"), None)
        };

        println!("\n{}", self.link().to_string_lossy());
        println!("{}", format!("  -> {}", store_path).bright_black());
        print!("  ");
        match age_str {
            Some(age) => print!("age: {}, ", age.bright_blue()),
            None => print!("age: {}, ", "n/a".bright_blue()),
        }
        if print_size {
            match size {
                Some(size) => print!("closure size: {}, ", size.to_string().yellow()),
                None => print!("closure size: {}, ", "n/a".to_string().yellow()),
            }
        }
        println!("type: {}", attributes.blue());
    }
}

fn find_links(path: &PathBuf, mut links: Vec<PathBuf>) -> Result<Vec<PathBuf>, String> {
    let metadata = path.symlink_metadata()
        .map_err(|e| e.to_string())?;
    let ft = metadata.file_type();

    if ft.is_dir() {
        for entry in fs::read_dir(path).map_err(|e| e.to_string())? {
            let child_path = entry
                .map_err(|e| e.to_string())?
                .path();
            links = find_links(&child_path, links)?;
        }
    } else if ft.is_symlink() {
        links.push(path.clone());
    }

    Ok(links)
}

pub fn gc_roots(include_missing: bool) -> Result<Vec<GCRoot>, String> {
    let gc_roots_dir = PathBuf::from_str(GC_ROOTS_DIR)
        .map_err(|e| e.to_string())?;
    let link_locations = find_links(&gc_roots_dir, Vec::new())?;
    let links: Result<Vec<_>, _> = link_locations.into_iter()
        .map(fs::read_link)
        .filter(|r_res| if let Ok(r) = r_res { include_missing || fs::exists(r).unwrap_or(true) } else { true } )
        .collect();


    let mut roots = Vec::new();
    for link_path in links.map_err(|e| e.to_string())? {
        let new = GCRoot::new(link_path)?;
        roots.push(new)
    }

    Ok(roots)
}
