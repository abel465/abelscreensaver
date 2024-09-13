use crate::mpvclient::MpvClient;
use crate::runner::UserEvent;
use crate::settings::Options;
use egui::{include_image, vec2, Image, Sense, Vec2};
use egui_glow::egui_winit::winit;
use std::time::{Duration, Instant};
use winit::dpi::PhysicalSize;
use winit::event_loop::EventLoopProxy;

struct ImageToggleButton {
    images: [Image<'static>; 2],
    on: bool,
}

impl ImageToggleButton {
    fn new(image_on: Image<'static>, image_off: Image<'static>, on: bool) -> Self {
        Self {
            images: [image_on, image_off],
            on,
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui) -> egui::Response {
        let image = &self.images[self.on as usize];
        ui.add_sized(Vec2::splat(25.0), image.clone())
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
    icon: Image<'static>,
    opts: Options,
    opts_copy: Options,
    open: bool,
}

impl SettingsGui {
    fn new(opts: Options) -> Self {
        Self {
            icon: Image::new(include_image!("../assets/svg/settings.svg")).sense(Sense::click()),
            opts_copy: opts.clone(),
            opts,
            open: false,
        }
    }

    fn show(
        &mut self,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
        event_proxy: &EventLoopProxy<UserEvent>,
    ) -> Option<egui::Response> {
        if ui.add(self.icon.clone()).clicked() {
            self.open = !self.open;
        }
        let size = vec2(290.0, 160.0);
        let window_size = ctx.input(|input| input.screen_rect().size());
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

    fn close_apply(&mut self, event_proxy: &EventLoopProxy<UserEvent>) {
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
    center_images: [Image<'static>; 4],
    mute_toggle_button: ImageToggleButton,
    pause_toggle_button: ImageToggleButton,
    settings_gui: SettingsGui,
    keep_visible: bool,
}

impl Overlay {
    const DURATION_HALF: Duration = Duration::from_millis(500);
    const DURATION: Duration = Duration::from_millis(1000);
    const CENTER_IMAGE_SIZE: Vec2 = Vec2::splat(200.0);

    pub fn new(size: PhysicalSize<u32>, opts: Options) -> Self {
        let center_images = [
            Image::new(include_image!("../assets/svg/mute.svg")).sense(Sense::click()),
            Image::new(include_image!("../assets/svg/unmute.svg")).sense(Sense::click()),
            Image::new(include_image!("../assets/svg/pause.svg")).sense(Sense::click()),
            Image::new(include_image!("../assets/svg/play.svg")).sense(Sense::click()),
        ];

        let mute_toggle_button = ImageToggleButton::new(
            Image::new(include_image!("./../assets/svg/unmute.svg")).sense(Sense::click()),
            Image::new(include_image!("./../assets/svg/mute.svg")).sense(Sense::click()),
            opts.mute,
        );

        let pause_toggle_button = ImageToggleButton::new(
            Image::new(include_image!("./../assets/svg/pause.svg")).sense(Sense::click()),
            Image::new(include_image!("./../assets/svg/play.svg")).sense(Sense::click()),
            false,
        );

        let inactive_instant = Instant::now() - Self::DURATION * 10;

        Self {
            path: String::new(),
            center_pos: ((vec2(size.width as f32, size.height as f32) - Self::CENTER_IMAGE_SIZE)
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
        }
    }

    pub fn ui(
        &mut self,
        ctx: &egui::Context,
        mpv_client: &MpvClient,
        event_proxy: &EventLoopProxy<UserEvent>,
    ) {
        ctx.output_mut(|output| {
            output.cursor_icon = egui::CursorIcon::Default;
        });
        if !self.has_media {
            egui::Area::new("no_media".into())
                .interactable(false)
                .fixed_pos(self.center_pos)
                .show(ctx, |ui| {
                    ui.horizontal_centered(|ui| {
                        let font_size = 36.0;
                        ui.add_sized(
                            vec2(Self::CENTER_IMAGE_SIZE.x, font_size),
                            egui::Label::new(egui::RichText::from("No Media").size(font_size)),
                        );
                    });
                });
        }
        if self.last_center_render_instant.elapsed() < Self::DURATION {
            egui::Area::new("center_area".into())
                .interactable(false)
                .fixed_pos(self.center_pos)
                .show(ctx, |ui| {
                    ui.add_sized(
                        Self::CENTER_IMAGE_SIZE,
                        self.center_images[self.center_image_index].clone(),
                    );
                });
        }
        if self.last_ui_render_instant.elapsed() < Self::DURATION_HALF || self.keep_visible {
            self.bottom_panel(ctx, mpv_client, event_proxy);
        } else if self.last_ui_render_instant.elapsed() > Self::DURATION {
            ctx.output_mut(|output| {
                output.cursor_icon = egui::CursorIcon::None;
            });
        }
    }

    fn bottom_panel(
        &mut self,
        ctx: &egui::Context,
        mpv_client: &MpvClient,
        event_proxy: &EventLoopProxy<UserEvent>,
    ) {
        let egui::InnerResponse { response, inner } = egui::TopBottomPanel::bottom("bottom_panel")
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    if self.pause_toggle_button.ui(ui).clicked() {
                        mpv_client.set_pause(self.pause_toggle_button.toggle());
                    }
                    if self.mute_toggle_button.ui(ui).clicked() {
                        mpv_client.set_mute(self.mute_toggle_button.toggle());
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let resp = self.settings_gui.show(ctx, ui, event_proxy);
                        if self.has_media {
                            self.path_label(ctx, ui);
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
        self.keep_visible = if self.settings_gui.open || response.contains_pointer() {
            self.last_ui_render_instant = Instant::now();
            true
        } else {
            false
        };
    }

    fn path_label(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        let path_label_width = egui::Area::new("phantom_path_label".into())
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
        ui.label(egui::RichText::new(&self.path).size(14.0));
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
