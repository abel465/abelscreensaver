use crate::screensaver::ScreenSaver;
use crate::Opt;
use egui_glow::egui_winit::winit;
use egui_glow::glow;
use glutin::event::{Event, WindowEvent};
use libmpv::events::Event as MPVEvent;
use libmpv::render::{OpenGLInitParams, RenderContext, RenderParam, RenderParamApiType};
use libmpv2 as libmpv;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

#[derive(Debug)]
pub enum UserEvent {
    RequestRedraw,
    MPVEvents,
    ClearOverlay,
}

type GLContext = Rc<glutin::ContextWrapper<glutin::PossiblyCurrent, glutin::window::Window>>;

fn setup_mpv(
    event_loop: &glutin::event_loop::EventLoop<UserEvent>,
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
        event_proxy.send_event(UserEvent::MPVEvents).unwrap();
    });
    let event_proxy = event_loop.create_proxy();
    render_context.set_update_callback(move || {
        event_proxy.send_event(UserEvent::RequestRedraw).unwrap();
    });
    mpv.event_context_mut().disable_deprecated_events().unwrap();

    (mpv, render_context)
}

pub fn run<I: Iterator<Item = PathBuf> + 'static>(opts: Opt, mut it: I) {
    let Some(first_path) = it.next() else {
        return;
    };
    let event_loop = glutin::event_loop::EventLoopBuilder::<UserEvent>::with_user_event().build();
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

    let (mpv, render_context) = setup_mpv(&event_loop, window.clone(), opts);
    let mut app = ScreenSaver::new(mpv, size, event_loop.create_proxy());
    app.playlist_append_play(&first_path);
    let mut egui_glow = egui_glow::winit::EguiGlow::new(&event_loop, gl, None);

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
                            app.playlist_prev();
                        }
                        winit::event::VirtualKeyCode::Right => {
                            app.playlist_next();
                        }
                        winit::event::VirtualKeyCode::M => {
                            app.toggle_mute();
                        }
                        winit::event::VirtualKeyCode::Space => {
                            app.toggle_pause();
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
                UserEvent::ClearOverlay => app.maybe_clear_overlay(),
                UserEvent::RequestRedraw => window.window().request_redraw(),
                UserEvent::MPVEvents => loop {
                    match app.next_event() {
                        Some(Ok(MPVEvent::StartFile)) => {
                            if let Some(path) = it.next() {
                                app.playlist_append(&path);
                            }
                        }
                        Some(Ok(MPVEvent::EndFile(_))) => {
                            if app.finished() {
                                ctrl_flow.set_exit();
                                break;
                            }
                        }
                        Some(Ok(MPVEvent::FileLoaded)) => {
                            app.show_path();
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
