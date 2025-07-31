use std::str::FromStr;
use std::{fs, process};
use std::path::{Path, PathBuf};

use rayon::slice::ParallelSliceMut;

use crate::caching::Cache;
use crate::{files::{self, *}, HashSet};


pub const NIX_STORE: &str = "/nix/store";
static CLOSURE_CACHE: Cache<StorePath, HashSet<StorePath>> = Cache::new();


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

        paths.par_sort_by_key(|sp| sp.0.clone());
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

    pub fn size_naive() -> Result<u64, String> {
        let total_size: u64 = Store::all_paths()?
            .iter()
            .map(|sp| sp.size_naive())
            .sum();
        Ok(total_size)
    }

    pub fn size() -> Result<u64, String> {
        let store_path = std::path::PathBuf::from(NIX_STORE);
        let size = dir_size_considering_hardlinks(&store_path);
        Ok(size)
    }

    pub fn blkdev() -> Result<String, String> {
        files::blkdev_of_path(Path::new(NIX_STORE))
    }

    pub fn gc() -> Result<(), String> {
        let result = process::Command::new("nix-store")
            .arg("--gc")
            .stdin(process::Stdio::inherit())
            .stdout(process::Stdio::inherit())
            .stderr(process::Stdio::inherit())
            .status();

        match result {
            Ok(status) => if status.success() {
                Ok(())
            } else {
                Err("Garbage collection failed".to_string())
            },
            Err(e) => Err(format!("Garbage collection failed: {}", e)),
        }
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
        let path = fs::canonicalize(link)
            .map_err(|e| e.to_string())?;
        Self::new(path)
    }

    pub fn path(&self) -> &PathBuf {
        &self.0
    }

    pub fn size(&self) -> u64 {
        dir_size_considering_hardlinks(&self.0)
    }

    pub fn size_naive(&self) -> u64 {
        dir_size_naive(&self.0)
    }

    pub fn closure(&self) -> Result<HashSet<StorePath>, String> {
        if let Some(closure) = CLOSURE_CACHE.lookup(self) {
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

        let closure: HashSet<_> = String::from_utf8(output.stdout)
            .map_err(|e| e.to_string())?
            .lines()
            .map(PathBuf::from_str)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
            .map(|i| i.into_iter().map(StorePath).collect())?;

        CLOSURE_CACHE.insert(self.clone(), closure.clone());
        Ok(closure)
    }

    pub fn closure_size(&self) -> u64 {
        let closure: Vec<_> = self.closure().unwrap_or_default()
            .iter()
            .map(|sp| sp.path())
            .cloned()
            .collect();
        dir_size_considering_hardlinks_all(&closure)
    }

    pub fn closure_size_naive(&self) -> u64 {
       self.closure().unwrap_or_default()
            .iter()
            .map(|sp| sp.path())
            .map(dir_size_naive)
            .sum()
    }
}
