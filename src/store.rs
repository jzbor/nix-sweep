use std::collections::HashMap;
use std::ops::Deref;
use std::str::FromStr;
use std::sync::RwLock;
use std::{fs, process};
use std::path::{Path, PathBuf};

use rayon::prelude::*;

use crate::files::*;


pub const NIX_STORE: &str = "/nix/store";
static STORE_PATH_SIZE_CACHE: RwLock<Option<HashMap<PathBuf, u64>>> = RwLock::new(None);
static CLOSURE_CACHE: RwLock<Option<HashMap<StorePath, Vec<StorePath>>>> = RwLock::new(None);


#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct StorePath(PathBuf);

pub struct Store();


impl Store {
    pub fn all_paths() -> Result<Vec<StorePath>, String> {
        let read_dir = match fs::read_dir(NIX_STORE) {
            Ok(rd) => rd,
            Err(e) => return Err(e.to_string()),
        };
        let mut paths: Vec<_> = read_dir.into_iter()
            .flatten()
            .map(|e| e.path())
            .filter(|p| Self::is_valid_path(p))
            .flat_map(StorePath::new)
            .collect();

        paths.sort_by_key(|sp| sp.0.clone());
        paths.dedup();

        Ok(paths)
    }

    pub fn is_valid_path(path: &Path) -> bool {
        let file_name = match path.file_name().and_then(|n| n.to_str()) {
            Some(file_name) => file_name,
            None => return false,
        };

        let is_in_store = path.starts_with(NIX_STORE);
        let has_sufficient_length = file_name.len() > 32;
        let starts_with_hash = file_name.chars()
            .take(32)
            .all(|c| c.is_ascii_alphanumeric() && (c.is_lowercase() || c.is_numeric()));

        is_in_store && has_sufficient_length && starts_with_hash
    }

    pub fn size() -> Result<u64, String> {
        let total_size: u64 = Store::all_paths()?
            .iter()
            .map(|sp| sp.size())
            .sum();
        Ok(total_size)
    }

    pub fn size_considering_hardlinks() -> Result<u64, String> {
        let store_path = std::path::PathBuf::from(NIX_STORE);
        let size = dir_size_considering_hardlinks(&store_path);
        Ok(size)
    }
}

impl StorePath {
    pub fn new(path: PathBuf) -> Result<Self, String> {
        if !Store::is_valid_path(&path) {
            Err(format!("'{}' is not a valid nix store path", path.to_string_lossy()))
        } else {
            Ok(StorePath(path))
        }
    }

    pub fn from_symlink(link: &PathBuf) -> Result<Self, String> {
        let path = read_link_full(link)?;
        Self::new(path)
    }

    pub fn path(&self) -> &PathBuf {
        &self.0
    }

    pub fn size(&self) -> u64 {
        match store_path_size_cache_lookup(self.path()) {
            Some(size) => size,
            None => {
                let size = dir_size(&self.0);
                store_path_size_cache_insert(self.path().clone(), size);
                size
            },
        }
    }

    pub fn closure(&self) -> Result<Vec<StorePath>, String> {
        if let Some(closure) = closure_cache_lookup(self) {
            return Ok(closure);
        }

        let output = process::Command::new("nix-store")
            .arg("--query")
            .arg("--requisites")
            .arg(&self.0)
            .stdin(process::Stdio::inherit())
            .stderr(process::Stdio::inherit())
            .output()
            .map_err(|e| e.to_string())?;

        if !output.status.success() {
            match output.status.code() {
                Some(code) => return Err(format!("`nix-store` failed (exit code {})", code)),
                None => return Err("`nix-store` failed".to_string()),
            }
        }

        let closure: Vec<_> = String::from_utf8(output.stdout)
            .map_err(|e| e.to_string())?
            .lines()
            .map(PathBuf::from_str)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
            .map(|i| i.into_iter().map(StorePath).collect())?;

        closure_cache_insert(self.clone(), closure.clone());
        Ok(closure)
    }

    pub fn closure_size(&self) -> u64 {
        let paths = self.closure().unwrap_or_default();
        paths.iter()
            .map(|p| p.size())
            .sum()
    }

    pub fn closure_size_considering_hardlinks(&self) -> u64 {
        let closure: Vec<_> = self.closure().unwrap_or_default()
            .iter()
            .map(|sp| sp.path())
            .cloned()
            .collect();
        dir_size_considering_hardlinks_all(&closure)
    }

    pub fn added_closure_size(&self, counts: &HashMap<StorePath, usize>) -> u64 {
        let paths = self.closure().unwrap_or_default();
        paths.iter()
            .filter(|p| counts.get(p).cloned().unwrap_or(1) <= 1)
            .map(|p| p.size())
            .sum()
    }
}

pub fn count_closure_paths(input_paths: &[StorePath]) -> HashMap<StorePath, usize> {
    input_paths.par_iter()
        .flat_map(|p| p.closure())
        .flatten()
        .fold(HashMap::new, |mut acc, v| {
            if let Some(existing) = acc.get_mut(&v) {
                *existing += 1;
            } else {
                acc.insert(v.clone(), 1);
            }
            acc
        })
        .reduce_with(|mut m1, m2| {
            for (k, v) in m2 {
                *m1.entry(k).or_default() += v;
            }
            m1
        }).unwrap_or(HashMap::new())
}

fn store_path_size_cache_lookup(path: &PathBuf) -> Option<u64> {
    if let Some(cache) = STORE_PATH_SIZE_CACHE.read().unwrap().deref() {
        cache.get(path).cloned()
    } else {
        None
    }
}

fn store_path_size_cache_insert(path: PathBuf, size: u64) {
    let mut cache_opt = STORE_PATH_SIZE_CACHE.write().unwrap();

    if let Some(cache) = cache_opt.as_mut() {
        cache.insert(path, size);
    } else {
        let mut cache = HashMap::new();
        cache.insert(path, size);
        *cache_opt = Some(cache);
    }
}

fn closure_cache_lookup(path: &StorePath) -> Option<Vec<StorePath>> {
    if let Some(cache) = CLOSURE_CACHE.read().unwrap().deref() {
        cache.get(path).cloned()
    } else {
        None
    }
}

fn closure_cache_insert(path: StorePath, closure: Vec<StorePath>) {
    let mut cache_opt = CLOSURE_CACHE.write().unwrap();

    if let Some(cache) = cache_opt.as_mut() {
        cache.insert(path, closure);
    } else {
        let mut cache = HashMap::new();
        cache.insert(path, closure);
        *cache_opt = Some(cache);
    }
}
