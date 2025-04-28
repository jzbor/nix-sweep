use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use crate::store_paths::StorePath;


const GC_ROOTS_DIR: &str = "/nix/var/nix/gcroots";

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

fn read_link_full(path: &PathBuf) -> Result<PathBuf, String> {
    if path.is_symlink() {
        let next = fs::read_link(path)
            .map_err(|e| e.to_string())?;
        read_link_full(&next)
    } else {
        Ok(path.clone())
    }
}

pub fn gc_root_is_profile(path: &Path) -> bool {
    let parent = path.parent().unwrap();
    parent.starts_with("/nix/var/nix/profiles")
    || parent.ends_with(".local/state/nix/profiles")
}

pub fn gc_root_is_current(path: &Path) -> bool {
    path.starts_with("/run/current-system")
    || path.starts_with("/run/booted-system")
    || path.ends_with("home-manager/gcroots/current-home")
    || path.ends_with("nix/flake-registry.json")
}

pub fn count_gc_deps(gc_roots: &HashMap<PathBuf, Result<StorePath, String>>) -> HashMap<StorePath, usize> {
    gc_roots.iter()
        .filter(|(_, v)| v.is_ok())
        .map(|(_, v)| v.as_ref().unwrap())
        .flat_map(|v| v.closure())
        .flatten()
        .fold(HashMap::new(), |mut acc, v| {
            if let Some(existing) = acc.get_mut(&v) {
                *existing += 1;
            } else {
                acc.insert(v.clone(), 1);
            }
            acc
        })
}

pub fn gc_roots(include_missing: bool) -> Result<HashMap<PathBuf, Result<StorePath, String>>, String> {
    let gc_roots_dir = PathBuf::from_str(GC_ROOTS_DIR)
        .map_err(|e| e.to_string())?;
    let link_locations = find_links(&gc_roots_dir, Vec::new())?;
    let links: Result<Vec<_>, _> = link_locations.into_iter()
        .map(fs::read_link)
        .filter(|r_res| if let Ok(r) = r_res { include_missing || fs::exists(r).unwrap_or(true) } else { true } )
        .collect();


    let mut link_map = HashMap::new();
    for link in links.map_err(|e| e.to_string())? {
        link_map.insert(link.clone(), read_link_full(&link).map(StorePath::new)?);
    }

    Ok(link_map)
}
