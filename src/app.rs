// use clap::{Parser, Subcommand};
// use clap_verbosity_flag::Verbosity;
// use std::path::PathBuf;
//
// #[derive(Parser)]
// pub(crate) struct App {
//     #[clap(short, long)]
//     pub(crate) threads: Option<u8>,
//     #[clap(flatten)]
//     pub(crate) verbose: Verbosity,
//     #[clap(short, long, parse(try_from_str))]
//     pub(crate) output: Option<PathBuf>,
//     #[clap(min_values(1), required(true))]
//     pub(crate) urls: Vec<String>,
// }
//
// // TODO add commands for sending, receiving, starting a relay server and downloading files through HTTP
// #[derive(Subcommand)]
// pub(crate) enum Commands {
//     Completions { shell: clap_complete_command::Shell },
//     Relay,
//     Send,
//     Receive,
// }
//
// impl App {
//     pub(crate) fn new() -> Self {
//         Self::parse_from(wild::args_os())
//     }
//     pub(crate) fn init_logging(&self) {
//         pretty_env_logger::formatted_builder()
//             .filter_level(self.verbose.log_level_filter())
//             .init()
//     }
// }
