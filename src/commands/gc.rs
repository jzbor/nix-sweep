use crate::utils::files;
use crate::utils::fmt::{FmtPercentage, FmtSize};
use crate::utils::interaction::{announce, ask};
use crate::nix::store::Store;


const GIB: u64 = 1024 * 1024 * 1024;


#[derive(clap::Args)]
pub struct GCCommand {
    /// Do not ask before running garbage collection
    #[clap(short('n'), long("non-interactive"), action = clap::ArgAction::SetFalse)]  // this is very confusing, but works
    interactive: bool,

    /// Ask before running garbage collection
    #[clap(short('i'), long("interactive"), overrides_with = "interactive")]
    _non_interactive: bool,

    /// Only perform gc if the store is bigger than BIGGER Gibibytes.
    #[clap(short, long)]
    bigger: Option<u64>,

    /// Only perform gc if the store uses more than QUOTA% of its device.
    #[clap(short, long, value_parser=clap::value_parser!(u64).range(1..100))]
    quota: Option<u64>,

    /// Don't actually run garbage collection
    #[clap(short, long)]
    dry_run: bool,

    /// Collect just as much garbage as to match --bigger or --quota
    ///
    /// The desired target size of the store is calculated based on --bigger or --quota and then
    /// rewritten to match the --max-freed option of nix-store(1). Garbage collection is then
    /// performed stopping, as soon as the desired target size is met.
    #[clap(short, long)]
    modest: bool,
}

impl GCCommand {
    pub fn new(interactive: bool, dry_run: bool, bigger: Option<u64>, quota: Option<u64>, modest: bool) -> Self {
        GCCommand { interactive, dry_run, bigger, quota, _non_interactive: !interactive, modest }
    }
}

impl super::Command for GCCommand {
    fn run(self) -> Result<(), String> {
        announce("Starting garbage collection".to_owned());
        if let Some(bigger) = self.bigger {
            eprintln!("Calculating store size...");
            let size = Store::size()?;
            eprintln!("Store has a size of {} (threshold: {})", FmtSize::new(size), FmtSize::new(bigger * GIB));
            if size <= bigger * GIB {
                let msg = format!("Nothing to do: Store size is at {} ({} below the threshold of {})",
                    FmtSize::new(size),
                    FmtSize::new(bigger * GIB - size),
                    FmtSize::new(bigger * GIB));
                eprintln!("\n-> {msg}");
                return Ok(());
            }
        }

        if let Some(quota) = self.quota {
            eprintln!("Calculating store size...");
            let size = Store::size()?;
            let blkdev_size = files::get_blkdev_size(&Store::blkdev()?)?;
            let percentage = size * 100 / blkdev_size;
            eprintln!("Store uses {percentage}% (quota: {quota}%)");
            if percentage <= quota {
                let msg = format!("Nothing to do: Device usage of store is at {} (below the threshold of {})",
                    FmtPercentage::new(size, blkdev_size),
                    FmtPercentage::new(quota, 100));
                eprintln!("\n-> {msg}");
                return Ok(());
            }
        }

        let max_freed = if self.modest {
            if let Some(bigger) = self.bigger {
                Some(Store::size()? - bigger * GIB)
            } else if let Some(quota) = self.quota {
                let blkdev_size = files::get_blkdev_size(&Store::blkdev()?)?;
                Some(Store::size()? - quota * blkdev_size / 100)
            } else {
                return Err("Cannot use --modest without --bigger or --quota being".to_owned());
            }
        } else {
            None
        };

        if let Some(bytes) = max_freed {
            eprintln!("Freeing up to {} (--modest)", FmtSize::new(bytes));
        }

        if self.dry_run {
            eprintln!("\n-> Skipping garbage collection (dry run)");
        } else if !self.interactive || ask("\nDo you want to perform garbage collection now?", false) {
            eprintln!("Starting garbage collector");
            Store::gc(max_freed)?
        }

        Ok(())
    }
}
