use std::time::Duration;
use std::time::SystemTime;
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;

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
