use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
pub(crate) struct App {
    #[structopt(short, long)]
    pub(crate) threads: Option<u8>,
    #[structopt(short, long)]
    pub(crate) debug: bool,
    #[structopt(short, long, parse(try_from_str))]
    pub(crate) output: Option<PathBuf>,
    #[structopt(min_values(1), required(true))]
    pub(crate) urls: Vec<String>,
}

impl App {
    pub(crate) fn new() -> Self {
        Self::from_args()
    }
}
