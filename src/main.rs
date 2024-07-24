mod gui;
mod media_iterator;
mod screensaver;

use media_iterator::media_iterator;
use std::env::current_dir;
use std::iter::Iterator;
use std::num::ParseIntError;
use std::path::PathBuf;
use std::time::Duration;
use structopt::StructOpt;

fn parse_millis(src: &str) -> Result<Duration, ParseIntError> {
    src.parse().map(Duration::from_millis)
}

#[derive(Debug, StructOpt, Clone)]
#[structopt(name = "abelscreensaver", about = "A capable screensaver.")]
pub struct Options {
    /// Randomize playback
    #[structopt(long)]
    random: bool,

    /// Include hidden entries
    #[structopt(short, long)]
    pub all: bool,

    /// Show path label
    #[structopt(short, long)]
    pub path_label: bool,

    /// Length of time (ms) for each image
    #[structopt(short, long, default_value = "4000", parse(try_from_str=parse_millis))]
    pub period: Duration,

    pub paths: Vec<PathBuf>,
}

fn main() {
    let mut opts = Options::from_args();
    if opts.paths.is_empty() {
        opts.paths.push(current_dir().unwrap());
    };
    let it = media_iterator(opts.clone());
    gui::run(opts, it);
}
