use std::collections::HashMap;
use std::ops::Deref;
use std::str::FromStr;
use std::sync::Mutex;
use std::{fs, process};
use std::path::PathBuf;


static STORE_PATH_SIZE_CACHE: Mutex<Option<HashMap<PathBuf, u64>>> = Mutex::new(None);

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct StorePath(PathBuf);

impl StorePath {
    pub fn new(path: PathBuf) -> Result<Self, String> {
        if !path.starts_with("/nix/store") {
            Err(format!("'{}' is not a path in the nix store", path.to_string_lossy()))
        } else {
            Ok(StorePath(path))
        }
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

        String::from_utf8(output.stdout)
            .map_err(|e| e.to_string())?
            .lines()
            .map(PathBuf::from_str)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
            .map(|i| i.into_iter().map(StorePath).collect())
    }

    pub fn closure_size(&self) -> u64 {
        let paths = self.closure().unwrap_or_default();
        paths.iter()
            .map(|p| p.size())
            .sum()
    }

    pub fn added_closure_size(&self, counts: &HashMap<StorePath, usize>) -> u64{
        let paths = self.closure().unwrap_or_default();
        paths.iter()
            .filter(|p| counts.get(p).cloned().unwrap_or(1) <= 1)
            .map(|p| p.size())
            .sum()
    }
}


fn dir_size(path: &PathBuf) -> u64 {
    let mut size = 0;
    let metadata = match path.metadata() {
        Ok(meta) => meta,
        Err(_) => return 0,
    };
        // .map_err(|e| e.to_string())?;
    let ft = metadata.file_type();

    if ft.is_dir() {
        for entry in fs::read_dir(path).into_iter().flatten() {
            let child_path = match entry {
                Ok(e) => e.path(),
                Err(_) => continue,
            };
            size += dir_size(&child_path);
        }
    } else if ft.is_file() {
        size += metadata.len();
    }

    size
}

fn store_path_size_cache_lookup(path: &PathBuf) -> Option<u64> {
    if let Some(cache) = STORE_PATH_SIZE_CACHE.lock().unwrap().deref() {
        cache.get(path).cloned()
    } else {
        None
    }
}

fn store_path_size_cache_insert(path: PathBuf, size: u64) {
    let mut cache_opt = STORE_PATH_SIZE_CACHE.lock().unwrap();

    if let Some(cache) = cache_opt.as_mut() {
        cache.insert(path, size);
    } else {
        let mut cache = HashMap::new();
        cache.insert(path, size);
        *cache_opt = Some(cache);
    }
}
