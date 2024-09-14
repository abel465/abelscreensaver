use crate::media_iterator::media_iterator;
use crate::mpvclient::MpvClient;
use crate::overlay::Overlay;
use crate::Options;
use egui_glow::egui_winit::winit;
use egui_glow::{glow, EventResponse};
use glutin::config::{Config, ConfigTemplateBuilder};
use glutin::context::{
    ContextApi, ContextAttributesBuilder, NotCurrentGlContext, PossiblyCurrentContext,
};
use glutin::display::GetGlDisplay;
use glutin::prelude::*;
use glutin::surface::{Surface, SwapInterval, WindowSurface};
use glutin_winit::GlWindow;
use libmpv::events::{Event as MPVEvent, PropertyData};
use libmpv::render::{OpenGLInitParams, RenderContext, RenderParam, RenderParamApiType};
use libmpv2 as libmpv;
use std::ffi::CString;
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::{DeviceEvent, DeviceId, ElementState, KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy};
use winit::keyboard::{Key, NamedKey, SmolStr};
use winit::window::{Fullscreen, Window, WindowId};

type GLContext = Rc<glutin::display::Display>;

#[derive(Debug)]
pub enum UserEvent {
    RequestRedraw,
    MPVEvents,
    Reset(Options),
}

fn setup_mpv(
    event_proxy: &EventLoopProxy<UserEvent>,
    ctx: GLContext,
    opts: &Options,
) -> (libmpv::Mpv, RenderContext) {
    let mut mpv = libmpv::Mpv::with_initializer(|mpv| {
        mpv.set_option("image-display-duration", opts.period_secs)?;
        mpv.set_option("mute", opts.mute)?;
        mpv.set_option("loop-playlist", "inf")?;
        Ok(())
    })
    .expect("Failed creating MPV");
    let mut render_context = RenderContext::new(
        unsafe { mpv.ctx.as_mut() },
        [
            RenderParam::ApiType(RenderParamApiType::OpenGl),
            RenderParam::InitParams(OpenGLInitParams {
                get_proc_address: |ctx: &GLContext, name: &str| {
                    ctx.get_proc_address(CString::new(name).unwrap().as_c_str()) as _
                },
                ctx,
            }),
        ],
    )
    .expect("Failed creating render context");

    mpv.event_context()
        .observe_property("path", libmpv::Format::String, 0)
        .unwrap();
    let event_proxy0 = event_proxy.clone();
    mpv.event_context_mut().set_wakeup_callback(move || {
        event_proxy0.send_event(UserEvent::MPVEvents).unwrap();
    });
    let event_proxy0 = event_proxy.clone();
    render_context.set_update_callback(move || {
        event_proxy0.send_event(UserEvent::RequestRedraw).unwrap();
    });
    mpv.event_context_mut().disable_deprecated_events().unwrap();

    (mpv, render_context)
}

pub fn run(opts: Options, black_pixel_path: PathBuf) {
    let event_loop = EventLoop::with_user_event().build().unwrap();
    let (window, gl_config) = {
        let window_attributes = Window::default_attributes()
            .with_fullscreen(Some(Fullscreen::Borderless(None)))
            .with_title("abelscreensaver");
        let display_builder =
            glutin_winit::DisplayBuilder::new().with_window_attributes(Some(window_attributes));
        let template_builder = ConfigTemplateBuilder::new();
        let (window, gl_config) = display_builder
            .build(&event_loop, template_builder, gl_config_picker)
            .unwrap();
        (window.unwrap(), gl_config)
    };

    let mut runner = Runner::new(
        opts,
        window,
        gl_config,
        event_loop.create_proxy(),
        black_pixel_path,
    );
    event_loop.run_app(&mut runner).unwrap();
}

struct ActiveRunner {
    gl_surface: Surface<WindowSurface>,
    gl_context: PossiblyCurrentContext,
    egui_glow: egui_glow::winit::EguiGlow,
    overlay: Overlay,
    render_context: RenderContext,
    mpv_client: MpvClient,
    has_media: bool,
    size: PhysicalSize<u32>,
}

impl ActiveRunner {
    fn new(
        opts: Options,
        gl_config: &Config,
        event_loop: &ActiveEventLoop,
        event_proxy: &EventLoopProxy<UserEvent>,
        window: &Window,
        it: &mut Box<dyn Iterator<Item = PathBuf>>,
        black_pixel_path: &Path,
    ) -> Self {
        let gl_display = gl_config.display();
        let size = window
            .current_monitor()
            .or(window.available_monitors().next())
            .unwrap()
            .size();
        let (gl_surface, gl_context) = {
            let gl_surface = unsafe {
                gl_display
                    .create_window_surface(
                        gl_config,
                        &window.build_surface_attributes(Default::default()).unwrap(),
                    )
                    .unwrap()
            };
            let gl_context = unsafe {
                gl_display.create_context(
                    gl_config,
                    &ContextAttributesBuilder::new()
                        .with_context_api(ContextApi::OpenGl(None))
                        .build(None),
                )
            }
            .unwrap()
            .make_current(&gl_surface)
            .unwrap();
            gl_surface.resize(
                &gl_context,
                NonZeroU32::new(size.width).unwrap(),
                NonZeroU32::new(size.height).unwrap(),
            );
            gl_surface
                .set_swap_interval(&gl_context, SwapInterval::Wait(NonZeroU32::new(1).unwrap()))
                .unwrap();
            (gl_surface, gl_context)
        };
        let egui_glow = {
            let mut egui_glow = egui_glow::winit::EguiGlow::new(
                event_loop,
                Arc::new(unsafe {
                    glow::Context::from_loader_function_cstr(|name| {
                        gl_display.get_proc_address(name)
                    })
                }),
                None,
                None,
                false,
            );
            egui_glow.run(window, |egui_ctx| {
                egui_extras::install_image_loaders(egui_ctx);
            });
            egui_glow
        };
        let (mpv_client, render_context, has_media) = {
            let (mpv, render_context) = setup_mpv(event_proxy, Rc::new(gl_display), &opts);
            let mpv_client = MpvClient::new(mpv);
            let has_media = if let Some(first_path) = it.next() {
                mpv_client.playlist_append_play(&first_path);
                true
            } else {
                mpv_client.set_image_duration(f64::MAX);
                mpv_client.playlist_append_play(black_pixel_path);
                false
            };
            (mpv_client, render_context, has_media)
        };
        let overlay = Overlay::new(size, opts);
        Self {
            size,
            egui_glow,
            mpv_client,
            gl_context,
            gl_surface,
            overlay,
            has_media,
            render_context,
        }
    }
}

struct Runner {
    opts: Options,
    window: Window,
    gl_config: Config,
    it: Box<dyn Iterator<Item = PathBuf>>,
    event_proxy: EventLoopProxy<UserEvent>,
    black_pixel_path: PathBuf,
    active_runner: Option<ActiveRunner>,
}

impl Runner {
    fn new(
        opts: Options,
        window: Window,
        gl_config: Config,
        event_proxy: EventLoopProxy<UserEvent>,
        black_pixel_path: PathBuf,
    ) -> Self {
        let it = Box::new(media_iterator(opts.clone()));
        Self {
            opts,
            window,
            gl_config,
            it,
            event_proxy,
            black_pixel_path,
            active_runner: None,
        }
    }
}

impl ApplicationHandler<UserEvent> for Runner {
    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UserEvent) {
        let window = &self.window;
        let Some(active_runner) = self.active_runner.as_mut() else {
            return;
        };
        let overlay = &mut active_runner.overlay;
        let mpv_client = &mut active_runner.mpv_client;
        let has_media = &mut active_runner.has_media;

        event_loop.set_control_flow(ControlFlow::Wait);
        match event {
            UserEvent::RequestRedraw => window.request_redraw(),
            UserEvent::MPVEvents => loop {
                match mpv_client.next_event() {
                    Some(Ok(MPVEvent::FileLoaded)) => {
                        if mpv_client.need_append() {
                            if let Some(path) = self.it.next() {
                                mpv_client.playlist_append(&path);
                            }
                        }
                        overlay.has_media = *has_media;
                    }
                    Some(Ok(MPVEvent::PropertyChange {
                        name: "path",
                        change: PropertyData::Str(str),
                        ..
                    })) => {
                        overlay.path = str.to_string();
                        if *has_media {
                            println!("{str}");
                        }
                    }
                    Some(Ok(_)) => {}
                    Some(Err(err)) => {
                        eprintln!("MPV Error: {}", err);
                        event_loop.exit();
                        break;
                    }
                    None => {
                        event_loop.set_control_flow(ControlFlow::Wait);
                        break;
                    }
                }
            },
            UserEvent::Reset(opts) => {
                self.it = Box::new(media_iterator(opts.clone()));
                *has_media = if let Some(first_path) = self.it.next() {
                    mpv_client.playlist_replace(&first_path);
                    mpv_client.playlist_clear();
                    mpv_client.set_image_duration(opts.period_secs);
                    mpv_client.set_mute(opts.mute);
                    true
                } else {
                    if *has_media {
                        mpv_client.playlist_replace(&self.black_pixel_path);
                        mpv_client.playlist_clear();
                        mpv_client.set_image_duration(f64::MAX);
                    }
                    false
                };
            }
        }
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.active_runner = Some(ActiveRunner::new(
            self.opts.clone(),
            &self.gl_config,
            event_loop,
            &self.event_proxy,
            &self.window,
            &mut self.it,
            &self.black_pixel_path,
        ));
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: DeviceId,
        event: DeviceEvent,
    ) {
        let Some(active_runner) = self.active_runner.as_mut() else {
            return;
        };
        if let DeviceEvent::MouseMotion { .. } = event {
            self.window.request_redraw();
            active_runner.overlay.reset_ui_render_instant();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: winit::event::WindowEvent,
    ) {
        let window = &self.window;
        let Some(active_runner) = self.active_runner.as_mut() else {
            return;
        };
        let egui_glow = &mut active_runner.egui_glow;
        let overlay = &mut active_runner.overlay;
        let render_context = &active_runner.render_context;
        let size = active_runner.size;
        let mpv_client = &active_runner.mpv_client;
        let gl_surface = &active_runner.gl_surface;
        let gl_context = &active_runner.gl_context;

        event_loop.set_control_flow(ControlFlow::Wait);
        let EventResponse { repaint, consumed } = egui_glow.on_window_event(window, &event);
        if repaint {
            window.request_redraw();
        }
        if consumed {
            return;
        }

        match event {
            WindowEvent::RedrawRequested => {
                render_context
                    .render::<GLContext>(0, size.width as _, size.height as _, true)
                    .expect("Failed to draw on glutin window");
                egui_glow.run(window, |egui_ctx| {
                    overlay.ui(egui_ctx, mpv_client, &self.event_proxy);
                });
                egui_glow.paint(window);
                if overlay.needs_repaint() {
                    window.request_redraw();
                }
                gl_surface.swap_buffers(gl_context).unwrap();
            }
            WindowEvent::CloseRequested => {
                self.active_runner = None;
                event_loop.exit();
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        logical_key: key,
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => match key {
                Key::Named(NamedKey::ArrowLeft) => mpv_client.playlist_prev(),
                Key::Named(NamedKey::ArrowRight) => mpv_client.playlist_next(),
                Key::Named(NamedKey::Space) => overlay.toggle_pause(mpv_client),
                Key::Character(str) if str == SmolStr::new_static("m") => {
                    overlay.toggle_mute(mpv_client)
                }
                _ => {}
            },
            _ => {}
        }
    }
}

pub fn gl_config_picker(configs: Box<dyn Iterator<Item = Config> + '_>) -> Config {
    configs
        .reduce(|accum, config| {
            if config.num_samples() > accum.num_samples() {
                config
            } else {
                accum
            }
        })
        .unwrap()
}
