use egui::{ComboBox, Slider};

use crate::{
    app::inputs::current,
    graphics::{light::Light, GlobalRenderer},
};

use super::{point_slider, vec3_slider};

#[derive(Default)]
pub struct LightEditor {
    current: Light,
    selection_id: usize,
}

impl LightEditor {
    pub fn ui(&mut self, ui: &mut egui::Ui, renderer: &mut GlobalRenderer) {
        let a = Light::None;
        let b = Light::default_point();
        let c = Light::default_directional();
        let d = Light::default_spotlight();

        egui::ComboBox::from_label("")
            .selected_text(format!("Light type: {}", self.current.label()))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut self.current, a, a.label());
                ui.selectable_value(&mut self.current, b, b.label());
                ui.selectable_value(&mut self.current, c, c.label());
                ui.selectable_value(&mut self.current, d, d.label());
            });

        ui.separator();

        match &mut self.current {
            Light::None => {
                ui.label("None");
            }
            Light::Point {
                color,
                intensity,
                position,
            } => {
                ui.heading("Pointlight");
                ui.label("Color: ");
                ui.color_edit_button_rgb(color.array_mut());
                ui.label("Intensity: ");
                ui.add(Slider::new(intensity, 0.0..=10.0));
                ui.label("Position: ");
                point_slider(ui, position, -10.0..=10.0);
            }
            Light::Directional {
                color,
                intensity,
                direction,
            } => {
                ui.heading("Directional Light");
                ui.label("Color: ");
                ui.color_edit_button_rgb(color.array_mut());
                ui.label("Intensity: ");
                ui.add(Slider::new(intensity, 0.0..=10.0));
                vec3_slider(ui, direction);
            }
            Light::Spotlight {
                color,
                intensity,
                position,
                direction,
                cut_off,
            } => {
                ui.heading("Spotlight");
                ui.label("Color: ");
                ui.color_edit_button_rgb(color.array_mut());
                ui.label("Intensity: ");
                ui.add(Slider::new(intensity, 0.0..=10.0));
                ui.label("Position: ");
                point_slider(ui, position, -10.0..=10.0);
                ui.label("Direction: ");
                vec3_slider(ui, direction);
                ui.add(Slider::new(cut_off, 0.0..=180.0).text("Cut off"));
            }
        }

        ui.separator();
        ui.label("Index: ");
        ui.add(Slider::new(
            &mut self.selection_id,
            0..=renderer.lights.storage_buffer.len() as usize - 1,
        ));
        ui.horizontal(|ui| {
            if self.selection_id <= renderer.lights.storage_buffer.len() as usize {
                if ui.button("Apply").clicked() {
                    renderer
                        .lights
                        .storage_buffer
                        .set(self.selection_id as u32, self.current.clone().into());
                }
            }
            if ui.button("Push").clicked() {
                renderer
                    .lights
                    .storage_buffer
                    .push(self.current.clone().into());
            }
        });
    }
}
