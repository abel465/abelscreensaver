mod media_iterator;

use media_iterator::media_iterator;
use std::{env, path::PathBuf};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "abelscreensaver", about = "A capable screensaver.")]
pub struct Opt {
    /// Randomizes playback
    #[structopt(long)]
    random: bool,

    /// Includes hidden entries
    #[structopt(short, long)]
    pub all: bool,

    pub paths: Vec<PathBuf>,
}

fn main() {
    let mut opts = Opt::from_args();
    if opts.paths.is_empty() {
        opts.paths.push(env::current_dir().unwrap());
    };
    let it = media_iterator(opts);
    for entry in it {
        println!("{:?}", &entry);
    }
}
