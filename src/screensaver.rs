use crate::gui::UserEvent;
use egui_glow::egui_winit::winit;
use libmpv::events::Event as MPVEvent;
use libmpv2 as libmpv;
use resvg::{tiny_skia, usvg};
use std::path::Path;
use std::time::{Duration, Instant};
use winit::dpi::PhysicalSize;
use winit::event_loop::EventLoopProxy;

pub struct ScreenSaver {
    mpv: libmpv::Mpv,
    overlay: Overlay,
}

impl ScreenSaver {
    pub fn new(
        mpv: libmpv::Mpv,
        window_size: PhysicalSize<u32>,
        event_proxy: EventLoopProxy<UserEvent>,
    ) -> Self {
        let overlay = Overlay::new(window_size, event_proxy);
        ScreenSaver { mpv, overlay }
    }

    pub fn playlist_prev(&self) {
        self.mpv.command("playlist-prev", &[]).ok();
    }

    pub fn playlist_next(&self) {
        self.mpv.command("playlist-next", &[]).ok();
    }

    pub fn playlist_append_play(&self, path: &Path) {
        self.mpv
            .command("loadfile", &[&path.to_str().unwrap(), "append-play"])
            .unwrap();
    }

    pub fn playlist_append(&self, path: &Path) {
        self.mpv
            .command("loadfile", &[&path.to_str().unwrap(), "append"])
            .unwrap();
    }

    pub fn show_path(&self) {
        self.mpv
            .command("show-text", &["${path}", "2147483647"])
            .unwrap();
    }

    pub fn finished(&self) -> bool {
        self.mpv.get_property::<String>("playlist-pos").unwrap() == "-1"
    }

    pub fn next_event(&mut self) -> Option<libmpv::Result<MPVEvent>> {
        self.mpv.event_context_mut().wait_event(0.0)
    }

    pub fn toggle_mute(&mut self) {
        let mute = self.mpv.get_property::<String>("mute").unwrap();
        let (mute, overlay_type) = if mute == "yes" {
            ("no", OverlayType::SoundOn)
        } else {
            ("yes", OverlayType::SoundOff)
        };
        self.mpv.set_property("mute", mute).unwrap();
        self.overlay.show(overlay_type, &self.mpv);
    }

    pub fn maybe_clear_overlay(&self) {
        self.overlay.maybe_clear(&self.mpv);
    }
}

struct BgraImage {
    path: String,
    size: PhysicalSize<u32>,
}

fn create_bgra(file_path: &str, temp_dir: &str, window_size: PhysicalSize<u32>) -> BgraImage {
    let path = std::path::Path::new(file_path);
    let tree = usvg::Tree::from_str(
        &std::fs::read_to_string(file_path).unwrap(),
        &usvg::Options::default(),
    )
    .unwrap();
    let width = (window_size.width.min(window_size.height) as f32 * 0.1) as u32;
    let size = tree.size();
    let scale = width as f32 / size.width();
    let height = (size.height() * scale) as u32;
    let mut pixmap = tiny_skia::Pixmap::new(width, height).unwrap();
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    let mut data = Vec::with_capacity((width * height * 4) as usize);
    for p in pixmap.pixels() {
        data.push(p.blue());
        data.push(p.green());
        data.push(p.red());
        data.push(p.alpha());
    }
    let file_name = path.file_name().unwrap().to_str().unwrap();
    let path = format!("{temp_dir}{file_name}");
    std::fs::write(&path, &data).expect("Unable to write file");
    BgraImage {
        path,
        size: PhysicalSize::<u32>::new(width, height),
    }
}

enum OverlayType {
    SoundOn,
    SoundOff,
}

struct Overlay {
    sound_on: BgraImage,
    sound_off: BgraImage,
    last_render_instant: Instant,
    event_proxy: EventLoopProxy<UserEvent>,
    window_size: PhysicalSize<u32>,
}

impl Overlay {
    const DURATION: Duration = Duration::from_secs(1);

    fn new(window_size: PhysicalSize<u32>, event_proxy: EventLoopProxy<UserEvent>) -> Self {
        let mut temp_dir = std::env::temp_dir();
        temp_dir.push("abelscreensaver/");
        std::fs::create_dir_all(&temp_dir).unwrap();
        let temp_dir = temp_dir.to_str().unwrap();

        Overlay {
            sound_on: create_bgra("assets/svg/sound-on.svg", temp_dir, window_size),
            sound_off: create_bgra("assets/svg/sound-off.svg", temp_dir, window_size),
            last_render_instant: Instant::now() - Self::DURATION,
            event_proxy,
            window_size,
        }
    }

    fn show(&mut self, overlay_type: OverlayType, mpv: &libmpv::Mpv) {
        let BgraImage {
            path,
            size: PhysicalSize { width, height },
        } = match overlay_type {
            OverlayType::SoundOff => &self.sound_off,
            OverlayType::SoundOn => &self.sound_on,
        };
        self.last_render_instant = Instant::now();
        mpv.command(
            "overlay-add",
            &[
                "0",
                &((self.window_size.width - width) / 2).to_string(),
                &((self.window_size.height - height) / 2).to_string(),
                &path,
                "0",
                "bgra",
                &width.to_string(),
                &height.to_string(),
                &(width * 4).to_string(),
            ],
        )
        .unwrap();
        let event_proxy = self.event_proxy.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Overlay::DURATION);
            event_proxy.send_event(UserEvent::ClearOverlay).unwrap();
        });
    }

    fn maybe_clear(&self, mpv: &libmpv::Mpv) {
        if self.last_render_instant.elapsed() >= Self::DURATION {
            mpv.command("overlay-remove", &["0"]).unwrap();
        }
    }
}
