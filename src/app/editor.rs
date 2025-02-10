use std::ops::RangeInclusive;

use egui::{Color32, Slider};
pub use egui_winit::State as EguiWinitState;
use nalgebra::{Point3, Vector3};
use winit::window::Window;

use crate::{game::GameState, graphics::camera::Projection};

pub struct Editor {
    pub gui_state: EguiWinitState,
    pub gui_ctx: egui::Context,
}

impl Editor {
    pub fn new(window: &Window) -> Self {
        let gui_ctx = egui::Context::default();
        let gui_state = EguiWinitState::new(
            gui_ctx.clone(),
            gui_ctx.viewport_id(),
            window,
            Some(window.scale_factor() as f32),
            None,
            None,
        );

        Self { gui_state, gui_ctx }
    }

    pub fn run(
        &mut self,
        egui_input: egui::RawInput,
        game_state: &mut GameState,
        proj: &mut Projection,
    ) -> (egui::FullOutput, egui::Context) {
        let output = self.gui_ctx.run(egui_input, |gui_ctx| {
            egui::Window::new("Editor window").show(gui_ctx, |ui| {
                ui.collapsing("View", |ui| {
                    ui.label("Eye: ");
                    point_slider(ui, &mut game_state.view.eye, -10.0..=10.0);
                    ui.label("Up: ");
                    vec_slider(ui, &mut game_state.view.up);
                    ui.label("Angle: ");
                    angle_slider(
                        ui,
                        (
                            &mut game_state.view.yaw_deg,
                            &mut game_state.view.pitch_deg,
                            &mut game_state.view.roll_deg,
                        ),
                    );
                });

                ui.collapsing("Projection", |ui| {
                    ui.label("Fov Y: ");
                    ui.add(Slider::new(&mut proj.fov_deg, 0.0..=180.0));
                });
            });
        });

        (output, self.gui_ctx.clone())
    }
}

fn point_slider(ui: &mut egui::Ui, value: &mut Point3<f32>, range: RangeInclusive<f32>) {
    ui.add(
        Slider::new(&mut value.coords[0], range.clone())
            .text("X")
            .text_color(Color32::RED),
    );
    ui.add(
        Slider::new(&mut value.coords[1], range.clone())
            .text("Y")
            .text_color(Color32::GREEN),
    );
    ui.add(
        Slider::new(&mut value.coords[2], range)
            .text("Z")
            .text_color(Color32::CYAN),
    );
}

fn vec_slider(ui: &mut egui::Ui, value: &mut Vector3<f32>) {
    ui.add(
        Slider::new(&mut value.data.0[0][0], -1.0..=1.0)
            .text("X")
            .text_color(Color32::RED),
    );
    ui.add(
        Slider::new(&mut value.data.0[0][1], -1.0..=1.0)
            .text("Y")
            .text_color(Color32::GREEN),
    );
    ui.add(
        Slider::new(&mut value.data.0[0][2], -1.0..=1.0)
            .text("Z")
            .text_color(Color32::CYAN),
    );
}

fn angle_slider(ui: &mut egui::Ui, (yaw, pitch, roll): (&mut f32, &mut f32, &mut f32)) {
    ui.add(
        Slider::new(yaw, -180.0..=180.0)
            .text("Yaw")
            .text_color(Color32::RED),
    );
    ui.add(
        Slider::new(pitch, -180.0..=180.0)
            .text("Pitch")
            .text_color(Color32::GREEN),
    );
    ui.add(
        Slider::new(roll, -180.0..=180.0)
            .text("Roll")
            .text_color(Color32::CYAN),
    );
}
