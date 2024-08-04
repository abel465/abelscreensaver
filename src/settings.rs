use egui::{pos2, vec2, Vec2};
use std::path::PathBuf;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Options {
    // Randomize playback
    pub random: bool,

    // Include hidden entries
    pub hidden: bool,

    // Include videos
    pub video: bool,

    // Mute audio
    pub mute: bool,

    // How long to show each image
    pub period_secs: f64,

    // The paths to search for media
    pub paths: Vec<PathBuf>,
}

impl Default for Options {
    fn default() -> Self {
        let users_dirs = directories::UserDirs::new().unwrap();
        Self {
            random: true,
            hidden: false,
            video: true,
            mute: false,
            period_secs: 4.0,
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
        let mut changed = egui::Frame::none()
            .inner_margin(Vec2::splat(2.0))
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing = Vec2::splat(12.0);
                egui::Grid::new("settings_grid")
                    .num_columns(2)
                    .spacing(vec2(16.0, 10.0))
                    .show(ui, |ui| {
                        let mut resp = ui.checkbox(&mut self.random, "Randomize")
                            | ui.checkbox(&mut self.hidden, "Include hidden files");
                        ui.end_row();
                        resp |= ui.checkbox(&mut self.video, "Include video")
                            | ui.add_enabled(
                                self.video,
                                egui::Checkbox::new(&mut self.mute, "Mute video"),
                            );
                        ui.end_row();
                        resp
                    })
                    .inner
                    | ui.add(egui::Slider::new(&mut self.period_secs, 0.1..=20.0).text("Period"))
            })
            .inner
            .changed();

        let focus_last_path = ui
            .horizontal(|ui| {
                ui.heading("Paths");
                let clicked = add_button(ui).clicked();
                if clicked
                    && !self
                        .paths
                        .last()
                        .is_some_and(|path| path.to_str().unwrap().is_empty())
                {
                    self.paths
                        .push(PathBuf::from(std::ffi::OsString::from(String::new())));
                }
                clicked
            })
            .inner;

        changed |= egui::Frame::none()
            .fill(egui::Color32::from_gray(20))
            .rounding(egui::Rounding::same(2.0))
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .max_height(12.0)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let length = self.paths.len();
                        let mut remove_index: Option<usize> = None;
                        let changed = (0..length).fold(false, |changed, i| {
                            let mut str = self.paths[i].to_str().unwrap().to_string();
                            ui.horizontal(|ui| {
                                let text_edit = ui.add(
                                    egui::TextEdit::singleline(&mut str)
                                        .desired_width(260.0)
                                        .margin(egui::vec2(13.0, 0.0)),
                                );
                                if text_edit.changed() {
                                    self.paths[i] = PathBuf::from(std::ffi::OsString::from(str));
                                }
                                if i == length - 1 && focus_last_path {
                                    text_edit.scroll_to_me(None);
                                    ui.memory().request_focus(text_edit.id);
                                }
                                let remove_button = remove_button(ui);
                                if remove_button.clicked() {
                                    remove_index = Some(i);
                                }
                                text_edit.changed() || remove_button.clicked()
                            })
                            .inner
                                || changed
                        });
                        if let Some(remove_index) = remove_index {
                            self.paths.remove(remove_index);
                        }
                        changed
                    })
                    .inner
            })
            .inner;
        if changed {
            self.save();
        }
    }
}

fn add_button(ui: &mut egui::Ui) -> egui::Response {
    let (rect, response) =
        ui.allocate_at_least(Vec2::splat(ui.spacing().icon_width), egui::Sense::click());
    let visuals = ui.style().interact(&response);
    let stroke = visuals.fg_stroke;
    let rect = rect.shrink(1.0).expand(visuals.expansion * 0.1);
    let p = rect.left_top();
    let d = rect.width();
    ui.painter().line_segment(
        [p + d / 2.0 * Vec2::Y, pos2(p.x + d, p.y + d / 2.0)],
        stroke,
    );
    ui.painter().line_segment(
        [p + d / 2.0 * Vec2::X, pos2(p.x + d / 2.0, p.y + d)],
        stroke,
    );
    response
}

fn remove_button(ui: &mut egui::Ui) -> egui::Response {
    let x = ui.spacing().icon_width - 3.0;
    let min_rect = ui.min_rect();
    let rect = egui::Rect {
        min: pos2(min_rect.max.x - x, min_rect.min.y + 3.0),
        max: pos2(min_rect.max.x, min_rect.max.y - 3.0),
    };
    ui.allocate_ui_at_rect(rect, |ui| {
        ui.centered_and_justified(|ui| {
            let (rect, response) = ui.allocate_at_least(Vec2::splat(x), egui::Sense::click());
            let visuals = ui.style().interact(&response);
            let stroke = visuals.fg_stroke;
            let rect = rect.shrink(1.0).expand(visuals.expansion * 0.1);
            let p = rect.left_top();
            let d = rect.width();
            ui.painter().line_segment([p, p + Vec2::splat(d)], stroke);
            ui.painter()
                .line_segment([p + d * Vec2::X, p + d * Vec2::Y], stroke);
            response
        })
        .inner
    })
    .inner
}
