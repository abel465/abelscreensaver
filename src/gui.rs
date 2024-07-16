use crate::Opt;
use egui_glow::egui_winit::winit;
use glutin::event::{Event, WindowEvent};
use glutin::event_loop::EventLoop;
use glutin::{ContextWrapper, PossiblyCurrent};
use libmpv::render::{OpenGLInitParams, RenderContext, RenderParam, RenderParamApiType};
use std::ffi::c_void;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

#[derive(Debug)]
enum MPVEvent {
    MPVRenderUpdate,
    MPVEventUpdate,
}

type Display = Rc<ContextWrapper<PossiblyCurrent, glutin::window::Window>>;

fn setup_mpv(
    event_loop: &EventLoop<MPVEvent>,
    display: Display,
    opts: Opt,
) -> (libmpv::Mpv, RenderContext) {
    fn get_proc_address(window: &Display, name: &str) -> *mut c_void {
        window.get_proc_address(name) as *mut _
    }
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
                get_proc_address,
                ctx: display,
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
        let gl = glow::Context::from_loader_function(|l| window.get_proc_address(l) as *const _);
        (Arc::new(gl), Rc::new(window))
    };

    let (mut mpv, render_context) = setup_mpv(&event_loop, window.clone(), opts);
    mpv.playlist_load_files(&[(
        &first_path.to_str().unwrap(),
        libmpv::FileState::AppendPlay,
        None,
    )])
    .unwrap();

    let mut egui_glow = egui_glow::winit::EguiGlow::new(&event_loop, gl, None);

    event_loop.run(move |event, _, ctrl_flow| {
        ctrl_flow.set_wait();

        match event {
            Event::RedrawRequested(_) => {
                egui_glow.run(window.window(), |_egui_ctx| {});
                render_context
                    .render::<Display>(0, size.width as _, size.height as _, true)
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
                            let _ = mpv.playlist_previous_weak();
                        }
                        winit::event::VirtualKeyCode::Right => {
                            let _ = mpv.playlist_next_weak();
                        }
                        winit::event::VirtualKeyCode::M => {
                            let mute = mpv.get_property::<String>("mute").unwrap();
                            let new_mute = if mute == "yes" { "no" } else { "yes" };
                            mpv.set_property::<&str>("mute", new_mute).unwrap();
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
                MPVEvent::MPVRenderUpdate => window.window().request_redraw(),
                MPVEvent::MPVEventUpdate => loop {
                    match mpv.event_context_mut().wait_event(0.0) {
                        Some(Ok(libmpv::events::Event::StartFile)) => {
                            if let Some(path) = it.next() {
                                mpv.playlist_load_files(&[(
                                    &path.to_str().unwrap(),
                                    libmpv::FileState::Append,
                                    None,
                                )])
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
