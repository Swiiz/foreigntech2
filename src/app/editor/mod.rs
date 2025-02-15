use std::ops::RangeInclusive;

use egui::{Color32, Slider};
pub use egui_winit::State as EguiWinitState;
use light::LightEditor;
use nalgebra::{Matrix4, Point3, Vector3};
use winit::window::Window;

use crate::{
    game::GameState,
    graphics::{camera::Projection, entities::model::ModelInstance, GlobalRenderer},
};

pub mod light;

pub struct Editor {
    pub gui_state: EguiWinitState,
    pub gui_ctx: egui::Context,

    pub light_editor: LightEditor,

    pub new_inst_pos: Point3<f32>,
    pub mat_id: u32,
    pub model_id: u32,
    pub mesh_id: u32,
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
        let light_editor = LightEditor::default();

        Self {
            gui_state,
            gui_ctx,
            light_editor,
            new_inst_pos: Default::default(),
            mat_id: 0,
            model_id: 0,
            mesh_id: 0,
        }
    }

    pub fn run(
        &mut self,
        renderer: &mut GlobalRenderer,
        egui_input: egui::RawInput,
        game_state: &mut GameState,
        proj: &mut Projection,
    ) -> (egui::FullOutput, egui::Context) {
        let output = self.gui_ctx.run(egui_input, |gui_ctx| {
            egui::Window::new("Editor window").show(gui_ctx, |ui| {
                ui.collapsing("View", |ui| {
                    ui.label("Eye: ");
                    point_slider(ui, &mut game_state.camera.eye, -10.0..=10.0);
                    ui.label("Up: ");
                    vec3_slider(ui, &mut game_state.camera.up);
                    ui.label("Angle: ");
                    angle_slider(
                        ui,
                        (
                            &mut game_state.camera.yaw_deg,
                            &mut game_state.camera.pitch_deg,
                            &mut game_state.camera.roll_deg,
                        ),
                    );
                });

                ui.collapsing("Projection", |ui| {
                    ui.label("Fov Y: ");
                    ui.add(Slider::new(&mut proj.fov_deg, 0.0..=180.0));
                });

                ui.collapsing("Lights", |ui| self.light_editor.ui(ui, renderer));

                ui.collapsing("Instances", |ui| {
                    point_slider(ui, &mut self.new_inst_pos, -10.0..=10.);
                    ui.add(
                        Slider::new(
                            &mut self.mat_id,
                            0..=renderer.entities.materials.len() as u32 - 1,
                        )
                        .text("Material ID"),
                    );
                    ui.add(
                        Slider::new(
                            &mut self.model_id,
                            0..=renderer.entities.models.model_count() as u32 - 1,
                        )
                        .text("Model ID"),
                    );
                    ui.add(
                        Slider::new(
                            &mut self.mesh_id,
                            0..=renderer.entities.models.mesh_count_of(self.model_id as u16) as u32
                                - 1,
                        )
                        .text("Mesh ID"),
                    );
                    if ui.button("Push").clicked() {
                        renderer.entities.models.add_instance(
                            self.model_id as u16,
                            self.mesh_id as u16,
                            ModelInstance {
                                transform: Matrix4::new_translation(&self.new_inst_pos.coords)
                                    .into(),
                                material_id: self.mat_id,
                            },
                        );
                    }
                })
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

fn vec3_slider(ui: &mut egui::Ui, value: &mut Vector3<f32>) {
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
        Slider::new(yaw, -90.0..=90.0)
            .text("Yaw")
            .text_color(Color32::RED),
    );
    ui.add(
        Slider::new(pitch, -90.0..=90.0)
            .text("Pitch")
            .text_color(Color32::GREEN),
    );
    ui.add(
        Slider::new(roll, -90.0..=90.0)
            .text("Roll")
            .text_color(Color32::CYAN),
    );
}
