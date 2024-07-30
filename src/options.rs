use std::iter::Iterator;
use std::num::ParseIntError;
use std::path::PathBuf;
use std::time::Duration;
use structopt::StructOpt;

fn parse_millis(src: &str) -> Result<Duration, ParseIntError> {
    src.parse().map(Duration::from_millis)
}

#[derive(Debug, StructOpt, Clone, serde::Serialize, serde::Deserialize)]
#[structopt(name = "abelscreensaver", about = "A capable screensaver.")]
pub struct Options {
    /// Randomize playback
    #[structopt(long)]
    pub random: bool,

    /// Include hidden entries
    #[structopt(short, long)]
    pub all: bool,

    /// Mute audio
    #[structopt(short, long)]
    pub mute: bool,

    /// Length of time (ms) for each image
    #[structopt(short, long, default_value = "4000", parse(try_from_str=parse_millis))]
    pub period: Duration,

    /// The paths to search for media
    /// If empty, all options
    pub paths: Vec<PathBuf>,
}
