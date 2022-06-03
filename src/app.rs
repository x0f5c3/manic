use clap::{Parser, Subcommand};
use clap_verbosity_flag::{LogLevel, Verbosity};
use std::path::PathBuf;

#[derive(Debug, Parser)]
pub(crate) struct App {
    #[clap(short, long)]
    pub(crate) threads: Option<u8>,
    #[clap(short, long, flatten)]
    pub(crate) verbose: Verbosity,
    #[clap(short, long, parse(try_from_str))]
    pub(crate) output: Option<PathBuf>,
    #[clap(min_values(1), required(true))]
    pub(crate) urls: Vec<String>,
}

// TODO add commands for sending, receiving, starting a relay server and downloading files through HTTP
#[derive(Debug, Subcommand)]
pub(crate) enum Commands {
    Completions { shell: clap_complete_command::Shell },
}

impl App {
    pub(crate) fn new() -> Self {
        Self::from_args()
    }
    pub(crate) fn init_logging(&self) {
        pretty_env_logger::formatted_builder()
            .filter_level(self.verbose.log_level_filter())
            .init()
    }
}
