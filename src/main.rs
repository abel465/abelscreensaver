mod media_iterator;
mod mpvclient;
mod overlay;
mod runner;
mod settings;

use crate::settings::Options;

fn main() {
    let opts = Options::load();
    runner::run(opts);
}
