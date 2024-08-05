use crate::mpvclient::MpvClient;
use crate::overlay::Overlay;
use crate::Options;
use egui_glow::egui_winit::winit::event::{ElementState, KeyboardInput, VirtualKeyCode};
use egui_glow::{glow, EventResponse};
use glutin::event::{DeviceEvent, Event, WindowEvent};
use glutin::event_loop::{EventLoop, EventLoopBuilder};
use libmpv::events::{Event as MPVEvent, PropertyData};
use libmpv::render::{OpenGLInitParams, RenderContext, RenderParam, RenderParamApiType};
use libmpv2 as libmpv;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

#[derive(Debug)]
pub enum UserEvent {
    RequestRedraw,
    MPVEvents,
}

type GLContext = Rc<glutin::ContextWrapper<glutin::PossiblyCurrent, glutin::window::Window>>;

fn setup_mpv(
    event_loop: &EventLoop<UserEvent>,
    ctx: GLContext,
    opts: &Options,
) -> (libmpv::Mpv, RenderContext) {
    let mut mpv = libmpv::Mpv::with_initializer(|mpv| {
        mpv.set_option("image-display-duration", opts.period_secs)?;
        mpv.set_option("mute", opts.mute)?;
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

    mpv.event_context()
        .observe_property("playback-abort", libmpv::Format::Flag, 0)
        .unwrap();
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

pub fn run<I: Iterator<Item = PathBuf> + 'static>(opts: Options, mut it: I) {
    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
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

    let (mpv, render_context) = setup_mpv(&event_loop, window.clone(), &opts);
    let mut egui_glow = egui_glow::winit::EguiGlow::new(&event_loop, gl, None);

    let mut overlay = Overlay::new(size, opts);
    let mut mpv_client = MpvClient::new(mpv);
    if let Some(first_path) = it.next() {
        mpv_client.playlist_append_play(&first_path);
    } else {
        overlay.no_media = true;
    }

    event_loop.run(move |event, _, ctrl_flow| {
        ctrl_flow.set_wait();

        match event {
            Event::RedrawRequested(_) => {
                render_context
                    .render::<GLContext>(0, size.width as _, size.height as _, true)
                    .expect("Failed to draw on glutin window");
                egui_glow.run(window.window(), |egui_ctx| {
                    overlay.ui(egui_ctx, &mpv_client)
                });
                if overlay.needs_repaint() {
                    window.window().request_redraw();
                }
                egui_glow.paint(window.window());
                window.swap_buffers().unwrap();
            }
            Event::WindowEvent { event, .. } => {
                let EventResponse { repaint, consumed } = egui_glow.on_event(&event);
                if repaint {
                    window.window().request_redraw();
                }
                if !consumed {
                    match event {
                        WindowEvent::CloseRequested => {
                            ctrl_flow.set_exit();
                        }
                        WindowEvent::KeyboardInput {
                            input:
                                KeyboardInput {
                                    virtual_keycode: Some(key),
                                    state: ElementState::Pressed,
                                    ..
                                },
                            ..
                        } => match key {
                            VirtualKeyCode::Left => mpv_client.playlist_prev(),
                            VirtualKeyCode::Right => mpv_client.playlist_next(),
                            VirtualKeyCode::M => overlay.toggle_mute(&mpv_client),
                            VirtualKeyCode::Space => overlay.toggle_pause(&mpv_client),
                            _ => {}
                        },
                        _ => {}
                    }
                }
            }
            Event::DeviceEvent {
                event: DeviceEvent::MouseMotion { .. },
                ..
            } => {
                window.window().request_redraw();
                overlay.reset_ui_render_instant();
            }
            Event::UserEvent(event) => match event {
                UserEvent::RequestRedraw => window.window().request_redraw(),
                UserEvent::MPVEvents => loop {
                    match mpv_client.next_event() {
                        Some(Ok(MPVEvent::FileLoaded)) => {
                            if let Some(path) = it.next() {
                                mpv_client.playlist_append(&path);
                            }
                            overlay.path = mpv_client.get_path();
                        }
                        Some(Ok(MPVEvent::PropertyChange {
                            name: "playback-abort",
                            change: PropertyData::Flag(true),
                            ..
                        })) => {
                            mpv_client.playlist_from_beginning();
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
