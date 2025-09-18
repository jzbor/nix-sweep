use std::path;

use colored::Colorize;

use crate::config::ConfigPreset;
use crate::utils::fmt::FmtWithEllipsis;
use crate::HashMap;


#[derive(clap::Args)]
pub struct PresetsCommand {
    /// Alternative config file
    #[clap(short('C'), long)]
    config: Option<path::PathBuf>,

    /// Only print the names
    #[clap(long)]
    names: bool,

    #[command(flatten)]
    queries: Queries,
}

#[derive(clap::Args, Clone)]
#[group(required = true, multiple = false)]
pub struct Queries {
    #[clap(short, long)]
    list: bool,

    #[clap(short, long)]
    show: Option<String>,

    #[clap(short('a'), long)]
    show_all: bool,
}

impl super::Command for PresetsCommand {
    fn run(self) -> Result<(), String> {

        if self.queries.list {
            let mut presets: Vec<_> = ConfigPreset::available(self.config.as_ref())?.into_iter().collect();
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
        }

        if let Some(preset_name) = self.queries.show {
            let preset = ConfigPreset::load(&preset_name, self.config.as_ref())?;
            let mut with_name = HashMap::default();
            with_name.insert(preset_name, preset);
            let pretty = toml::to_string_pretty(&with_name)
                .map_err(|e| e.to_string())?;
            println!("{}", pretty);
            return Ok(());
        }

        if self.queries.show_all {
            let all = ConfigPreset::load_all(self.config.as_ref())?;
            let pretty = toml::to_string_pretty(&all)
                .map_err(|e| e.to_string())?;
            println!("{}", pretty);
            return Ok(());
        }

        Ok(())
    }
}
