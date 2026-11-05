mod app;
mod camera;
mod color;
mod cosmos;
mod framebuffer;
mod obj;
mod scene;
mod shaders;
mod triangle;

use std::error::Error;
use std::f32::consts::PI;

const WIDTH: usize = 1280;
const HEIGHT: usize = 960;

const ROTATION_SPEED: f32 = PI / 60.0;

fn main() -> Result<(), Box<dyn Error>> {
    app::run()
}
