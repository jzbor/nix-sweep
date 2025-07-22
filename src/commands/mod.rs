pub mod analyze;
pub mod cleanout;
pub mod gc;
pub mod gc_roots;
pub mod generations;
pub mod man;
pub mod path_info;
pub mod tidyup_gc_roots;

pub trait Command: clap::Args {
    fn run(self) -> Result<(), String>;
}
