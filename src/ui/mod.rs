use std::{num::NonZeroU32, time::Instant};

use glutin::{
    config::ConfigTemplateBuilder,
    context::{ContextAttributesBuilder, NotCurrentGlContext, PossiblyCurrentContext},
    display::{GetGlDisplay, GlDisplay},
    surface::{GlSurface, Surface, SurfaceAttributesBuilder, WindowSurface},
};
use imgui_glow_renderer::glow::HasContext;
use imgui_winit_support::winit::{
    dpi::LogicalSize,
    event_loop::EventLoop,
    raw_window_handle::HasWindowHandle,
    window::{Window, WindowAttributes},
};
use imgui_winit_support::WinitPlatform;
use std::error::Error;

use crate::core::{feusic::loader::MusicLoader, player::FeusicPlayerController};

mod view;

const TITLE: &str = "Hello, imgui-rs!";

pub fn run_ui<M: MusicLoader>(player: &FeusicPlayerController<M>) -> Result<(), Box<dyn Error>> {
    let (event_loop, window, surface, context) = create_window();
    let (mut winit_platform, mut imgui_context) = imgui_init(&window);

    let gl = glow_context(&context);

    let mut ig_renderer = imgui_glow_renderer::AutoRenderer::new(gl, &mut imgui_context)
        .expect("failed to create renderer");

    let mut last_frame = Instant::now();

    #[allow(deprecated)]
    event_loop.run(move |event, window_target| match event {
        winit::event::Event::NewEvents(_) => {
            let now = Instant::now();
            imgui_context
                .io_mut()
                .update_delta_time(now.duration_since(last_frame));
            last_frame = now;
        }
        winit::event::Event::AboutToWait => {
            winit_platform
                .prepare_frame(imgui_context.io_mut(), &window)
                .unwrap();
            window.request_redraw();
        }
        winit::event::Event::WindowEvent {
            event: winit::event::WindowEvent::RedrawRequested,
            ..
        } => {
            unsafe { ig_renderer.gl_context().clear(glow::COLOR_BUFFER_BIT) };

            let ui = imgui_context.frame();
            view::render(&ui, player).expect("error rendering window");

            winit_platform.prepare_render(ui, &window);
            let draw_data = imgui_context.render();

            ig_renderer
                .render(draw_data)
                .expect("error rendering imgui");

            surface
                .swap_buffers(&context)
                .expect("Failed to swap buffers");
        }
        winit::event::Event::WindowEvent {
            event: winit::event::WindowEvent::CloseRequested,
            ..
        } => {
            window_target.exit();
        }
        winit::event::Event::WindowEvent {
            event: winit::event::WindowEvent::Resized(new_size),
            ..
        } => {
            if new_size.width > 0 && new_size.height > 0 {
                surface.resize(
                    &context,
                    NonZeroU32::new(new_size.width).unwrap(),
                    NonZeroU32::new(new_size.height).unwrap(),
                );
            }
            winit_platform.handle_event(imgui_context.io_mut(), &window, &event);
        }
        event => {
            winit_platform.handle_event(imgui_context.io_mut(), &window, &event);
        }
    })?;

    Ok(())
}

fn create_window() -> (
    EventLoop<()>,
    Window,
    Surface<WindowSurface>,
    PossiblyCurrentContext,
) {
    let event_loop = EventLoop::new().unwrap();

    let window_attributes = WindowAttributes::default()
        .with_title(TITLE)
        .with_inner_size(LogicalSize::new(1024, 768));
    let (window, cfg) = glutin_winit::DisplayBuilder::new()
        .with_window_attributes(Some(window_attributes))
        .build(&event_loop, ConfigTemplateBuilder::new(), |mut configs| {
            configs.next().unwrap()
        })
        .expect("Failed to create OpenGL window");

    let window = window.unwrap();

    let context_attribs =
        ContextAttributesBuilder::new().build(Some(window.window_handle().unwrap().as_raw()));
    let context = unsafe {
        cfg.display()
            .create_context(&cfg, &context_attribs)
            .expect("Failed to create OpenGL context")
    };

    let surface_attribs = SurfaceAttributesBuilder::<WindowSurface>::new()
        .with_srgb(Some(true))
        .build(
            window.window_handle().unwrap().as_raw(),
            NonZeroU32::new(1024).unwrap(),
            NonZeroU32::new(768).unwrap(),
        );
    let surface = unsafe {
        cfg.display()
            .create_window_surface(&cfg, &surface_attribs)
            .expect("Failed to create OpenGL surface")
    };

    let context = context
        .make_current(&surface)
        .expect("Failed to make OpenGL context current");

    (event_loop, window, surface, context)
}

fn glow_context(context: &PossiblyCurrentContext) -> glow::Context {
    unsafe {
        glow::Context::from_loader_function_cstr(|s| context.display().get_proc_address(s).cast())
    }
}

fn imgui_init(window: &Window) -> (WinitPlatform, imgui::Context) {
    let mut imgui_context = imgui::Context::create();
    imgui_context.set_ini_filename(None);

    let mut winit_platform = WinitPlatform::new(&mut imgui_context);
    winit_platform.attach_window(
        imgui_context.io_mut(),
        window,
        imgui_winit_support::HiDpiMode::Rounded,
    );

    imgui_context
        .fonts()
        .add_font(&[imgui::FontSource::DefaultFontData { config: None }]);

    imgui_context.io_mut().font_global_scale = (1.0 / winit_platform.hidpi_factor()) as f32;

    (winit_platform, imgui_context)
}
