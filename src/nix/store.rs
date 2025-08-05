use std::hash::{Hash, Hasher};
use std::str::FromStr;
use std::{fs, process};
use std::path::{Path, PathBuf};

use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use crate::utils::caching::Cache;
use crate::utils::files;
use crate::HashSet;


pub const NIX_STORE: &str = "/nix/store";
const CLOSURE_LOOKUP_CHUNK_SIZE: usize = 1024;
static CLOSURE_CACHE: Cache<u64, HashSet<StorePath>> = Cache::new();


#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct StorePath(PathBuf);

pub struct Store();


impl Store {
    pub fn all_paths() -> Result<HashSet<StorePath>, String> {
        let read_dir = match fs::read_dir(NIX_STORE) {
            Ok(rd) => rd,
            Err(e) => return Err(e.to_string()),
        };

        let paths = read_dir.into_iter()
            .flatten()
            .map(|e| e.path())
            .filter(|p| Self::is_valid_path(p))
            .flat_map(StorePath::new)
            .collect();
        Ok(paths)
    }

    pub fn paths_dead() -> Result<HashSet<StorePath>, String> {
        Self::paths_with_flag("--print-dead")
    }

    fn paths_with_flag(flag: &str) -> Result<HashSet<StorePath>, String> {
        let output = process::Command::new("nix-store")
            .arg("--gc")
            .arg(flag)
            .output()
            .map_err(|e| e.to_string())?;

        if !output.status.success() {
            match output.status.code() {
                Some(code) => return Err(format!("`nix-store` failed (exit code {code})")),
                None => return Err("`nix-store` failed".to_string()),
            }
        }

        let paths: HashSet<_> = String::from_utf8(output.stdout)
            .map_err(|e| e.to_string())?
            .lines()
            .flat_map(|p| StorePath::new(p.into()))
            .collect();

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
        let size = files::dir_size_considering_hardlinks(&store_path);
        Ok(size)
    }

    pub fn blkdev() -> Result<String, String> {
        files::blkdev_of_path(Path::new(NIX_STORE))
    }

    pub fn gc(max_freed: Option<u64>) -> Result<(), String> {
        let mut command = process::Command::new("nix-store");
        command.arg("--gc");
        if let Some(amount) = max_freed {
            command.args(["--max-freed".to_owned(), format!("{amount}")]);
        }
        let result = command
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
            Err(e) => Err(format!("Garbage collection failed: {e}")),
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
        files::dir_size_considering_hardlinks(&self.0)
    }

    pub fn size_naive(&self) -> u64 {
        files::dir_size_naive(&self.0)
    }

    pub fn is_drv(&self) -> bool {
        self.0.to_string_lossy().ends_with("drv")
    }

    pub fn closure(&self) -> Result<HashSet<StorePath>, String> {
        Self::closure_helper(&[self])
    }

    pub fn closure_size(&self) -> u64 {
        let closure: Vec<_> = self.closure().unwrap_or_default()
            .iter()
            .map(|sp| sp.path())
            .cloned()
            .collect();
        files::dir_size_considering_hardlinks_all(&closure)
    }

    pub fn closure_size_naive(&self) -> u64 {
       self.closure().unwrap_or_default()
            .iter()
            .map(|sp| sp.path())
            .map(files::dir_size_naive)
            .sum()
    }

    fn closure_helper(paths: &[&Self]) -> Result<HashSet<StorePath>, String> {
        let key_hash = {
            let mut hasher = crate::Hasher::default();
            paths.hash(&mut hasher);
            hasher.finish()
        };
        if let Some(closure) = CLOSURE_CACHE.lookup(&key_hash) {
            return Ok(closure);
        }

        let paths: Vec<_> = paths.iter().map(|sp| sp.path().clone()).collect();
        let output = process::Command::new("nix-store")
            .arg("--query")
            .arg("--requisites")
            .args(&paths)
            .stdin(process::Stdio::inherit())
            .stderr(process::Stdio::inherit())
            .output()
            .map_err(|e| e.to_string())?;

        if !output.status.success() {
            match output.status.code() {
                Some(code) => return Err(format!("`nix-store` failed (exit code {code})")),
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

        CLOSURE_CACHE.insert(key_hash, closure.clone());

        Ok(closure)
    }

    pub fn full_closure(paths: &[&Self]) -> HashSet<StorePath> {
        let chunks: Vec<_> = paths.chunks(CLOSURE_LOOKUP_CHUNK_SIZE).collect();
        chunks.par_iter()
            .flat_map(|c| Self::closure_helper(c))
            .flatten()
            .collect()
    }

}
