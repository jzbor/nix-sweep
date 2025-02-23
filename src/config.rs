use clap::Parser;

/// If no action (--list, --rm, --interactive, --gc) is given the program defaults to --interactive.
///
/// If no profile type (--home, --user, --system) is given the program defaults to --user.
#[derive(Parser, Debug)]
#[command(version, about, long_about)]
pub struct Config {
    /// Only list generations with their age, don't remove them
    #[clap(long)]
    pub list: bool,

    /// Remove generations
    #[clap(long)]
    pub rm: bool,

    /// Ask for confirmation before starting removal
    #[clap(short, long)]
    pub interactive: bool,

    /// Run nix garbage collection afterwards
    #[clap(long)]
    pub gc: bool,

    /// Delete generations older than <OLDER> days
    #[clap(short, long, default_value = "30")]
    pub older: u64,

    /// Keep at least <KEEP> generations
    ///
    /// This takes precedence over --older and --max.
    #[clap(short, long, default_value = "10")]
    pub keep: usize,

    /// Keep at most <MAX> generations
    #[clap(short, long)]
    pub max: Option<usize>,

    /// Apply to the home-manager profile
    #[clap(long)]
    pub home: bool,

    /// Apply to the default user profile
    #[clap(short, long)]
    pub user: bool,

    /// Apply to the system profile
    #[clap(short, long)]
    pub system: bool,
}
