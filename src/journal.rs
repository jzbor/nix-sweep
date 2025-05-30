use std::fs;
use std::path::{Path, PathBuf};

use crate::files;

pub const JOURNAL_PATH: &str = "/var/log/journal";


pub fn journal_exists() -> bool {
    fs::exists(Path::new(JOURNAL_PATH))
        .unwrap_or(false)
}

pub fn journal_size() -> u64 {
    files::dir_size_naive(&PathBuf::from(JOURNAL_PATH))
}
