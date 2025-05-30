use std::collections::HashMap;
use std::fs;
use std::num;
use std::os::unix::fs::{FileTypeExt, MetadataExt};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

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

pub fn blkdev_of_path(path: &PathBuf) -> Result<String, String> {
    let dev = path.symlink_metadata()
        .map_err(|e| e.to_string())?
        .dev();
    find_blkdev(dev)
}

pub fn find_blkdev(id: u64) -> Result<String, String> {
    fs::read_dir("/dev")
        .unwrap()
        .into_iter()
        .flatten()
        .flat_map(|e| e.path().file_name().map(|n| (e, n.to_string_lossy().to_string())))
        .flat_map(|(e, n)| e.metadata().map(|m| (n, m)))
        .filter(|(_, m)| m.file_type().is_block_device())
        .find(|(_, m)| m.rdev() == id)
        .map(|(n, _)| n)
        .ok_or(format!("Could not find device for id {}", id))
}

pub fn get_blkdev_size(name: &str) -> Result<u64, String> {
    let size_file_path = PathBuf::from(&format!("/sys/class/block/{}/size", name));
    fs::read_to_string(size_file_path)
        .map_err(|e| e.to_string())?
        .lines()
        .next()
        .ok_or(String::from("Size file empty"))?
        .parse()
        .map_err(|e: num::ParseIntError| e.to_string())
        .map(|n: u64| n * 512)
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

