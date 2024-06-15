mod gui;
mod media_iterator;

use eframe::egui;
use gui::MyApp;
use media_iterator::media_iterator;
use std::iter::Iterator;
use std::num::ParseIntError;
use std::path::PathBuf;
use std::time::Duration;
use structopt::StructOpt;

pub const INITIAL_WINDOW_SIZE: egui::Vec2 = egui::Vec2::splat(1.0);

fn parse_millis(src: &str) -> Result<Duration, ParseIntError> {
    src.parse().map(|x| Duration::from_millis(x))
}

#[derive(Debug, StructOpt, Clone)]
#[structopt(name = "abelscreensaver", about = "A capable screensaver.")]
pub struct Opt {
    /// Randomizes playback
    #[structopt(long)]
    random: bool,

    /// Includes hidden entries
    #[structopt(short, long)]
    pub all: bool,

    /// Length of time (ms) for each image
    #[structopt(short, long, default_value = "2000", parse(try_from_str=parse_millis))]
    pub period: Duration,

    pub paths: Vec<PathBuf>,
}

fn main() -> Result<(), eframe::Error> {
    let mut opts = Opt::from_args();
    if opts.paths.is_empty() {
        opts.paths.push(std::env::current_dir().unwrap());
    };
    let it = media_iterator(opts.clone());
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(INITIAL_WINDOW_SIZE)
            .with_fullscreen(true),
        ..Default::default()
    };
    eframe::run_native(
        "Image Viewer",
        options,
        Box::new(|_| Box::new(MyApp::new(opts, it))),
    )
}
