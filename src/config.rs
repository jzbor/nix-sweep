use clap::Parser;
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Config {
    /// Only list generations with their age don't remove them
    #[clap(long)]
    pub list: bool,

    /// Run nix garbage collection afterwards
    #[clap(long)]
    pub gc: bool,
}
