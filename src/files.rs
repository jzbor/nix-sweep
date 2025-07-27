use std::fs;
use std::num;
use std::os::unix::fs::{FileTypeExt, MetadataExt};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicU64;
use std::sync::atomic::AtomicUsize;
use std::sync::mpsc::*;
use std::sync::RwLock;

use rayon::iter::{IntoParallelRefIterator, ParallelBridge, ParallelIterator};

use crate::caching::Cache;
use crate::interaction::resolve;
use crate::HashMap;
use crate::HashSet;


static INODE_CACHE: Cache<PathBuf, HashMap<InoKey, u64>> = Cache::new();

type Ino = u64;
type DevId = u64;
type InoKey = (DevId, Ino);

// pub fn dir_size_naive(path: &PathBuf) -> u64 {
//     jwalk::WalkDir::new(path).into_iter()
//         .par_bridge()
//         .flatten()
//         .flat_map(|e| e.metadata())
//         .map(|m| m.len())
//         .sum()
// }


pub fn dir_size_naive(path: &PathBuf) -> u64 {
    let counter = AtomicU64::new(0);
    dir_size_naive_helper(path, &counter);
    counter.into_inner()
}

pub fn dir_size_naive_helper(path: &PathBuf, counter: &AtomicU64) {
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
            .flatten()
            .par_bridge()
            .for_each(|entry| dir_size_naive_helper(&entry.path(), &counter));
    } else if ft.is_file() {
        counter.fetch_add(metadata.len(), std::sync::atomic::Ordering::Relaxed);
    }
}

pub fn dir_size_considering_hardlinks_all(paths: &[PathBuf]) -> u64 {
    let known = RwLock::new(HashSet::default());
    let counter = AtomicU64::new(0);
    paths.par_iter()
        .for_each(|p| dir_size_hl_helper(p, &known, &counter));
    counter.into_inner()
}

pub fn dir_size_considering_hardlinks(path: &PathBuf) -> u64 {
    let known = RwLock::new(HashSet::default());
    let counter = AtomicU64::new(0);
    dir_size_hl_helper(path, &known, &counter);
    counter.into_inner()
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

fn dir_size_hl_helper(path: &PathBuf, known: &RwLock<HashSet<InoKey>>, counter: &AtomicU64) {
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
            .for_each(|e| dir_size_hl_helper(&e.path(), known, counter));
    } else if ft.is_file() {
        let ino_id = (metadata.dev(), metadata.ino());
        if !known.read().unwrap().contains(&ino_id) {
            if known.write().unwrap().insert(ino_id) {
                counter.fetch_add(metadata.len(), std::sync::atomic::Ordering::Relaxed);
            }
        }
    }
}

