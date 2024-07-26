use crate::mpvclient::MpvClient;
use crate::runner::UserEvent;
use crate::Options;
use egui_extras::RetainedImage;
use glutin::dpi::PhysicalSize;
use glutin::event_loop::EventLoopProxy;
use std::time::{Duration, Instant};

struct ImageToggleButton {
    images: [RetainedImage; 2],
    first: bool,
}

impl ImageToggleButton {
    fn new(image_on: RetainedImage, image_off: RetainedImage, on: bool) -> Self {
        Self {
            images: [image_on, image_off],
            first: on,
        }
    }

    fn ui(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) -> egui::Response {
        let image = &self.images[self.first as usize];
        ui.add(
            egui::Image::new(image.texture_id(ctx), image.size_vec2()).sense(egui::Sense::click()),
        )
    }

    fn toggle(&mut self) -> bool {
        self.first = !self.first;
        self.first
    }
}

enum ImageVariants {
    Mute = 0,
    UnMute,
    Pause,
    Play,
}

pub struct Overlay {
    pub app: MpvClient,
    path: String,
    last_ui_render_instant: Instant,
    last_center_render_instant: Instant,
    center_pos: egui::Pos2,
    center_image_index: usize,
    center_images: [RetainedImage; 4],
    mute_toggle_button: ImageToggleButton,
    pause_toggle_button: ImageToggleButton,
}

impl Overlay {
    const UI_DURATION: Duration = Duration::from_millis(500);
    const CENTER_IMAGE_DURATION: Duration = Duration::from_millis(1000);

    pub fn new(app: MpvClient, size: PhysicalSize<u32>, opts: &Options) -> Self {
        let mute_toggle_button = ImageToggleButton::new(
            RetainedImage::from_svg_bytes(
                "sound-on",
                std::include_bytes!("../assets/svg/sound-on.svg"),
            )
            .unwrap(),
            RetainedImage::from_svg_bytes(
                "sound-off",
                std::include_bytes!("../assets/svg/sound-off.svg"),
            )
            .unwrap(),
            opts.mute,
        );

        let pause_toggle_button = ImageToggleButton::new(
            RetainedImage::from_svg_bytes(
                "pause-on",
                std::include_bytes!("../assets/svg/pause.svg"),
            )
            .unwrap(),
            RetainedImage::from_svg_bytes(
                "pause-off",
                std::include_bytes!("../assets/svg/play.svg"),
            )
            .unwrap(),
            false,
        );

        let center_images = [
            RetainedImage::from_svg_bytes_with_size(
                "mute-center",
                std::include_bytes!("../assets/svg/sound-off.svg"),
                egui_extras::image::FitTo::Zoom(6.0),
            )
            .unwrap(),
            RetainedImage::from_svg_bytes_with_size(
                "unmute-center",
                std::include_bytes!("../assets/svg/sound-on.svg"),
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

        Self {
            app,
            path: String::new(),
            center_pos: ((egui::vec2(size.width as f32, size.height as f32)
                - center_images[0].size_vec2())
                / 2.0)
                .to_pos2(),
            center_image_index: 0,
            center_images,
            last_ui_render_instant: Instant::now() - Self::UI_DURATION,
            last_center_render_instant: Instant::now() - Self::CENTER_IMAGE_DURATION,
            mute_toggle_button,
            pause_toggle_button,
        }
    }

    pub fn ui(&mut self, ctx: &egui::Context) {
        if self.last_ui_render_instant.elapsed() <= Self::UI_DURATION {
            egui::Area::new("path_label")
                .movable(false)
                .interactable(false)
                .fixed_pos(egui::pos2(0.0, 1418.0))
                .show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.label(egui::RichText::new(&self.path).size(14.0));
                    });
                });
            if egui::TopBottomPanel::bottom("bottom_panel")
                .show(ctx, |ui| {
                    ui.horizontal_centered(|ui| {
                        if self.pause_toggle_button.ui(ctx, ui).clicked() {
                            self.app.set_pause(self.pause_toggle_button.toggle());
                        }
                        if self.mute_toggle_button.ui(ctx, ui).clicked() {
                            self.app.set_mute(self.mute_toggle_button.toggle());
                        }
                    });
                })
                .response
                .hover_pos()
                .is_some()
            {
                self.last_ui_render_instant = Instant::now();
            };
        }
        if self.last_center_render_instant.elapsed() <= Self::CENTER_IMAGE_DURATION {
            egui::Area::new("center_area")
                .movable(false)
                .interactable(false)
                .fixed_pos(self.center_pos)
                .show(ctx, |ui| {
                    self.center_images[self.center_image_index].show(ui);
                });
        }
    }

    pub fn toggle_mute(&mut self, event_proxy: EventLoopProxy<UserEvent>) {
        let mute = self.mute_toggle_button.toggle();
        self.app.set_mute(mute);
        self.center_image_index = if mute {
            ImageVariants::Mute
        } else {
            ImageVariants::UnMute
        } as usize;
        self.set_overlay_repaint_timer(event_proxy)
    }

    pub fn toggle_pause(&mut self, event_proxy: EventLoopProxy<UserEvent>) {
        let pause = self.pause_toggle_button.toggle();
        self.app.set_pause(pause);
        self.center_image_index = if pause {
            ImageVariants::Pause
        } else {
            ImageVariants::Play
        } as usize;
        self.set_overlay_repaint_timer(event_proxy)
    }

    fn set_overlay_repaint_timer(&mut self, event_proxy: EventLoopProxy<UserEvent>) {
        self.last_center_render_instant = Instant::now();
        std::thread::spawn(move || {
            std::thread::sleep(Self::CENTER_IMAGE_DURATION);
            event_proxy.send_event(UserEvent::RequestRedraw).unwrap();
        });
    }

    pub fn set_ui_repaint_timer(&mut self, event_proxy: EventLoopProxy<UserEvent>) {
        self.last_ui_render_instant = Instant::now();
        std::thread::spawn(move || {
            std::thread::sleep(Self::UI_DURATION);
            event_proxy.send_event(UserEvent::RequestRedraw).unwrap();
        });
    }

    pub fn set_path(&mut self) {
        self.path = self.app.get_path();
    }
}
