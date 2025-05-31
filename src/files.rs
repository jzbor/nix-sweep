use std::fs;
use std::num;
use std::os::unix::fs::{FileTypeExt, MetadataExt};
use std::path::{Path, PathBuf};

use rayon::iter::{IntoParallelRefIterator, ParallelBridge, ParallelIterator};
use rustc_hash::FxHashMap as HashMap;

use crate::caching::Cache;


static INODE_CACHE: Cache<PathBuf, HashMap<InoKey, u64>> = Cache::new();

type Ino = u64;
type DevId = u64;
type InoKey = (DevId, Ino);

pub fn dir_size_naive(path: &PathBuf) -> u64 {
    let metadata = match path.symlink_metadata() {
        Ok(meta) => meta,
        Err(_) => return 0,
    };
    let ft = metadata.file_type();

    let size = if ft.is_dir() {
        let read_dir = match fs::read_dir(path) {
            Ok(rd) => rd,
            Err(_) => return 0,
        };
        read_dir.into_iter()
            .flatten()
            .par_bridge()
            .map(|entry| dir_size_naive(&entry.path()))
            .sum()
    } else if ft.is_file() {
        metadata.len()
    } else {
        0
    };

    size
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
    let inodes = paths.par_iter()
        .map(|p| (p, INODE_CACHE.lookup(p)))
        .map(|(p, inoo)| match inoo {
            Some(inodes) => inodes,
            None => INODE_CACHE.insert_inline(p.clone(), dir_size_hl_helper(p)),
        })
        .reduce(HashMap::default, |mut last, next| { last.extend(next); last });
    inodes.values().sum()
}

pub fn dir_size_considering_hardlinks(path: &PathBuf) -> u64 {
    let inodes = match INODE_CACHE.lookup(path) {
        Some(inodes) => inodes,
        None => INODE_CACHE.insert_inline(path.clone(), dir_size_hl_helper(path)),
    };
    inodes.values().sum()
}

pub fn blkdev_of_path(path: &Path) -> Result<String, String> {
    let dev = path.symlink_metadata()
        .map_err(|e| e.to_string())?
        .dev();
    find_blkdev(dev)
}

pub fn find_blkdev(id: u64) -> Result<String, String> {
    fs::read_dir("/dev")
        .unwrap()
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

fn dir_size_hl_helper(path: &PathBuf) -> HashMap<InoKey, u64> {
    let metadata = match path.symlink_metadata() {
        Ok(meta) => meta,
        Err(_) => return HashMap::default(),
    };
    let ft = metadata.file_type();

    if ft.is_dir() {
        let read_dir = match fs::read_dir(&path) {
            Ok(rd) => rd,
            Err(_) => return HashMap::default(),
        };
        let inodes = read_dir.into_iter()
            .par_bridge()
            .flatten()
            .map(|e| dir_size_hl_helper(&e.path()))
            .reduce(HashMap::default, |mut last, next| { last.extend(next); last });
        inodes
    } else if ft.is_file() {
        let mut new = HashMap::default();
        new.insert((metadata.dev(), metadata.ino()), metadata.len());
        new
    } else {
        HashMap::default()
    }
}

