pub mod add_root;
pub mod analyze;
pub mod cleanout;
pub mod completions;
pub mod gc;
pub mod gc_roots;
pub mod generations;
pub mod man;
pub mod path_info;
pub mod tidyup_gc_roots;
pub mod list_presets;

pub trait Command: clap::Args {
    fn run(self) -> Result<(), String>;
}
