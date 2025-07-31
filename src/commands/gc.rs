use colored::Colorize;

use crate::files;
use crate::fmt::{FmtPercentage, FmtSize};
use crate::interaction::{announce, ask};
use crate::store::Store;


const GIB: u64 = 1024 * 1024 * 1024;


#[derive(clap::Args)]
pub struct GCCommand {
    /// Do not ask before running garbage collection
    #[clap(short('n'), long("non-interactive"), action = clap::ArgAction::SetFalse)]  // this is very confusing, but works
    interactive: bool,

    /// Ask before running garbage collection
    #[clap(short('i'), long("interactive"), overrides_with = "interactive")]
    _non_interactive: bool,

    /// Only perform garbage collection, if the store is bigger than BIGGER Gibibytes.
    #[clap(short, long)]
    bigger: Option<u64>,

    /// Only perform garbage collection, if the store uses a bigger percentage of its disk than QUOTA%.
    #[clap(short, long)]
    quota: Option<u64>,

    /// Don't actually run garbage collection
    #[clap(short, long)]
    dry_run: bool,
}

impl GCCommand {
    pub fn new(interactive: bool, dry_run: bool) -> Self {
        GCCommand { interactive, dry_run, _non_interactive: !interactive, bigger: None, quota: None }
    }
}

impl super::Command for GCCommand {
    fn run(self) -> Result<(), String> {
        if let Some(bigger) = self.bigger {
            if bigger != 0 {
                eprintln!("Calculating store size...");
                let size = Store::size()?;
                eprintln!("Store has a size of {}", FmtSize::new(size));
                if size < bigger * GIB {
                    let msg = format!("Nothing to do: Store size is at {} ({} below the threshold of {})",
                        FmtSize::new(size),
                        FmtSize::new(bigger * GIB - size),
                        FmtSize::new(bigger * GIB));
                    announce(msg);
                    return Ok(());
                }
            }
        }

        if let Some(quota) = self.quota {
            if quota != 0 {
                eprintln!("Calculating store size...");
                let size = Store::size()?;
                eprintln!("Store has a size of {}", FmtSize::new(size));

                let blkdev_size = files::get_blkdev_size(&Store::blkdev()?)?;
                let percentage = size * 100 / blkdev_size;
                if percentage < quota as u64 {
                    let msg = format!("Nothing to do: Device usage of store is at {} (below the threshold of {})",
                        FmtPercentage::new(size, blkdev_size),
                        FmtPercentage::new(quota as u64, 100));
                    announce(msg);
                    return Ok(());
                }
            }
        }


        if self.dry_run {
            announce("Skipping garbage collection (dry run)".green().to_string());
        } else {
            announce("Running garbage collection".green().to_string());
            if !self.interactive || ask("Do you want to perform garbage collection now?", false) {
                Store::gc()?
            }
        }

        Ok(())
    }
}
