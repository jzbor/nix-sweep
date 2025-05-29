use std::os::unix::fs::MetadataExt;
use std::sync::Mutex;
use std::{collections::HashMap, fs};
use std::path::{Path, PathBuf};

use rayon::iter::{ParallelBridge, ParallelIterator};

pub fn dir_size(path: &PathBuf) -> u64 {
    let metadata = match path.symlink_metadata() {
        Ok(meta) => meta,
        Err(_) => return 0,
    };
    let ft = metadata.file_type();

    if ft.is_dir() {
        let read_dir = match fs::read_dir(path) {
            Ok(rd) => rd,
            Err(_) => return 0,
        };
        read_dir.into_iter()
            .flatten()
            .par_bridge()
            .map(|entry| dir_size(&entry.path()))
            .sum()
    } else if ft.is_file() {
        metadata.len()
    } else {
        0
    }
}

pub fn read_link_full(path: &PathBuf) -> Result<PathBuf, String> {
    if path.is_symlink() {
        let next = fs::read_link(path)
            .map_err(|e| e.to_string())?;
        read_link_full(&next)
    } else {
        Ok(path.clone())
    }
}

pub fn dir_size_considering_hardlinks_all(paths: &[PathBuf]) -> u64 {
    let files = Mutex::new(HashMap::default());
    for path in paths {
        dir_size_hl_helper(path.clone(), &files);
    }
    files.into_inner().unwrap().values().sum()
}

pub fn dir_size_considering_hardlinks(path: &Path) -> u64 {
    let files = Mutex::new(HashMap::default());
    dir_size_hl_helper(path.to_path_buf(), &files);
    files.into_inner().unwrap().values().sum()
}

fn dir_size_hl_helper(path: PathBuf, files: &Mutex<HashMap<u64, u64>>) {
    let metadata = match path.symlink_metadata() {
        Ok(meta) => meta,
        Err(_) => return,
    };
    let ft = metadata.file_type();

    if ft.is_dir() {
        let read_dir = match fs::read_dir(path) {
            Ok(rd) => rd,
            Err(_) => return,
        };
        read_dir.into_iter()
            .par_bridge()
            .flatten()
            .for_each(|entry| dir_size_hl_helper(entry.path(), files));
    } else if ft.is_file() {
        files.lock().unwrap()
            .insert(metadata.ino(), metadata.len());
    }
}

