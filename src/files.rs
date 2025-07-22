use smol::fs;
use std::num;
use std::os::unix::fs::{FileTypeExt, MetadataExt};
use std::path::{Path, PathBuf};

use async_recursion::async_recursion;
use smol::stream::StreamExt;

use crate::caching::Cache;
use crate::HashMap;


static INODE_CACHE: Cache<PathBuf, HashMap<InoKey, u64>> = Cache::new();

type Ino = u64;
type DevId = u64;
type InoKey = (DevId, Ino);


#[async_recursion]
pub async fn dir_size_naive(path: &PathBuf) -> u64 {
    let metadata = match path.symlink_metadata() {
        Ok(meta) => meta,
        Err(_) => return 0,
    };
    let ft = metadata.file_type();

    if ft.is_dir() {
        let read_dir = match fs::read_dir(path).await {
            Ok(rd) => rd,
            Err(_) => return 0,
        };

        let results: Vec<_> = read_dir
            .filter(|entry| entry.is_ok())
            .map(|entry| entry.unwrap())
            .map(async |entry| dir_size_naive(&entry.path()).await)
            .collect()
            .await;
        futures::future::join_all(results).await
            .into_iter()
            .sum()
    } else if ft.is_file() {
        metadata.len()
    } else {
        0
    }
}

pub async fn dir_size_considering_hardlinks_all(paths: &[PathBuf]) -> u64 {
    let mut inodes = HashMap::default();
    let mut stream = smol::stream::iter(paths)
        .map(|p| (p, INODE_CACHE.lookup(p)))
        .map(async |(p, inoo)| match inoo {
            Some(inodes) => inodes,
            None => INODE_CACHE.insert_inline(p.clone(), dir_size_hl_helper(p.clone()).await),
        });

    while let Some(e) = stream.next().await {
        inodes.extend(e.await)
    }

    inodes.values().sum()
}

pub async fn dir_size_considering_hardlinks(path: &PathBuf) -> u64 {
    let inodes = match INODE_CACHE.lookup(path) {
        Some(inodes) => inodes,
        None => INODE_CACHE.insert_inline(path.clone(), dir_size_hl_helper(path.clone()).await),
    };
    inodes.values().sum()
}

pub fn blkdev_of_path(path: &Path) -> Result<String, String> {
    let dev = path.symlink_metadata()
        .map_err(|e| e.to_string())?
        .dev();
    find_blkdev(dev)
}

fn find_blkdev(id: u64) -> Result<String, String> {
    std::fs::read_dir("/dev")
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
    std::fs::read_to_string(size_file_path)
        .map_err(|e| e.to_string())?
        .lines()
        .next()
        .ok_or(String::from("Size file empty"))?
        .parse()
        .map_err(|e: num::ParseIntError| e.to_string())
        .map(|n: u64| n * 512)
}

#[async_recursion]
async fn dir_size_hl_helper(path: PathBuf) -> HashMap<InoKey, u64> {
    let metadata = match fs::symlink_metadata(&path).await {
        Ok(meta) => meta,
        Err(_) => return HashMap::default(),
    };

    if metadata.is_dir() {
        let mut readdir = match fs::read_dir(path).await {
            Ok(rd) => rd,
            Err(_) => return HashMap::default(),
        };

        let mut acc = HashMap::default();

        while let Ok(Some(entry)) = readdir.try_next().await {
            acc.extend(dir_size_hl_helper(entry.path()).await);
        }

        acc
    } else if metadata.is_file() {
        let mut new = HashMap::default();
        new.insert((metadata.dev(), metadata.ino()), metadata.len());
        new
    } else {
        HashMap::default()
    }
}
