mod media_iterator;
mod mpvclient;
mod overlay;
mod runner;
mod settings;

use crate::settings::Options;
use std::fs::File;
use std::io::prelude::Write;
use std::path::PathBuf;

fn main() {
    ensure_ffprobe_exists();
    runner::run(Options::load(), black_pixel_path());
}

fn ensure_ffprobe_exists() {
    if let Err(ffprobe::FfProbeError::Io(err)) = ffprobe::ffprobe("") {
        if err.kind() == std::io::ErrorKind::NotFound {
            panic!("ffprobe: command not found");
        }
    }
}

/// Writes a black pixel to the temp filesystem and returns the path
fn black_pixel_path() -> PathBuf {
    let temp_dir = std::env::temp_dir().join("abel_screensaver/");
    std::fs::create_dir_all(&temp_dir).unwrap();
    let black_pixel_path = temp_dir.join("black_pixel.pbm");
    let mut file = File::create(&black_pixel_path).unwrap();
    file.write_all(b"P1\n1 1\n1").unwrap();
    black_pixel_path
}
