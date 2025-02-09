use std::time::Duration;

use nalgebra::Vector3;
use winit::keyboard::KeyCode;

use crate::{app::inputs::Inputs, graphics::camera::View};

pub struct GameState {
    pub view: View,
}

impl GameState {
    pub fn new() -> Self {
        Self {
            view: View::default(),
        }
    }

    pub fn update(&mut self, inputs: &Inputs, dt: Duration) -> () {
        let (dx, dy) = inputs.mouse_diff();

        let sensitivity = 0.5;
        let speed = 2.0;

        let dts = dt.as_secs_f32();

        /*self.view.pitch_deg += dy * sensitivity * dts;
        self.view.yaw_deg += dx * sensitivity * dts; */

        let (yaw_sin, yaw_cos) = (self.view.yaw_deg - 90.).to_radians().sin_cos();
        let up = self.view.up * speed * dts;
        let forward = Vector3::new(yaw_cos, 0.0, yaw_sin).normalize() * speed * dts;
        let right = Vector3::new(-yaw_sin, 0.0, yaw_cos).normalize() * speed * dts;

        if inputs.key_held(KeyCode::KeyW) {
            self.view.eye += forward;
        }
        if inputs.key_held(KeyCode::KeyS) {
            self.view.eye -= forward;
        }
        if inputs.key_held(KeyCode::KeyD) {
            self.view.eye += right;
        }
        if inputs.key_held(KeyCode::KeyA) {
            self.view.eye -= right;
        }
        if inputs.key_held(KeyCode::Space) {
            self.view.eye += up;
        }
        if inputs.key_held(KeyCode::ShiftLeft) {
            self.view.eye -= up;
        }
    }
}
