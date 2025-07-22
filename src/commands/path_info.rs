use std::fs;
use std::path::PathBuf;

use colored::Colorize;

use crate::fmt::*;
use crate::store::StorePath;


#[derive(clap::Args)]
pub struct PathInfoCommand {
    /// Paths to get information about
    #[clap(required = true)]
    paths: Vec<PathBuf>,
}

impl super::Command for PathInfoCommand {
    fn run(self) -> Result<(), String> {
        for path in &self.paths {
            let metadata = fs::symlink_metadata(path)
                .map_err(|e| e.to_string())?;
            let store_path = StorePath::from_symlink(path)?;
            let closure = store_path.closure()?;
            let size = store_path.size();
            let naive_size = store_path.size_naive();
            let closure_size = store_path.closure_size();
            let naive_closure_size = store_path.closure_size_naive();

            println!();

            if metadata.is_symlink() {
                println!("{}", path.to_string_lossy());
                println!("  {}", format!("-> {}", store_path.path().to_string_lossy()).bright_black());
            } else {
                println!("{}", store_path.path().to_string_lossy());
            }

            println!();

            print!("  size:             {}", FmtSize::new(size).left_pad().bright_yellow());
            if naive_size > size {
                print!(" \t{}", FmtSize::new(naive_size)
                    .with_prefix::<18>("hardlinking saves ".to_owned())
                    .bracketed()
                    .right_pad()
                );
            }
            println!();

            print!("  closure size:     {}", FmtSize::new(closure_size).left_pad().yellow());
            if naive_closure_size > closure_size {
                print!(" \t{}", FmtSize::new(naive_closure_size - closure_size)
                    .with_prefix::<18>("hardlinking saves ".to_owned())
                    .bracketed()
                    .right_pad()
                );
            }
            println!();

            println!("  paths in closure: {:>align$}", closure.len().to_string().bright_blue(), align = FmtSize::MAX_WIDTH);
            println!();
        }

        Ok(())

    }
}
