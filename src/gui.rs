use crate::{Opt, INITIAL_WINDOW_SIZE};
use eframe::egui::{self, ColorImage, TextureHandle, TextureOptions, Vec2, ViewportCommand};
use image::codecs::gif::GifDecoder;
use image::imageops::FilterType;
use image::{AnimationDecoder, GenericImageView};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

struct Gif {
    frames: Vec<GifFrame>,
    duration: Duration,
}

impl Gif {
    fn new(frames: Vec<GifFrame>, duration: Duration) -> Self {
        Self { frames, duration }
    }
}

struct GifFrame {
    texture: TextureHandle,
    duration: Duration,
}

impl GifFrame {
    fn new(texture: TextureHandle, duration: Duration) -> Self {
        Self { texture, duration }
    }
}

enum Image {
    Static(TextureHandle),
    Gif(Gif),
}

pub struct MyApp<I: Iterator<Item = PathBuf>> {
    last_image_time: Instant,
    opts: Opt,
    current_entry: Option<PathBuf>,
    next_entry: Option<PathBuf>,
    current_image: Option<Image>,
    next_image: Option<Image>,
    current_duration: Option<Duration>,
    it: I,
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
            next_entry,
            current_image: None,
            next_image: None,
            current_duration: None,
            it,
            window_size: INITIAL_WINDOW_SIZE,
        }
    }

    fn initialize(&mut self, ctx: &egui::Context, window_size: Vec2) {
        if window_size != self.window_size {
            self.current_image = get_image(self.current_entry.as_deref(), window_size, ctx);
            self.next_image = get_image(self.next_entry.as_deref(), window_size, ctx);
            self.current_duration = get_duration(&self.current_image, self.opts.period);
            if self.window_size == INITIAL_WINDOW_SIZE {
                self.last_image_time = Instant::now();
            }
            self.window_size = window_size;
        }
    }

    fn maybe_advance(
        &mut self,
        ctx: &egui::Context,
        window_size: Vec2,
        elapsed: Duration,
        maybe_duration: Option<Duration>,
    ) -> Option<Duration> {
        maybe_duration.map(|duration| {
            if elapsed > duration {
                std::mem::swap(&mut self.current_entry, &mut self.next_entry);
                std::mem::swap(&mut self.current_image, &mut self.next_image);
                self.next_entry = self.it.next();
                self.next_image = get_image(self.next_entry.as_deref(), window_size, ctx);
                self.current_duration = get_duration(&self.current_image, self.opts.period);
                self.last_image_time = Instant::now();
                println!("{:?}", self.current_entry);
                Duration::ZERO
            } else {
                elapsed
            }
        })
    }
}

impl<I: Iterator<Item = PathBuf>> eframe::App for MyApp<I> {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let window_size = window_size(ctx);
        if window_size == INITIAL_WINDOW_SIZE {
            return;
        }
        self.initialize(ctx, window_size);
        let elapsed = Instant::now() - self.last_image_time;
        let (Some(elapsed), Some(current_image)) = (
            self.maybe_advance(ctx, window_size, elapsed, self.current_duration),
            &self.current_image,
        ) else {
            ctx.send_viewport_cmd(ViewportCommand::Close);
            return;
        };
        let (current_texture, repaint_after) = match current_image {
            Image::Static(texture) => (texture, self.opts.period - elapsed),
            Image::Gif(gif) => {
                let mut i = 0;
                let mut time = gif.frames[0].duration;
                while time < elapsed {
                    i += 1;
                    time += gif.frames[i].duration;
                }
                (&gif.frames[i].texture, time - elapsed)
            }
        };
        egui::CentralPanel::default()
            .frame(egui::Frame::none())
            .show(ctx, |ui| {
                ui.with_layout(
                    egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
                    |ui| {
                        ui.image(current_texture);
                    },
                );
            });
        ctx.request_repaint_after(repaint_after);
    }
}

fn get_duration(image: &Option<Image>, period: Duration) -> Option<Duration> {
    image.as_ref().map(|image| match image {
        Image::Static(_) => period,
        Image::Gif(gif) => gif.duration,
    })
}

fn get_static_image(path: &Path, window_size: Vec2, ctx: &egui::Context) -> Option<Image> {
    load_image_from_path(path, window_size).ok().map(|image| {
        Image::Static(ctx.load_texture(path.to_string_lossy(), image, TextureOptions::default()))
    })
}

fn get_animated_image(path: &Path, window_size: Vec2, ctx: &egui::Context) -> Option<Image> {
    std::fs::File::open(path).ok().and_then(|file| {
        let decoder = GifDecoder::new(file).unwrap();
        decoder.into_frames().collect_frames().ok().map(|frames| {
            let frames: Vec<_> = frames
                .into_iter()
                .enumerate()
                .map(|(i, frame)| {
                    GifFrame::new(
                        ctx.load_texture(
                            format!("{}_frame{i}", path.to_string_lossy()),
                            image_from_buffer(frame.buffer(), window_size),
                            TextureOptions::default(),
                        ),
                        frame.delay().into(),
                    )
                })
                .collect();
            let duration = frames.iter().map(|frame| frame.duration).sum();
            Image::Gif(Gif::new(frames, duration))
        })
    })
}

fn get_image(entry: Option<&Path>, window_size: Vec2, ctx: &egui::Context) -> Option<Image> {
    use mime_guess::mime;
    entry.and_then(|path| {
        mime_guess::from_path(path)
            .first()
            .and_then(|x| match (x.type_(), x.subtype()) {
                (mime::IMAGE, mime::GIF) => get_animated_image(path, window_size, ctx),
                (mime::IMAGE, _) => get_static_image(path, window_size, ctx),
                _ => panic!(),
            })
    })
}

fn load_image_from_path(
    path: &Path,
    window_size: Vec2,
) -> Result<egui::ColorImage, image::ImageError> {
    let buffer = image::io::Reader::open(path)?.decode()?;
    Ok(image_from_buffer(&buffer, window_size))
}

fn image_from_buffer<I: GenericImageView<Pixel = image::Rgba<u8>>>(
    buffer: &I,
    Vec2 {
        x: window_w,
        y: window_h,
    }: Vec2,
) -> ColorImage {
    let (nwidth, nheight) = {
        let (w, h) = (buffer.width() as f32, buffer.height() as f32);
        let ratio = (window_w / w).min(window_h / h);
        ((ratio * w) as _, (ratio * h) as _)
    };
    let image_buffer = image::imageops::resize(buffer, nwidth, nheight, FilterType::Nearest);
    let pixels = image_buffer.as_flat_samples();
    egui::ColorImage::from_rgba_unmultiplied([nwidth as _, nheight as _], pixels.as_slice())
}

fn window_size(ctx: &egui::Context) -> Vec2 {
    ctx.input(|i| i.screen_rect).max.to_vec2()
}
