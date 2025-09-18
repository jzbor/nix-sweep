use std::path;

use colored::Colorize;

use crate::config::ConfigPreset;
use crate::utils::fmt::FmtWithEllipsis;


#[derive(clap::Args)]
pub struct ListPresetsCommand {
    /// Alternative config file
    #[clap(short('C'), long)]
    config: Option<path::PathBuf>,

    /// Only print the names
    #[clap(long)]
    names: bool,

}

impl super::Command for ListPresetsCommand {
    fn run(self) -> Result<(), String> {
        let mut presets: Vec<_> = ConfigPreset::available(self.config)?.into_iter().collect();
        presets.sort();

        if self.names {
            presets.iter()
                .for_each(|(name, _)| println!("{name}"));
        } else {
            let preset_len = presets.iter()
                .map(|(p, _)| p.len())
                .max()
                .unwrap_or(0);
            let list_len = presets.iter()
                .map(|(_, s)| s.iter().map(|e| e.len() + 2).sum::<usize>() - 2)
                .max()
                .unwrap_or(0);
            for (preset, sources) in presets {
                println!("{}  {}",
                    FmtWithEllipsis::fitting_terminal(preset, preset_len, list_len + 4)
                        .right_pad(),
                    format!("({})", sources.join(",")).bright_black(),
                );
            }
        }

        Ok(())
    }
}
