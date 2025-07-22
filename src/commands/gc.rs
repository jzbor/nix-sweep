use colored::Colorize;

use crate::gc;
use crate::interaction::ask;


#[derive(clap::Args)]
pub struct GCCommand {
    /// Ask before running garbage collection
    #[clap(short('n'), long("non-interactive"), action = clap::ArgAction::SetFalse)]  // this is very confusing, but works
    interactive: bool,

    /// Do not ask before running garbage collection
    #[clap(short('i'), long("interactive"), overrides_with = "interactive")]
    _non_interactive: bool,

    /// Don't actually run garbage collection
    #[clap(short, long)]
    dry_run: bool,
}

impl GCCommand {
    pub fn new(interactive: bool, dry_run: bool) -> Self {
        GCCommand { interactive, dry_run, _non_interactive: !interactive }
    }
}

impl super::Command for GCCommand {
    async fn run(self) -> Result<(), String> {
        if self.dry_run {
            println!("\n{}", "=> Skipping garbage collection (dry run)".green());
        } else {
            println!("\n{}", "=> Running garbage collection".green());
            if !self.interactive || ask("Do you want to perform garbage collection now?", false) {
                gc::gc()?
            }
        }

        Ok(())
    }
}
