use std::fs;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;

use clap::Parser;
use serde::{Deserialize, Serialize};

use crate::HashMap;


const SYSTEM_CONFIG: &str = "/etc/nix-sweep/presets.toml";
const APP_PREFIX: &str = "nix-sweep";
const CONFIG_FILENAME: &str = "presets.toml";
pub const DEFAULT_PRESET: &str = "default";


#[derive(Debug, Deserialize)]
pub struct ConfigFile(HashMap<String, ConfigPreset>);

#[derive(Clone, Debug, Serialize, Deserialize, Parser)]
#[serde(rename_all = "kebab-case")]
pub struct ConfigPreset {
    /// Keep at least this many generations
    ///
    /// Pass 0 to unset this option.
    #[clap(long)]
    pub keep_min: Option<usize>,

    /// Keep at most this many generations
    ///
    /// Pass 0 to unset this option.
    #[clap(long)]
    pub keep_max: Option<usize>,

    /// Keep all generations newer than this many days
    ///
    /// Pass 0 to unset this option.
    #[clap(long, value_parser = |s: &str| duration_str::parse_std(s))]
    #[serde(default, deserialize_with = "duration_str::deserialize_option_duration")]
    pub keep_newer: Option<Duration>,

    /// Discard all generations older than this many days
    ///
    /// Pass 0 to unset this option.
    #[clap(long, value_parser = |s: &str| duration_str::parse_std(s))]
    #[serde(default, deserialize_with = "duration_str::deserialize_option_duration")]
    pub remove_older: Option<Duration>,

    /// Remove these specific generations
    ///
    /// You can pass the option multiple times to remove multiple generations.
    #[clap(short, long("generation"), id = "GENERATION")]
    #[serde(skip)]
    pub generations: Vec<usize>,

    /// Do not ask before removing generations or running garbage collection
    #[clap(short('n'), long("non-interactive"), action = clap::ArgAction::SetFalse)]  // this is very confusing, but works
    pub interactive: Option<bool>,

    /// Ask before removing generations or running garbage collection
    #[clap(short('i'), long("interactive"), overrides_with = "interactive", action = clap::ArgAction::SetTrue)]
    #[serde(skip_serializing)]
    pub _non_interactive: Option<bool>,

    /// Run GC afterwards
    #[clap(long, action = clap::ArgAction::SetTrue)]
    pub gc: Option<bool>,

    /// Only perform gc if the store is bigger than BIGGER Gibibytes.
    #[clap(long)]
    pub gc_bigger: Option<u64>,

    /// Only perform gc if the store uses more than QUOTA% of its device.
    #[clap(long, value_parser=clap::value_parser!(u64).range(0..100))]
    pub gc_quota: Option<u64>,

    /// Collect just as much garbage as to match --gc-bigger or --gc-quota
    #[clap(long)]
    #[serde(default)]
    pub gc_modest: bool,
}

impl ConfigFile {
    fn from_str(s: &str) -> Result<Self, String> {
        let config: Self = toml::from_str(s)
            .map_err(|e| e.to_string())?;

        for preset in config.0.values() {
            preset.validate()?;
        }

        Ok(config)
    }

    pub fn read_config_file(path: &PathBuf) -> Result<ConfigFile, String> {
        let s = fs::read_to_string(path)
            .map_err(|e| e.to_string())?;
        Self::from_str(&s)
    }

    fn get_config(path: &PathBuf) -> Result<Option<ConfigFile>, String> {
        if fs::exists(path).map_err(|e| e.to_string())? {
            Self::read_config_file(path).map(Some)
        } else {
            Ok(None)
        }

    }

    fn get_system_config() -> Result<Option<ConfigFile>, String> {
        let path = PathBuf::from_str(SYSTEM_CONFIG)
            .map_err(|e| e.to_string())?;
        Self::get_config(&path)
    }

    fn get_user_config() -> Result<Option<ConfigFile>, String> {
        xdg::BaseDirectories::with_prefix(APP_PREFIX)
            .get_config_file(CONFIG_FILENAME)
            .ok_or(String::from("Unable to open config file"))
            .and_then(|d| Self::get_config(&d))
    }

    fn get_preset(&self, s: &str) -> Option<&ConfigPreset> {
        self.0.get(s)
    }
}

impl ConfigPreset {
    pub fn load(preset_name: &str, custom_config_file: Option<PathBuf>) -> Result<ConfigPreset, String> {
        let system_config = ConfigFile::get_system_config()?;
        let user_config = ConfigFile::get_user_config()?;
        let custom_config = match custom_config_file {
            Some(path) => Some(ConfigFile::read_config_file(&path)?),
            None => None,
        };

        let system_named_preset = system_config.as_ref()
            .and_then(|c| c.get_preset(preset_name));
        let user_named_preset = user_config.as_ref()
            .and_then(|c| c.get_preset(preset_name));
        let custom_named_preset = custom_config.as_ref()
            .and_then(|c| c.get_preset(preset_name));

        if system_named_preset.is_none()
                && user_named_preset.is_none()
                && custom_named_preset.is_none()
                && preset_name != DEFAULT_PRESET {
            return Err(format!("Could not find preset '{preset_name}'"));
        }

        let preset = Self::default()
            .override_with_opt(system_named_preset)
            .override_with_opt(user_named_preset)
            .override_with_opt(custom_named_preset)
            .finalize();

        Ok(preset)
    }

    pub fn validate(&self) -> Result<(), String> {
        if let (Some(min), Some(max)) = (self.keep_min, self.keep_max)
            && min > max {
                return Err("Invalid configuration - keep-min is greater than keep-max".to_owned());
            }

        if let (Some(newer), Some(older)) = (self.keep_newer, self.remove_older)
            && newer > older {
                return Err("Invalid configuration - keep-newer is greater than remove-older".to_owned());
            }

        Ok(())
    }

    pub fn override_with(&self, other: &ConfigPreset) -> Self {
        let mut keep_min = match (self.keep_min, other.keep_min) {
            (None, None) => None,
            (_, Some(0)) => None,
            (_, Some(val)) => Some(val),
            (Some(val), None) => Some(val),
        };

        let mut keep_max = match (self.keep_max, other.keep_max) {
            (None, None) => None,
            (_, Some(0)) => None,
            (_, Some(val)) => Some(val),
            (Some(val), None) => Some(val),
        };

        let mut keep_newer = match (self.keep_newer, other.keep_newer) {
            (None, None) => None,
            (_, Some(Duration::ZERO)) => None,
            (_, Some(val)) => Some(val),
            (Some(val), None) => Some(val),
        };

        let mut remove_older = match (self.remove_older, other.remove_older) {
            (None, None) => None,
            (_, Some(Duration::ZERO)) => None,
            (_, Some(val)) => Some(val),
            (Some(val), None) => Some(val),
        };

        let interactive = match (self.interactive, other.interactive) {
            (None, None) => None,
            (_, Some(val)) => Some(val),
            (Some(val), None) => Some(val),
        };

        let gc = match (self.gc, other.gc) {
            (None, None) => None,
            (_, Some(val)) => Some(val),
            (Some(val), None) => Some(val),
        };

        let gc_bigger = match (self.gc_bigger, other.gc_bigger) {
            (None, None) => None,
            (_, Some(val)) => Some(val),
            (Some(val), None) => Some(val),
        };

        let gc_quota = match (self.gc_quota, other.gc_quota) {
            (None, None) => None,
            (_, Some(val)) => Some(val),
            (Some(val), None) => Some(val),
        };



        if keep_min > keep_max && keep_min.is_some() && keep_max.is_some() {
            if other.keep_min.is_none() {
                keep_min = keep_max;
            } else if other.keep_max.is_none() {
                keep_max = keep_min;
            } else {
                panic!("Inconsistent config after load (keep_min: {keep_min:?}, keep_max: {keep_max:?})");
            }
        }

        if keep_newer > remove_older && keep_newer.is_some() && remove_older.is_some(){
            if other.keep_newer.is_none() {
                keep_newer = remove_older;
            } else if other.keep_max.is_none() {
                remove_older = keep_newer;
            } else {
                panic!("Inconsistent config after load (keep_newer: {keep_newer:?}, remove_older: {remove_older:?})");
            }
        }

        let gc_modest = self.gc_modest || other.gc_modest;

        ConfigPreset {
            keep_min, keep_max, keep_newer, remove_older,
            interactive, _non_interactive: None,
            gc, gc_bigger, gc_quota, gc_modest,
            generations: other.generations.clone(),
        }
    }

    pub fn override_with_opt(&self, other: Option<&ConfigPreset>) -> Self {
        if let Some(preset) = other {
            self.override_with(preset)
        } else {
            (*self).clone()
        }
    }

    fn finalize(&self) -> Self {
        ConfigPreset {
            keep_min: if let Some(0) = self.keep_min { None } else { self.keep_min },
            keep_max: if let Some(0) = self.keep_max { None } else { self.keep_max },
            keep_newer: if let Some(Duration::ZERO) = self.keep_newer { None } else { self.keep_newer },
            remove_older: if let Some(Duration::ZERO) = self.remove_older { None } else { self.remove_older },
            interactive: self.interactive,
            _non_interactive: None,
            gc: self.gc,
            gc_bigger: if let Some(0) = self.gc_bigger { None } else { self.gc_bigger },
            gc_quota: if let Some(0) = self.gc_quota { None } else { self.gc_quota },
            gc_modest: self.gc_modest,
            generations: self.generations.clone(),
        }
    }
}

impl Default for ConfigPreset {
    fn default() -> Self {
        ConfigPreset {
            keep_min: Some(1),
            keep_max: None,
            keep_newer: None,
            remove_older: None,
            interactive: None,
            _non_interactive: None,
            gc: None,
            gc_bigger: None,
            gc_quota: None,
            gc_modest: false,
            generations: Vec::default(),
        }
    }
}
