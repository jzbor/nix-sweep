use clap::Parser;
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Config {
    /// Delete only generations older than <OLDER> days
    #[clap(short, long, default_value = "30")]
    pub older: Option<usize>,

    /// Keep at least <KEEP> generations
    #[clap(short, long, default_value = "10")]
    pub keep: Option<usize>,

    /// Only list generations with their age don't remove them
    #[clap(long)]
    pub list: bool,

    /// Run nix garbage collection afterwards
    #[clap(long)]
    pub gc: bool,
}
