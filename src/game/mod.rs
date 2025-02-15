use std::time::Duration;

use nalgebra::{Rotation3, Vector3, Vector4};
use winit::keyboard::KeyCode;

use crate::{app::inputs::Inputs, graphics::camera::Camera};

pub struct GameState {
    pub camera: Camera,
    pub paused: bool,
}

impl GameState {
    pub fn new() -> Self {
        Self {
            camera: Camera::default(),
            paused: false,
        }
    }

    pub fn update(&mut self, inputs: &Inputs, dt: Duration) -> () {
        let (dx, dy) = inputs.mouse_diff();

        let sensitivity = 2.;
        let speed = 3.;

        let dts = dt.as_secs_f32();
        if !self.paused {
            self.camera.yaw_deg -= dx * sensitivity * dts;
            self.camera.pitch_deg =
                (self.camera.pitch_deg - dy * sensitivity * dts).clamp(-90., 90.);
        }

        #[rustfmt::skip]
        let (forward, right, up) =(
            if inputs.key_held(KeyCode::KeyW) { 1. } else { 0. } + if inputs.key_held(KeyCode::KeyS) { -1. } else { 0. },
            if inputs.key_held(KeyCode::KeyD) { 1. } else { 0. } + if inputs.key_held(KeyCode::KeyA) { -1. } else { 0. },
            if inputs.key_held(KeyCode::Space) { 1. } else { 0. } + if inputs.key_held(KeyCode::ShiftLeft) { -1. } else { 0. },
        );

        let transl = Vector4::new(right, up, -forward, 0.);
        let rot = Rotation3::from_axis_angle(&Vector3::y_axis(), self.camera.yaw_deg.to_radians())
            .to_homogeneous();
        self.camera.eye += (rot * transl).xyz() * speed * dts;

        if inputs.key_pressed(KeyCode::Escape) {
            self.paused = !self.paused;
        }
    }
}
