mod media_iterator;
mod mpvclient;
mod options;
mod overlay;
mod runner;
mod settings;

use crate::options::Options;
use media_iterator::media_iterator;
use std::env::current_dir;
use structopt::StructOpt;

fn main() {
    let mut opts = Options::from_args();
    if opts.paths.is_empty() {
        opts = Options::load();
    };
    let it = media_iterator(opts.clone());
    runner::run(opts, it);
}
