use crate::settings::Options;
use crate::{mpvclient::MpvClient, runner::UserEvent};
use egui::vec2;
use egui_extras::RetainedImage;
use glutin::{dpi::PhysicalSize, event_loop::EventLoopProxy};
use std::time::{Duration, Instant};

struct ImageToggleButton {
    images: [RetainedImage; 2],
    on: bool,
}

impl ImageToggleButton {
    fn new(image_on: RetainedImage, image_off: RetainedImage, on: bool) -> Self {
        Self {
            images: [image_on, image_off],
            on,
        }
    }

    fn ui(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) -> egui::Response {
        let image = &self.images[self.on as usize];
        ui.add(
            egui::Image::new(image.texture_id(ctx), image.size_vec2()).sense(egui::Sense::click()),
        )
    }

    fn toggle(&mut self) -> bool {
        self.on = !self.on;
        self.on
    }
}

enum ImageVariants {
    Mute = 0,
    UnMute,
    Pause,
    Play,
}

struct SettingsGui {
    icon: RetainedImage,
    opts: Options,
    opts_copy: Options,
    open: bool,
}

impl SettingsGui {
    fn new(opts: Options) -> Self {
        Self {
            icon: RetainedImage::from_svg_bytes(
                "settings",
                std::include_bytes!("../assets/svg/settings.svg"),
            )
            .unwrap(),
            opts_copy: opts.clone(),
            opts,
            open: false,
        }
    }

    fn show(
        &mut self,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
        event_proxy: EventLoopProxy<UserEvent>,
    ) -> Option<egui::Response> {
        if ui
            .add(
                egui::Image::new(self.icon.texture_id(ctx), self.icon.size_vec2())
                    .sense(egui::Sense::click()),
            )
            .clicked()
        {
            self.open = !self.open;
        }
        let size = vec2(290.0, 160.0);
        let window_size = ctx.input().screen_rect().size();
        let mut open = self.open;
        let resp = egui::Window::new("Settings")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .fixed_pos((window_size - size - vec2(18.0, 110.0)).to_pos2())
            .fixed_size(size)
            .show(ctx, |ui| {
                self.opts.ui(ui);
                egui::Frame::none()
                    .show(ui, |ui| {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.button("Ok")
                        })
                        .inner
                    })
                    .inner
            });
        if self.open && !open {
            self.close_cancel();
        }
        let ok_clicked = resp
            .as_ref()
            .is_some_and(|resp| resp.inner.as_ref().unwrap().clicked());
        if ok_clicked {
            self.close_apply(event_proxy);
            self.opts.save();
        }
        resp.map(|resp| resp.response)
    }

    pub fn close_cancel(&mut self) {
        self.open = false;
        self.opts = self.opts_copy.clone();
    }

    fn close_apply(&mut self, event_proxy: EventLoopProxy<UserEvent>) {
        self.open = false;
        if self.opts != self.opts_copy {
            self.opts_copy = self.opts.clone();
            event_proxy
                .send_event(UserEvent::Reset(self.opts.clone()))
                .unwrap();
        }
    }
}

pub struct Overlay {
    pub path: String,
    pub has_media: bool,
    last_ui_render_instant: Instant,
    last_center_render_instant: Instant,
    center_pos: egui::Pos2,
    center_image_index: usize,
    center_images: [RetainedImage; 4],
    mute_toggle_button: ImageToggleButton,
    pause_toggle_button: ImageToggleButton,
    settings_gui: SettingsGui,
    keep_visible: bool,
    path_copy_instant: Instant,
}

impl Overlay {
    const DURATION_HALF: Duration = Duration::from_millis(500);
    const DURATION: Duration = Duration::from_millis(1000);

    pub fn new(size: PhysicalSize<u32>, opts: Options) -> Self {
        let mute_toggle_button = ImageToggleButton::new(
            RetainedImage::from_svg_bytes(
                "unmute",
                std::include_bytes!("../assets/svg/unmute.svg"),
            )
            .unwrap(),
            RetainedImage::from_svg_bytes("mute", std::include_bytes!("../assets/svg/mute.svg"))
                .unwrap(),
            opts.mute,
        );

        let pause_toggle_button = ImageToggleButton::new(
            RetainedImage::from_svg_bytes("play", std::include_bytes!("../assets/svg/pause.svg"))
                .unwrap(),
            RetainedImage::from_svg_bytes("pause", std::include_bytes!("../assets/svg/play.svg"))
                .unwrap(),
            false,
        );

        let center_images = [
            RetainedImage::from_svg_bytes_with_size(
                "mute-center",
                std::include_bytes!("../assets/svg/mute.svg"),
                egui_extras::image::FitTo::Zoom(6.0),
            )
            .unwrap(),
            RetainedImage::from_svg_bytes_with_size(
                "unmute-center",
                std::include_bytes!("../assets/svg/unmute.svg"),
                egui_extras::image::FitTo::Zoom(6.0),
            )
            .unwrap(),
            RetainedImage::from_svg_bytes_with_size(
                "pause-center",
                std::include_bytes!("../assets/svg/pause.svg"),
                egui_extras::image::FitTo::Zoom(6.0),
            )
            .unwrap(),
            RetainedImage::from_svg_bytes_with_size(
                "play-center",
                std::include_bytes!("../assets/svg/play.svg"),
                egui_extras::image::FitTo::Zoom(6.0),
            )
            .unwrap(),
        ];

        let inactive_instant = Instant::now() - Self::DURATION * 10;

        Self {
            path: String::new(),
            center_pos: ((vec2(size.width as f32, size.height as f32)
                - center_images[0].size_vec2())
                / 2.0)
                .to_pos2(),
            center_image_index: 0,
            center_images,
            last_ui_render_instant: inactive_instant,
            last_center_render_instant: inactive_instant,
            settings_gui: SettingsGui::new(opts),
            mute_toggle_button,
            pause_toggle_button,
            has_media: true,
            keep_visible: false,
            path_copy_instant: inactive_instant,
        }
    }

    pub fn ui(
        &mut self,
        ctx: &egui::Context,
        mpv_client: &MpvClient,
        event_proxy: EventLoopProxy<UserEvent>,
    ) {
        if self.last_ui_render_instant.elapsed() < Self::DURATION_HALF || self.keep_visible {
            self.bottom_panel(ctx, mpv_client, event_proxy);
        } else if self.last_ui_render_instant.elapsed() > Self::DURATION {
            ctx.output().cursor_icon = egui::CursorIcon::None;
        }
        if !self.has_media {
            egui::Area::new("no_media")
                .interactable(false)
                .fixed_pos(self.center_pos)
                .show(ctx, |ui| {
                    ui.label(egui::RichText::from("No Media").size(36.0));
                });
        }
        if self.last_center_render_instant.elapsed() < Self::DURATION {
            egui::Area::new("center_area")
                .interactable(false)
                .fixed_pos(self.center_pos)
                .show(ctx, |ui| {
                    self.center_images[self.center_image_index].show(ui);
                });
        }
    }

    fn bottom_panel(
        &mut self,
        ctx: &egui::Context,
        mpv_client: &MpvClient,
        event_proxy: EventLoopProxy<UserEvent>,
    ) {
        let egui::InnerResponse { response, inner } = egui::TopBottomPanel::bottom("bottom_panel")
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    if self.pause_toggle_button.ui(ctx, ui).clicked() {
                        mpv_client.set_pause(self.pause_toggle_button.toggle());
                    }
                    if self.mute_toggle_button.ui(ctx, ui).clicked() {
                        mpv_client.set_mute(self.mute_toggle_button.toggle());
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let resp = self.settings_gui.show(ctx, ui, event_proxy);
                        if self.has_media {
                            self.path_label(ctx, ui)
                        }
                        resp
                    })
                    .inner
                })
                .inner
            });
        if response.clicked_elsewhere() && inner.is_some_and(|r| r.clicked_elsewhere()) {
            self.settings_gui.close_cancel();
        }
        self.keep_visible = if self.settings_gui.open || response.hovered() {
            self.last_ui_render_instant = Instant::now();
            true
        } else {
            false
        };
    }

    fn path_label(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        let path_label_width = egui::Area::new("phantom_path_label")
            .interactable(false)
            .show(ctx, |ui| {
                ui.add_visible(
                    false,
                    egui::Label::new(egui::RichText::new(&self.path).size(14.0)),
                )
            })
            .response
            .rect
            .width();
        let available_width = ui.available_width();
        ui.add_space((available_width - path_label_width) / 2.0);
        if ui
            .add(
                egui::Label::new(egui::RichText::new(&self.path).size(14.0))
                    .sense(egui::Sense::click()),
            )
            .on_hover_text(if self.path_copy_instant.elapsed() < Self::DURATION * 2 {
                "Copied"
            } else {
                "Click to copy"
            })
            .clicked()
        {
            ui.output().copied_text = self.path.clone();
            self.path_copy_instant = Instant::now();
        }
    }

    pub fn toggle_mute(&mut self, mpv_client: &MpvClient) {
        let mute = self.mute_toggle_button.toggle();
        mpv_client.set_mute(mute);
        self.center_image_index = if mute {
            ImageVariants::Mute
        } else {
            ImageVariants::UnMute
        } as usize;
        self.last_center_render_instant = Instant::now();
    }

    pub fn toggle_pause(&mut self, mpv_client: &MpvClient) {
        let pause = self.pause_toggle_button.toggle();
        mpv_client.set_pause(pause);
        self.center_image_index = if pause {
            ImageVariants::Pause
        } else {
            ImageVariants::Play
        } as usize;
        self.last_center_render_instant = Instant::now();
    }

    pub fn reset_ui_render_instant(&mut self) {
        self.last_ui_render_instant = Instant::now();
    }

    pub fn needs_repaint(&self) -> bool {
        self.last_ui_render_instant.elapsed() < Self::DURATION
            || self.last_center_render_instant.elapsed() < Self::DURATION
    }
}
