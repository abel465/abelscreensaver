mod media_iterator;
mod mpvclient;
mod overlay;
mod runner;
mod settings;

use crate::settings::Options;
use media_iterator::media_iterator;

fn main() {
    let opts = Options::load();
    let it = media_iterator(opts.clone());
    runner::run(opts, it);
}
