use std::process;
use std::time::Duration;
use std::time::SystemTime;
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;

use colored::Colorize;
use rayon::iter::IntoParallelRefIterator;
use rayon::iter::ParallelIterator;
use rayon::slice::ParallelSliceMut;

use crate::utils::files::dir_size_considering_hardlinks_all;
use crate::utils::fmt::*;
use crate::nix::store::StorePath;
use crate::HashSet;


const GC_ROOTS_DIR: &str = "/nix/var/nix/gcroots";


#[derive(Clone)]
pub struct GCRoot {
    link: PathBuf,
    age: Result<Duration, String>,
    store_path: Result<StorePath, String>,
}

impl GCRoot {
    fn new(link: PathBuf) -> Result<Self, String> {
        let store_path = StorePath::from_symlink(&link);
        Self::new_with_store_path(link, store_path)
    }

    fn new_with_store_path(link: PathBuf, store_path: Result<StorePath, String>) -> Result<Self, String> {
        let last_modified = fs::symlink_metadata(&link)
            .and_then(|m| m.modified())
            .map_err(|e| format!("Unable to get metadata for path {}: {}", link.to_string_lossy(), e));
        let now = SystemTime::now();
        let age = match last_modified {
            Ok(m) => now.duration_since(m)
                .map_err(|e| format!("Unable to calculate generation age: {e}")),
            Err(e) => Err(e),
        };

        Ok(GCRoot { link, age, store_path })
    }

    pub fn all_search_directory(include_missing: bool) -> Result<Vec<Self>, String> {
        let gc_roots_dir = PathBuf::from_str(GC_ROOTS_DIR)
            .map_err(|e| e.to_string())?;
        let link_locations = find_links(&gc_roots_dir, Vec::new())?;
        let links: Result<Vec<_>, _> = link_locations.into_iter()
            .map(fs::read_link)
            .filter(|r_res| if let Ok(r) = r_res { include_missing || fs::exists(r).unwrap_or(true) } else { true } )
            .collect();


        let mut roots = Vec::new();
        for link_path in links.map_err(|e| e.to_string())? {
            let new = GCRoot::new(link_path)?;
            roots.push(new)
        }

        Ok(roots)
    }

    pub fn all(query_nix: bool, include_proc: bool, include_missing: bool) -> Result<Vec<Self>, String> {
        if include_proc {
            Self::all_with_proc()
        } else if query_nix {
            let mut roots = Self::all_with_proc()?;
            roots.retain(|r| !r.is_proc());
            Ok(roots)
        } else {
            Self::all_search_directory(include_missing)
        }

    }

    pub fn all_with_proc() -> Result<Vec<Self>, String> {
        let output = process::Command::new("nix-store")
            .arg("--gc")
            .arg("--print-roots")
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

        let roots: Vec<_> = String::from_utf8(output.stdout)
            .map_err(|e| e.to_string())?
            .lines()
            .map(|l| l.split_once(" -> "))
            .flatten()
            .filter(|(link, _)| *link != "{censored}")
            .map(|(link, store_path)| (link, StorePath::new(store_path.into())))
            .map(|(link, store_path)| GCRoot::new_with_store_path(link.into(), store_path))
            .collect::<Result<Vec<Self>, String>>()?;

        Ok(roots)
    }

    pub fn link(&self) -> &PathBuf {
        &self.link
    }

    pub fn store_path(&self) -> Result<&StorePath, &String> {
        self.store_path.as_ref()
    }

    pub fn is_accessible(&self) -> bool {
        self.store_path().is_ok()
    }

    pub fn is_profile(&self) -> bool {
        let parent = self.link.parent().unwrap();
        parent.starts_with("/nix/var/nix/profiles")
        || parent.ends_with(".local/state/nix/profiles")
    }

    pub fn is_current(&self) -> bool {
        self.link.starts_with("/run/current-system")
        || self.link.starts_with("/run/booted-system")
        || self.link.ends_with("home-manager/gcroots/current-home")
        || self.link.ends_with("nix/flake-registry.json")
    }

    pub fn is_proc(&self) -> bool {
        self.link().starts_with("/proc")
    }

    pub fn is_independent(&self) -> bool {
        !self.is_profile() && !self.is_current() && !self.is_proc()
    }

    pub fn age(&self) -> Result<&Duration, &String> {
        self.age.as_ref()
    }

    pub fn profile_paths() -> Result<Vec<PathBuf>, String> {
        let links: Option<Vec<_>> = Self::all(false, false, false)?.into_iter()
            .filter(|r| r.is_profile())
            .map(|r| r.link().to_str().map(|s| s.to_owned()))
            .collect();
        let mut paths: Vec<_> = links.ok_or(String::from("Unable to format gc root link"))?
            .par_iter()
            .flat_map(|l| {
                let mut s = match l.strip_suffix("-link") {
                    Some(rem) => rem.to_string(),
                    None => return None,
                };

                while let Some(last) = s.pop() {
                    if !last.is_numeric() {
                        match last {
                            '-' => return Some(PathBuf::from(s)),
                            _ => return None,
                        }
                    }
                }
                None
            }).collect();

        paths.par_sort();
        paths.dedup();

        Ok(paths)
    }

    pub fn closure(&self) -> Result<HashSet<StorePath>, String> {
        self.store_path.clone().and_then(|sp| sp.closure())
    }

    pub fn closure_size(&self) -> Result<u64, String> {
        self.store_path.clone().map(|sp| sp.closure_size())
    }

    pub fn full_closure(roots: &[Self]) -> Result<HashSet<StorePath>, String> {
        let full_closure: HashSet<_> = roots.par_iter()
            .flat_map(|r| r.closure())
            .flatten()
            .collect();
        Ok(full_closure)
    }

    pub fn full_closure_size(roots: &[Self]) -> Result<u64, String> {
        let full_closure: Vec<_> = Self::full_closure(roots)?
            .iter()
            .map(|sp| sp.path())
            .cloned()
            .collect();
        Ok(dir_size_considering_hardlinks_all(&full_closure))
    }

    pub fn filter_roots(mut roots: Vec<Self>, include_profiles: bool, include_current: bool, include_inaccessible: bool,
                        older: Option<Duration>, newer: Option<Duration>) -> Vec<Self>{
        if !include_profiles {
            roots.retain(|r| !r.is_profile());
        }
        if !include_current {
            roots.retain(|r| !r.is_current());
        }
        if !include_inaccessible {
            roots.retain(|r| r.is_accessible());
        }

        if let Some(older) = older {
            roots.retain(|r| match r.age() {
                Ok(age) => age > &older,
                Err(_) => true,
            })
        }
        if let Some(newer) = newer {
            roots.retain(|r| match r.age() {
                Ok(age) => age <= &newer,
                Err(_) => true,
            })
        }

        roots
    }

    pub fn print_concise(&self, closure_size: Option<u64>, show_size: bool, max_col_len: usize) {
        let size_str = if show_size {
            FmtOrNA::mapped(closure_size, FmtSize::new)
                .left_pad()
        } else {
            String::new()
        };
        let age_str = FmtOrNA::mapped(self.age().ok(), |s| FmtAge::new(*s).with_suffix::<4>(" old".to_owned()))
            .or_empty()
            .right_pad();

        let link = self.link().to_string_lossy().to_string();
        let link_str = FmtWithEllipsis::fitting_terminal(link, max_col_len, 32)
            .right_pad();

        println!("{}  {}    {}",
            link_str,
            size_str.yellow(),
            age_str.bright_blue());
    }

    pub fn print_fancy(&self, closure_size: Option<u64>, show_size: bool) {
        let attribute_items: Vec<String> = [
            (self.is_profile(), "profile"),
            (self.is_current(), "current"),
            (self.is_proc(), "process"),
        ].iter()
            .map(|(b, n)| if *b { n.to_string() } else { String::new() })
            .filter(|s| !s.is_empty())
            .collect();

        let attributes = if attribute_items.is_empty() {
            "(other)".to_owned()
        } else {
            format!("({})", attribute_items.join(", "))
        };

        let age_str = self.age()
            .ok()
            .map(|a| FmtAge::new(*a).to_string());

        let (store_path, size) = if let Ok(store_path) = self.store_path() {
            let store_path_str = store_path.path().to_string_lossy().into();
            if let Some(closure_size) = closure_size {
                (store_path_str, Some(FmtSize::new(closure_size)))
            } else {
                (store_path_str, None)
            }
        } else {
            (String::from("<not accessible>"), None)
        };

        println!("\n{}", self.link().to_string_lossy());
        println!("{}", format!("  -> {store_path}").bright_black());
        print!("  ");
        match age_str {
            Some(age) => print!("age: {}, ", age.bright_blue()),
            None => print!("age: {}, ", "n/a".bright_blue()),
        }
        if show_size {
            match size {
                Some(size) => print!("closure size: {}, ", size.to_string().yellow()),
                None => print!("closure size: {}, ", "n/a".to_string().yellow()),
            }
        }
        println!("type: {}", attributes.blue());
    }
}

fn find_links(path: &PathBuf, mut links: Vec<PathBuf>) -> Result<Vec<PathBuf>, String> {
    let metadata = path.symlink_metadata()
        .map_err(|e| e.to_string())?;
    let ft = metadata.file_type();

    if ft.is_dir() {
        for entry in fs::read_dir(path).map_err(|e| e.to_string())? {
            let child_path = entry
                .map_err(|e| e.to_string())?
                .path();
            links = find_links(&child_path, links)?;
        }
    } else if ft.is_symlink() {
        links.push(path.clone());
    }

    Ok(links)
}
