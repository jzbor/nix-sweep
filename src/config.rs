use clap::Parser;
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Config {
    /// Only list generations with their age, don't remove them
    #[clap(long)]
    pub list: bool,

    /// Remove generations
    #[clap(long)]
    pub rm: bool,

    /// Run nix garbage collection afterwards
    #[clap(long)]
    pub gc: bool,

    /// Ask for confirmation before starting removal
    #[clap(short, long)]
    pub interactive: bool,

    /// Delete only generations older than <OLDER> days
    #[clap(short, long, default_value = "30")]
    pub older: u64,

    /// Keep at least <KEEP> generations
    #[clap(short, long, default_value = "10")]
    pub keep: usize,

    /// Keep at most <MAX> generations
    #[clap(short, long)]
    pub max: Option<usize>,

    /// Apply to the system profile
    #[clap(short, long)]
    pub system: bool,
}
