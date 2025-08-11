use std::os::unix;
use std::{env, fs};
use std::path::PathBuf;

use crate::nix::store::Store;
use crate::utils::fmt::FmtWithEllipsis;
use crate::utils::interaction::conclusion;

use super::Command;


#[derive(clap::Args)]
pub struct AddRootCommand {
    /// Where to point the gc root to
    target: PathBuf,

    /// The preferred name for the gc root
    name: Option<PathBuf>,

    /// Point the gc root directly to the corresponding store path
    #[clap(short, long)]
    direct: bool,
}


impl Command for AddRootCommand {
    fn run(self) -> Result<(), String> {
        if !self.target.exists() {
            return Err("Target does not exist".to_owned());
        }

        let canonic = fs::canonicalize(&self.target)
            .map_err(|e| e.to_string())?;
        if !Store::is_valid_path(&canonic) {
            return Err("Target does not point to a store path".to_owned());
        }

        let root_target = if self.direct {
            canonic
        } else {
            self.target.clone()
        };

        let gc_parent = match env::var("USER") {
            Ok(user) => PathBuf::from(format!("/nix/var/nix/gcroots/per-user/{}", user)),
            Err(_) => PathBuf::from("/nix/var/nix/gcroots"),
        };

        let full_gc_path = match self.name {
            Some(n) => gc_parent.join(n),
            None => {
                let mut count = 0;
                while gc_parent.join(format!("gcroot-{}", count)).is_symlink() {
                    count += 1;
                }
                gc_parent.join(format!("gcroot-{}", count))
            },
        };

        unix::fs::symlink(&root_target, &full_gc_path)
            .map_err(|e| e.to_string())?;

        let target_str = root_target.to_string_lossy().to_string();
        let target_len = target_str.len();
        let root_str = full_gc_path.to_string_lossy().to_string();
        let root_len = root_str.len();
        conclusion(&format!("Added root for {}\n               at {}\n",
            FmtWithEllipsis::fitting_terminal(target_str, target_len, 18),
            FmtWithEllipsis::fitting_terminal(root_str, root_len, 18)));

        Ok(())
    }
}
