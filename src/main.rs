mod media_iterator;
mod mpvclient;
mod overlay;
mod runner;
mod settings;

use crate::settings::Options;
use std::fs::File;
use std::io::prelude::Write;

fn main() {
    let temp_dir = std::env::temp_dir().join("abel_screensaver/");
    std::fs::create_dir_all(&temp_dir).unwrap();
    let black_pixel_path = temp_dir.join("black_pixel.pbm");
    let mut file = File::create(&black_pixel_path).unwrap();
    file.write_all(std::include_bytes!("../assets/black_pixel.pbm"))
        .unwrap();
    let opts = Options::load();
    runner::run(opts, black_pixel_path);
}
