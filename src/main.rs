mod camera;
mod color;
mod framebuffer;
mod obj;
mod triangle;

use minifb::{Key, KeyRepeat, Window, WindowOptions};
use nalgebra_glm::Vec3;
use std::error::Error;
use std::f32::consts::PI;
use std::time::Duration;

use crate::camera::Camera;
use crate::color::Color;
use crate::framebuffer::Framebuffer;
use crate::obj::Obj;
use crate::triangle::{triangle, wireframe};

const WIDTH: usize = 1280;
const HEIGHT: usize = 960;
const BACKGROUND_COLOR: u32 = 0x040C24;

const ROTATION_SPEED: f32 = PI / 60.0;

fn create_camera(model: &Obj) -> Camera {
    let mut min = model.vertices[0];
    let mut max = model.vertices[0];
    for vertex in &model.vertices {
        min = min.zip_map(vertex, f32::min);
        max = max.zip_map(vertex, f32::max);
    }

    let center = (min + max) / 2.0;
    Camera::new(
        center + Vec3::new(-0.4, 0.5, 0.8) * (max - min).norm(),
        center,
        Vec3::new(0.0, 1.0, 0.0),
    )
}

pub fn render(framebuffer: &mut Framebuffer, model: &Obj, camera: &Camera, hologram: bool) {
    let right = camera.basis_change(&Vec3::new(1.0, 0.0, 0.0));
    let up = camera.basis_change(&Vec3::new(0.0, 1.0, 0.0));
    let backward = camera.basis_change(&Vec3::new(0.0, 0.0, 1.0));

    let vertices: Vec<Vec3> = model
        .vertices
        .iter()
        .map(|vertex| {
            let relative = vertex - camera.eye;
            Vec3::new(
                relative.dot(&right),
                relative.dot(&up),
                relative.dot(&backward),
            )
        })
        .collect();

    // El radio mantiene la misma escala y el modelo completo durante toda la órbita.
    let radius = model
        .vertices
        .iter()
        .map(|vertex| (vertex - camera.center).norm())
        .fold(0.0, f32::max);
    let scale =
        0.85 * framebuffer.width.min(framebuffer.height) as f32 / (2.0 * radius.max(f32::EPSILON));

    let screen_vertices: Vec<Vec3> = vertices
        .iter()
        .map(|vertex| {
            let vertex = vertex * scale;
            Vec3::new(
                framebuffer.width as f32 / 2.0 + vertex.x,
                framebuffer.height as f32 / 2.0 - vertex.y,
                -vertex.z,
            )
        })
        .collect();

    let light_direction = Vec3::new(-0.3, 0.8, 1.0).normalize();
    let halfway = (light_direction + Vec3::new(0.0, 0.0, 1.0)).normalize();
    let color = Color::new(230, 90, 24);
    let highlight = Color::new(255, 210, 155);

    for face in &model.faces {
        // Se toman manualmente los índices de cada cara para formar sus triángulos.
        for index in 1..face.len() - 1 {
            if hologram {
                // Sin ocultar las aristas traseras para poder inspeccionar toda la malla.
                framebuffer.set_current_color(Color::new(35, 195, 255).to_hex());
                wireframe(
                    framebuffer,
                    &screen_vertices[face[0]],
                    &screen_vertices[face[index]],
                    &screen_vertices[face[index + 1]],
                );
                continue;
            }

            let a = vertices[face[0]];
            let b = vertices[face[index]];
            let c = vertices[face[index + 1]];
            let normal = (b - a).cross(&(c - a));
            if normal.norm_squared() < f32::EPSILON {
                continue;
            }

            let normal = normal.normalize();
            let normal = if normal.z < 0.0 { -normal } else { normal };
            let diffuse = normal.dot(&light_direction).max(0.0);
            let specular = normal.dot(&halfway).max(0.0).powf(48.0);
            let reflection = (0.5 + 0.5 * normal.y).powf(6.0);
            let surface =
                color * (0.18 + 0.7 * diffuse) + highlight * (0.85 * specular + 0.25 * reflection);
            framebuffer.set_current_color(surface.to_hex());
            triangle(
                framebuffer,
                &screen_vertices[face[0]],
                &screen_vertices[face[index]],
                &screen_vertices[face[index + 1]],
            );
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let frame_delay = Duration::from_millis(16);
    let model = Obj::load(concat!(env!("CARGO_MANIFEST_DIR"), "/Benatar.obj"))?;
    let mut camera = create_camera(&model);

    let mut framebuffer = Framebuffer::new(WIDTH, HEIGHT);
    framebuffer.set_background_color(Color::from_hex(BACKGROUND_COLOR).to_hex());

    let mut window = Window::new(
        "Benatar - Metal anaranjado | H: holograma | Flechas: orbitar",
        WIDTH,
        HEIGHT,
        WindowOptions::default(),
    )?;

    let mut camera_moved = true;
    let mut hologram = false;

    while window.is_open() && !window.is_key_down(Key::Escape) {
        if window.is_key_pressed(Key::H, KeyRepeat::No) {
            hologram = !hologram;
            camera_moved = true;
            window.set_title(if hologram {
                "Benatar - Holograma | H: metal anaranjado | Flechas: orbitar"
            } else {
                "Benatar - Metal anaranjado | H: holograma | Flechas: orbitar"
            });
        }

        let orbit = [
            (Key::Left, -ROTATION_SPEED, 0.0),
            (Key::Right, ROTATION_SPEED, 0.0),
            (Key::Up, 0.0, -ROTATION_SPEED),
            (Key::Down, 0.0, ROTATION_SPEED),
        ];

        for (key, delta_yaw, delta_pitch) in orbit {
            if window.is_key_down(key) {
                camera.orbit(delta_yaw, delta_pitch);
                camera_moved = true;
            }
        }

        if camera_moved {
            framebuffer.clear();
            render(&mut framebuffer, &model, &camera, hologram);
            camera_moved = false;
        }

        window.update_with_buffer(&framebuffer.buffer, WIDTH, HEIGHT)?;
        std::thread::sleep(frame_delay);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hologram_shows_the_mesh_instead_of_solid_surfaces() {
        let model = Obj::load(concat!(env!("CARGO_MANIFEST_DIR"), "/Benatar.obj")).unwrap();
        let camera = create_camera(&model);
        let mut framebuffer = Framebuffer::new(WIDTH, HEIGHT);
        render(&mut framebuffer, &model, &camera, false);
        let solid = framebuffer.buffer.clone();

        framebuffer.clear();
        render(&mut framebuffer, &model, &camera, true);
        assert!(framebuffer.buffer != solid);
        assert!(framebuffer.buffer.iter().any(|pixel| *pixel != 0));
        assert!(framebuffer.buffer.iter().all(|pixel| {
            let red = (pixel >> 16) & 0xFF;
            let blue = pixel & 0xFF;
            blue >= red
        }));

        framebuffer.clear();
        render(&mut framebuffer, &model, &camera, false);
        assert!(framebuffer.buffer == solid);
    }

    #[test]
    fn renders_the_benatar_model_inside_the_framebuffer() {
        let model = Obj::load(concat!(env!("CARGO_MANIFEST_DIR"), "/Benatar.obj")).unwrap();
        assert_eq!(model.vertices.len(), 1186);
        assert_eq!(model.faces.len(), 2340);

        let mut framebuffer = Framebuffer::new(WIDTH, HEIGHT);
        let mut camera = create_camera(&model);
        render(&mut framebuffer, &model, &camera, false);

        let painted = framebuffer
            .buffer
            .iter()
            .filter(|pixel| **pixel != 0)
            .count();
        assert!(painted > 10_000);
        assert!(painted < WIDTH * HEIGHT / 2);
        assert!(framebuffer.buffer[..WIDTH].iter().all(|pixel| *pixel == 0));
        assert!(
            framebuffer.buffer[(HEIGHT - 1) * WIDTH..]
                .iter()
                .all(|pixel| *pixel == 0)
        );

        let initial_view = framebuffer.buffer.clone();
        let radius = (camera.eye - camera.center).norm();
        let center = camera.center;
        camera.orbit(0.5, -0.2);
        framebuffer.clear();
        render(&mut framebuffer, &model, &camera, false);

        assert!(framebuffer.buffer != initial_view);
        assert!(((camera.eye - center).norm() - radius).abs() < 1e-4);
        assert_eq!(camera.center, center);

        for pitch in [-PI, PI] {
            camera.orbit(0.0, pitch);
            for _ in 0..8 {
                camera.orbit(PI / 4.0, 0.0);
                framebuffer.clear();
                render(&mut framebuffer, &model, &camera, false);

                assert!(framebuffer.buffer.iter().any(|pixel| *pixel != 0));
                assert!(framebuffer.buffer[..WIDTH].iter().all(|pixel| *pixel == 0));
                assert!(
                    framebuffer.buffer[(HEIGHT - 1) * WIDTH..]
                        .iter()
                        .all(|pixel| *pixel == 0)
                );
                assert!(
                    framebuffer
                        .buffer
                        .chunks(WIDTH)
                        .all(|row| row[0] == 0 && row[WIDTH - 1] == 0)
                );
                assert!(((camera.eye - center).norm() - radius).abs() < 1e-4);
            }
        }
    }
}
