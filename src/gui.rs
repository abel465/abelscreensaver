use crate::Opt;
use egui_glow::egui_winit::winit;
use egui_glow::glow;
use glutin::event::{Event, WindowEvent};
use libmpv::render::{OpenGLInitParams, RenderContext, RenderParam, RenderParamApiType};
use libmpv2 as libmpv;
use resvg::{tiny_skia, usvg};
use std::io::ErrorKind;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Debug)]
enum MPVEvent {
    MPVRenderUpdate,
    MPVEventUpdate,
    MPVClearOverlay,
}

type GLContext = Rc<glutin::ContextWrapper<glutin::PossiblyCurrent, glutin::window::Window>>;

fn setup_mpv(
    event_loop: &glutin::event_loop::EventLoop<MPVEvent>,
    ctx: GLContext,
    opts: Opt,
) -> (libmpv::Mpv, RenderContext) {
    let mut mpv = libmpv::Mpv::with_initializer(|mpv| {
        mpv.set_option("osd-align-x", "left")?;
        mpv.set_option("osd-align-y", "bottom")?;
        mpv.set_option("osd-margin-x", "5")?;
        mpv.set_option("osd-margin-y", "5")?;
        mpv.set_option("osd-border-size", "1")?;
        mpv.set_option("osd-font-size", "9")?;
        mpv.set_option(
            "image-display-duration",
            opts.period.as_secs_f32().to_string(),
        )?;
        Ok(())
    })
    .expect("Failed creating MPV");
    let mut render_context = RenderContext::new(
        unsafe { mpv.ctx.as_mut() },
        [
            RenderParam::ApiType(RenderParamApiType::OpenGl),
            RenderParam::InitParams(OpenGLInitParams {
                get_proc_address: |ctx: &GLContext, name: &str| ctx.get_proc_address(name) as _,
                ctx,
            }),
        ],
    )
    .expect("Failed creating render context");

    let event_proxy = event_loop.create_proxy();
    mpv.event_context_mut().set_wakeup_callback(move || {
        event_proxy.send_event(MPVEvent::MPVEventUpdate).unwrap();
    });
    let event_proxy = event_loop.create_proxy();
    render_context.set_update_callback(move || {
        event_proxy.send_event(MPVEvent::MPVRenderUpdate).unwrap();
    });
    mpv.event_context_mut().disable_deprecated_events().unwrap();

    (mpv, render_context)
}

struct BgraImage {
    path: String,
    width: u32,
    height: u32,
}

fn create_bgra(
    file_path: &str,
    temp_dir: &str,
    window_size: winit::dpi::PhysicalSize<u32>,
) -> BgraImage {
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
        width,
        height,
    }
}

struct Overlay {
    sound_on: BgraImage,
    sound_off: BgraImage,
    last_render_instant: Instant,
}

impl Overlay {
    const DURATION: Duration = Duration::from_secs(1);

    fn new(window_size: winit::dpi::PhysicalSize<u32>) -> Self {
        let mut temp_dir = std::env::temp_dir();
        temp_dir.push("abelscreensaver/");
        std::fs::create_dir(&temp_dir)
            .or_else(|e| {
                if e.kind() == ErrorKind::AlreadyExists {
                    Ok(())
                } else {
                    Err(e)
                }
            })
            .unwrap();
        let temp_dir = temp_dir.to_str().unwrap();

        let sound_on = create_bgra("assets/svg/sound-on.svg", temp_dir, window_size);
        let sound_off = create_bgra("assets/svg/sound-off.svg", temp_dir, window_size);

        Overlay {
            sound_on,
            sound_off,
            last_render_instant: Instant::now() - Self::DURATION,
        }
    }

    fn set_instant(&mut self) {
        self.last_render_instant = Instant::now();
    }

    fn should_clear(&self) -> bool {
        self.last_render_instant.elapsed() >= Self::DURATION
    }
}

pub fn main_stuff<I: Iterator<Item = PathBuf> + 'static>(opts: Opt, mut it: I) {
    let Some(first_path) = it.next() else {
        return;
    };
    let event_loop = glutin::event_loop::EventLoopBuilder::<MPVEvent>::with_user_event().build();
    let size = event_loop
        .primary_monitor()
        .or(event_loop.available_monitors().next())
        .unwrap()
        .size();
    let (gl, window) = unsafe {
        let window_builder = glutin::window::WindowBuilder::new()
            .with_inner_size(size)
            .with_fullscreen(Some(glutin::window::Fullscreen::Borderless(None)));
        let window = glutin::ContextBuilder::new()
            .with_vsync(true)
            .build_windowed(window_builder, &event_loop)
            .expect("Failed to build glutin window")
            .make_current()
            .expect("Failed to make window current");
        let gl = glow::Context::from_loader_function(|name| window.get_proc_address(name));
        (Arc::new(gl), Rc::new(window))
    };

    let mut overlay = Overlay::new(size);
    let (mut mpv, render_context) = setup_mpv(&event_loop, window.clone(), opts);
    mpv.command("loadfile", &[&first_path.to_str().unwrap(), "append-play"])
        .unwrap();
    let mut egui_glow = egui_glow::winit::EguiGlow::new(&event_loop, gl, None);
    let event_proxy = event_loop.create_proxy();

    event_loop.run(move |event, _, ctrl_flow| {
        ctrl_flow.set_wait();

        match event {
            Event::RedrawRequested(_) => {
                egui_glow.run(window.window(), |_egui_ctx| {});
                render_context
                    .render::<GLContext>(0, size.width as _, size.height as _, true)
                    .expect("Failed to draw on glutin window");
                egui_glow.paint(window.window());
                window.swap_buffers().unwrap();
            }
            Event::WindowEvent { event, .. } => {
                match event {
                    WindowEvent::CloseRequested => {
                        ctrl_flow.set_exit();
                    }
                    WindowEvent::KeyboardInput {
                        input:
                            winit::event::KeyboardInput {
                                virtual_keycode: Some(key),
                                state: winit::event::ElementState::Pressed,
                                ..
                            },
                        ..
                    } => match key {
                        winit::event::VirtualKeyCode::Left => {
                            mpv.command("playlist-prev", &[]).ok();
                        }
                        winit::event::VirtualKeyCode::Right => {
                            mpv.command("playlist-next", &[]).ok();
                        }
                        winit::event::VirtualKeyCode::M => {
                            let mute = mpv.get_property::<String>("mute").unwrap();
                            let (new_mute, image) = if mute == "yes" {
                                ("no", &overlay.sound_on)
                            } else {
                                ("yes", &overlay.sound_off)
                            };
                            mpv.set_property("mute", new_mute).unwrap();
                            mpv.command(
                                "overlay-add",
                                &[
                                    "0",
                                    &((size.width - image.width) / 2).to_string(),
                                    &((size.height - image.height) / 2).to_string(),
                                    &image.path,
                                    "0",
                                    "bgra",
                                    &image.width.to_string(),
                                    &image.height.to_string(),
                                    &(image.width * 4).to_string(),
                                ],
                            )
                            .unwrap();
                            overlay.set_instant();
                            let event_proxy = event_proxy.clone();
                            std::thread::spawn(move || {
                                std::thread::sleep(Overlay::DURATION);
                                event_proxy.send_event(MPVEvent::MPVClearOverlay).unwrap();
                            });
                        }
                        _ => {}
                    },
                    _ => {}
                }
                if egui_glow.on_event(&event).repaint {
                    window.window().request_redraw();
                }
            }
            Event::UserEvent(event) => match event {
                MPVEvent::MPVClearOverlay => {
                    if overlay.should_clear() {
                        mpv.command("overlay-remove", &["0"]).unwrap();
                    }
                }
                MPVEvent::MPVRenderUpdate => window.window().request_redraw(),
                MPVEvent::MPVEventUpdate => loop {
                    match mpv.event_context_mut().wait_event(0.0) {
                        Some(Ok(libmpv::events::Event::StartFile)) => {
                            if let Some(path) = it.next() {
                                mpv.command("loadfile", &[&path.to_str().unwrap(), "append"])
                                    .unwrap();
                            }
                        }
                        Some(Ok(libmpv::events::Event::EndFile(_))) => {
                            if mpv.get_property::<String>("playlist-pos").unwrap() == "-1" {
                                ctrl_flow.set_exit();
                                break;
                            }
                        }
                        Some(Ok(libmpv::events::Event::FileLoaded)) => {
                            mpv.command("show-text", &["${path}", "2147483647"])
                                .unwrap();
                        }
                        Some(Ok(_)) => {}
                        Some(Err(err)) => {
                            eprintln!("MPV Error: {}", err);
                            ctrl_flow.set_exit();
                            break;
                        }
                        None => {
                            ctrl_flow.set_wait();
                            break;
                        }
                    }
                },
            },
            _ => {}
        }
    })
}
