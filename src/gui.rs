use crate::{Opt, INITIAL_WINDOW_SIZE};
use eframe::egui::load::SizedTexture;
use eframe::egui::{self, TextureHandle, TextureOptions, Vec2, ViewportCommand};
use image::imageops::FilterType;
use std::path::{Path, PathBuf};
use std::time::Instant;

pub struct MyApp<I: Iterator<Item = PathBuf>> {
    last_image_time: Instant,
    current_entry: Option<PathBuf>,
    next_entry: Option<PathBuf>,
    current_texture: Option<TextureHandle>,
    next_texture: Option<TextureHandle>,
    it: I,
    opts: Opt,
    window_size: Vec2,
}

impl<I: Iterator<Item = PathBuf>> MyApp<I> {
    pub fn new(opts: Opt, mut it: I) -> Self {
        let current_entry = it.next();
        let next_entry = it.next();
        println!("{:?}", current_entry);
        Self {
            last_image_time: Instant::now(),
            opts,
            current_entry,
            current_texture: None,
            next_texture: None,
            next_entry,
            it,
            window_size: INITIAL_WINDOW_SIZE,
        }
    }
}

impl<I: Iterator<Item = PathBuf>> eframe::App for MyApp<I> {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let window_size = window_size(ctx);
        if window_size != self.window_size {
            self.current_texture = get_texture(self.current_entry.as_deref(), window_size, ctx);
            self.next_texture = get_texture(self.next_entry.as_deref(), window_size, ctx);
            if self.window_size == INITIAL_WINDOW_SIZE {
                self.last_image_time = Instant::now();
            }
            self.window_size = window_size;
        }
        if window_size == INITIAL_WINDOW_SIZE {
            return;
        }
        if Instant::now() > self.last_image_time + self.opts.period {
            self.current_entry = self.next_entry.clone();
            std::mem::swap(&mut self.current_texture, &mut self.next_texture);
            self.next_entry = self.it.next();
            self.next_texture = get_texture(self.next_entry.as_deref(), window_size, ctx);
            self.last_image_time = Instant::now();
            println!("{:?}", self.current_entry);
        }
        let Some(current_texture) = &self.current_texture else {
            ctx.send_viewport_cmd(ViewportCommand::Close);
            return;
        };
        egui::CentralPanel::default()
            .frame(egui::Frame::none())
            .show(ctx, |ui| {
                ui.with_layout(
                    egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
                    |ui| {
                        ui.image(SizedTexture::from_handle(&current_texture));
                    },
                );
            });
        ctx.request_repaint_after(self.last_image_time + self.opts.period - Instant::now());
    }
}

fn get_texture(
    entry: Option<&Path>,
    window_size: Vec2,
    ctx: &egui::Context,
) -> Option<TextureHandle> {
    entry.map(|path| {
        ctx.load_texture(
            path.to_string_lossy(),
            load_image_from_path(&path, window_size).unwrap(),
            TextureOptions::default(),
        )
    })
}

fn load_image_from_path(
    path: &Path,
    Vec2 {
        x: window_w,
        y: window_h,
    }: Vec2,
) -> Result<egui::ColorImage, image::ImageError> {
    let image = image::io::Reader::open(path)?.decode()?;
    let (nwidth, nheight) = {
        let (w, h) = (image.width() as f32, image.height() as f32);
        let ratio = (window_w / w).min(window_h / h);
        ((ratio * w) as _, (ratio * h) as _)
    };
    let image_buffer = image::imageops::resize(&image, nwidth, nheight, FilterType::Nearest);
    let pixels = image_buffer.as_flat_samples();
    Ok(egui::ColorImage::from_rgba_unmultiplied(
        [nwidth as _, nheight as _],
        pixels.as_slice(),
    ))
}

fn window_size(ctx: &egui::Context) -> Vec2 {
    ctx.input(|i| i.screen_rect).max.to_vec2()
}
