use std::{sync::Arc, time::Instant};

use editor::Editor;
use inputs::Inputs;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{self, ActiveEventLoop},
    window::{Window, WindowAttributes},
};

use crate::{
    constants,
    game::GameState,
    graphics::{
        camera::Projection,
        ctx::{self, GraphicsCtx},
        GlobalRenderer, RenderData,
    },
};

pub mod editor;
pub mod inputs;

pub struct App {
    window: Arc<Window>,
    inputs: Inputs,

    graphics: GraphicsCtx,
    proj: Projection,
    renderer: GlobalRenderer,

    editor: Editor,
    game_state: GameState,

    last_update: Instant,
}

impl App {
    pub fn run() {
        let event_loop = event_loop::EventLoop::new().expect("Failed to create event loop");
        event_loop.set_control_flow(event_loop::ControlFlow::Poll);
        event_loop
            .run_app(&mut AppRunner::default())
            .unwrap_or_else(|e| panic!("Failed to run app: {e}"));
    }

    fn init(event_loop: &ActiveEventLoop) -> Self {
        let window: Arc<_> = event_loop
            .create_window(WindowAttributes::default().with_title(constants::WINDOW_TITLE))
            .expect("Failed to create window")
            .into();

        let inputs = Inputs::default();
        let graphics = GraphicsCtx::new(window.clone());
        let proj = Projection {
            aspect_ratio: 1.0,
            fov_deg: 90.0,
        };
        let renderer = GlobalRenderer::new(&graphics);
        let editor_state = Editor::new(&window);
        let game_state = GameState::new();
        let last_update = Instant::now();

        App {
            window,
            inputs,
            graphics,
            proj,
            renderer,
            editor: editor_state,
            game_state,
            last_update,
        }
    }

    fn render(&mut self) {
        let window_size: (u32, u32) = self.window.inner_size().into();
        if window_size.0 < 1 || window_size.1 < 1 {
            return;
        }

        let egui_input = self.editor.gui_state.take_egui_input(&self.window);
        let (egui_output, egui_ctx) = self.editor.run(
            &mut self.renderer,
            &self.graphics,
            egui_input,
            &mut self.game_state,
            &mut self.proj,
        );

        let render_data = RenderData {
            window_size,
            aspect_ratio: self.window.scale_factor() as f32,

            egui_ctx,
            egui_output,
        };

        self.renderer.submit(&self.graphics, render_data);
        self.window.request_redraw();
    }

    fn update(&mut self) {
        let dt = self.last_update.elapsed();
        self.last_update = Instant::now();
        self.game_state.update(&self.inputs, dt);

        self.renderer
            .update_view(&self.graphics, &self.game_state.view);
        self.inputs.step();
    }

    fn resize_viewport(&mut self) {
        self.proj.aspect_ratio = self.window.scale_factor() as f32;
        self.renderer.update_proj(&self.graphics, &self.proj);

        self.graphics.resize(self.window.inner_size().into());
        self.renderer.update_viewport_size(&self.graphics);
    }
}

#[derive(Default)]
struct AppRunner(Option<App>);

impl ApplicationHandler for AppRunner {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.0 = Some(App::init(event_loop));
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        if let Some(app) = &mut self.0 {
            app.inputs.process_window_event(&event);
            let _ = app.editor.gui_state.on_window_event(&app.window, &event);

            match event {
                WindowEvent::CloseRequested | WindowEvent::Destroyed => {
                    event_loop.exit();
                }
                WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
                    app.resize_viewport();
                }
                WindowEvent::RedrawRequested => {
                    app.render();
                }
                _ => {}
            }
        }
    }

    fn device_event(
        &mut self,
        _: &winit::event_loop::ActiveEventLoop,
        _: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        if let Some(app) = &mut self.0 {
            app.inputs.process_device_event(&event);
            if let winit::event::DeviceEvent::MouseMotion { delta } = event {
                app.editor.gui_state.on_mouse_motion(delta);
            }
        }
    }

    fn about_to_wait(&mut self, _: &event_loop::ActiveEventLoop) {
        if let Some(app) = &mut self.0 {
            app.update();
        }
    }
}
