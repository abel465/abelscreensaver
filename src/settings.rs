use std::iter::Iterator;
use std::num::ParseIntError;
use std::path::PathBuf;
use std::time::Duration;
use structopt::StructOpt;

fn parse_millis(src: &str) -> Result<Duration, ParseIntError> {
    src.parse().map(Duration::from_millis)
}

/// dshfgakjdsh gfksjdahg kjsdahg
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

impl Default for Options {
    fn default() -> Self {
        let users_dirs = directories::UserDirs::new().unwrap();
        Self {
            random: true,
            all: false,
            mute: true,
            period: Duration::from_secs(4),
            paths: vec![users_dirs.picture_dir().unwrap().to_path_buf()],
        }
    }
}

impl Options {
    pub fn load() -> Self {
        let project_dirs = directories::ProjectDirs::from("", "", "abelscreensaver").unwrap();
        let config_dir = project_dirs.config_dir();
        std::fs::create_dir_all(config_dir).unwrap();
        let file = std::fs::File::open(config_dir.join("config.json"));
        if let Ok(file) = file {
            serde_json::from_reader(file).unwrap()
        } else {
            let result = Self::default();
            result.save();
            result
        }
    }

    fn save(&self) {
        let serialized = serde_json::to_string_pretty(self).unwrap();
        let project_dirs = directories::ProjectDirs::from("", "", "abelscreensaver").unwrap();
        let config_dir = project_dirs.config_dir();
        std::fs::create_dir_all(config_dir).unwrap();
        std::fs::write(config_dir.join("config.json"), serialized).unwrap();
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        let resp = ui.checkbox(&mut self.random, "Randomize")
            | ui.checkbox(&mut self.all, "Include hidden files")
            | ui.checkbox(&mut self.mute, "Mute")
            | {
                let mut period_secs = self.period.as_secs_f64();
                let resp = ui.add(
                    egui::Slider::new(&mut period_secs, 0.1..=10.0)
                        .clamp_to_range(false)
                        .text("Period"),
                );
                if resp.changed() {
                    dbg!(&resp);
                }
                if resp.changed() {
                    self.period = Duration::from_secs_f64(period_secs);
                }
                resp
            };
        let mut set_last_path_focus = false;

        ui.horizontal(|ui| {
            ui.heading("Paths");
            let (rect, response) = ui.allocate_at_least(
                egui::Vec2::splat(ui.spacing().icon_width),
                egui::Sense::click(),
            );
            let visuals = ui.style().interact(&response);
            let stroke = visuals.fg_stroke;
            let rect = rect.shrink(1.0).expand(visuals.expansion * 0.1);
            let p = rect.left_top();
            let d = rect.width();
            ui.painter().line_segment(
                [
                    egui::pos2(p.x, p.y + d / 2.0),
                    egui::pos2(p.x + d, p.y + d / 2.0),
                ],
                stroke,
            );
            ui.painter().line_segment(
                [
                    egui::pos2(p.x + d / 2.0, p.y),
                    egui::pos2(p.x + d / 2.0, p.y + d),
                ],
                stroke,
            );
            if response.clicked() {
                set_last_path_focus = true;
                if !self
                    .paths
                    .last()
                    .is_some_and(|path| path.to_str().unwrap().is_empty())
                {
                    self.paths
                        .push(PathBuf::from(std::ffi::OsString::from(String::new())));
                }
            }
        });

        let changed = egui::Frame::none()
            .fill(egui::Color32::from_gray(20))
            .rounding(egui::Rounding::same(2.0))
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .stick_to_bottom(true)
                    .max_height(12.0)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let mut add_textedit = |path: &mut PathBuf| {
                            let mut str = path.to_str().unwrap().to_string();
                            let resp = ui.text_edit_singleline(&mut str);
                            if resp.changed() {
                                *path = PathBuf::from(std::ffi::OsString::from(str));
                            }
                            resp
                        };
                        let (last_path, paths) = self.paths.split_last_mut().unwrap();
                        let changed = paths.iter_mut().fold(false, |changed, path| {
                            changed || add_textedit(path).changed()
                        });
                        let resp = add_textedit(last_path);
                        if set_last_path_focus {
                            resp.scroll_to_me(None);
                            ui.memory().request_focus(resp.id);
                        }
                        changed || resp.changed()
                    })
                    .inner
            })
            .inner;
        if resp.changed() || changed {
            self.save();
        }
    }
}
