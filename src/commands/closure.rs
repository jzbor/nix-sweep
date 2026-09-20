use std::collections::VecDeque;
use std::fs;
use std::path::PathBuf;

use colored::Colorize;
use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};
use rayon::slice::ParallelSliceMut;

use crate::{HashMap, HashSet};
use crate::utils::fmt::*;
use crate::nix::store::StorePath;


#[derive(clap::Args)]
pub struct ClosureCommand {
    /// Path to closure
    path: PathBuf,

    /// Sort by closure size instead of path size
    #[clap(short('c'),long)]
    by_closure_size: bool,

    /// Reverse sorting
    #[clap(long)]
    reverse: bool,

    /// Reverse sorting
    #[clap(short, long)]
    tree: bool,

    /// Show hashes
    #[clap(short('H'), long)]
    hash: bool,
}

struct FullClosureGraph {
    depends_on: HashMap<StorePath, HashSet<StorePath>>,
    depended_on_by: HashMap<StorePath, HashSet<StorePath>>,
}

struct PathInfo {
    size: u64,
    naive_closure_size: u64,
}

impl FullClosureGraph {
    fn build(path: StorePath) -> Result<Self, String> {
        let closure = path.closure()?;
        let mut depends_on: HashMap<_, _> = closure.iter()
            .cloned()
            .map(|p| (p, HashSet::default()))
            .collect();
        let mut depended_on_by: HashMap<_, _> = closure.iter()
            .cloned()
            .map(|p| (p, HashSet::default()))
            .collect();

        for path in closure {
            let references = path.references()?;
            for reference in references {
                depends_on.get_mut(&path)
                    .map(|s| s.insert(reference.clone()));
                depended_on_by.get_mut(&reference)
                    .map(|s| s.insert(path.clone()));
            }
        }

        Ok(Self { depends_on, depended_on_by })
    }
}


impl ClosureCommand {
    fn list_references(&self, depth: usize, path: &StorePath) -> Result<(), String> {
        let closure = path.closure()?;
        let graph = FullClosureGraph::build(path.to_owned())?;
        let path_infos: HashMap<_, _> = closure.par_iter()
            .map(|p| (p.clone(), (p.size_naive(), p.closure_size_naive())))
            .collect();
        let mut printed = HashSet::default();
        let mut queue = VecDeque::new();
        let mut leading_bars: Vec<bool> = Vec::new();

        queue.push_back((path.to_owned(), 0, false));


        while let Some((cur, depth, last)) = queue.pop_front() {
            let (size, closure_size) = path_infos.get(&cur).unwrap();
            let depends_on = graph.depends_on.get(&cur).unwrap();
            let depended_on_by = graph.depended_on_by.get(&cur).unwrap();


            // eprintln!("printed: {:?}", printed);
            if !printed.contains(&cur) {
                let mut enqueue: Vec<_> = depends_on.iter()
                    .cloned()
                    .collect();
                if self.by_closure_size {
                    enqueue.par_sort_by(|p, o| path_infos.get(p).unwrap().1.cmp(&path_infos.get(o).unwrap().1));
                } else {
                    enqueue.par_sort_by(|p, o| path_infos.get(p).unwrap().0.cmp(&path_infos.get(o).unwrap().0));
                }
                if self.reverse {
                    enqueue.reverse();
                }
                enqueue.into_iter()
                    .enumerate()
                    .for_each(|(i, e)| queue.push_front((e, depth + 1, i == 0)));
                // eprintln!("queue_after: {:?}", queue);
            }


            let full_name = if self.hash {
                format!("{}-{}", cur.hash(), cur.name().bright_blue())
            } else {
                cur.name().bright_blue().to_string()
            };

            if depth > 0 {
                for d in 0..(depth - 1) {
                    if leading_bars.get(d).cloned().unwrap_or_default() {
                        print!("│ ");
                    } else {
                        print!("  ");
                    }
                }

                for _ in leading_bars.len()..=depth {
                    leading_bars.push(true);
                }
                if last {
                    print!("└─");
                    leading_bars[depth - 1] = false;
                } else {
                    print!("├─");
                    leading_bars[depth - 1] = true;
                }

                if depends_on.is_empty() || printed.contains(&cur) {
                    print!("── ");
                } else {
                    print!("┬─ ");
                }
            }

            let line = format!("{}  ({} / {})",
                full_name,
                FmtSize::new(*size).to_string().yellow(),
                FmtSize::new(*closure_size).to_string().yellow());

            if printed.contains(&cur) {
                println!("{}", line.dimmed());
            } else {
                println!("{}", line);
            }
                // offset=(depth as usize).saturating_sub(1_usize));
            printed.insert(cur);
        }

        Ok(())
    }
}


impl super::Command for ClosureCommand {
    fn run(self) -> Result<(), String> {
        if self.tree {
            let root = StorePath::from_symlink(&self.path)?;
            self.list_references(0, &root)?;
        } else {
            let root = StorePath::from_symlink(&self.path)?;
            let root_closure = root.closure()?;
            let mut root_closure_info: Vec<_> = root_closure.into_par_iter()
                .map(|p| (p.name(), p.hash(), p.size_naive(), p.closure_size_naive()))
                .collect();

            if self.by_closure_size {
                root_closure_info.par_sort_by_key(|(_, _, _, closure_size)| *closure_size);
            } else {
                root_closure_info.par_sort_by_key(|(_, _, path_size, _)| *path_size);
            }
            if self.reverse {
                root_closure_info.reverse();
            }

            let longest_name_len = root_closure_info.iter()
                .map(|(n, h, _, _)| if self.hash { h.len() + n.len() + 1 } else { n.len() })
                .max()
                .unwrap_or_default();

            let escape_offset = if colored::control::SHOULD_COLORIZE.should_colorize() {
                9
            } else {
                0
            };

            for (name, hash, size, closure_size) in root_closure_info {
                let full_name = if self.hash {
                    format!("{}-{}", hash, name.bright_blue())
                } else {
                    name.bright_blue().to_string()
                };

                println!("{:<align_name$} {} / {}",
                    full_name,
                    FmtSize::new(size).left_pad().yellow(),
                    FmtSize::new(closure_size).right_pad().yellow(),
                    align_name=longest_name_len + escape_offset);
            }
        }



        Ok(())
    }
}
