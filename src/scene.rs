use nalgebra_glm::Vec3;
use std::error::Error;

use crate::shaders::{Shader, light_direction};
use crate::triangle::{shaded_triangle, wireframe};
use crate::{camera::Camera, color::Color, framebuffer::Framebuffer, obj::Obj};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Body {
    Star,
    Rocky,
    Gas,
    Dendro,
    Ice,
    Desert,
}

impl Body {
    pub const ALL: [Self; 6] = [
        Self::Star,
        Self::Rocky,
        Self::Gas,
        Self::Dendro,
        Self::Ice,
        Self::Desert,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::Star => "Helios - Estrella",
            Self::Rocky => "Thalassa - Planeta rocoso",
            Self::Gas => "Aureon - Gigante gaseoso",
            Self::Dendro => "Verdantia - Planeta Dendro",
            Self::Ice => "Borealis - Planeta helado",
            Self::Desert => "Sirocco - Planeta desertico",
        }
    }

    pub fn slug(self) -> &'static str {
        match self {
            Self::Star => "estrella",
            Self::Rocky => "rocoso",
            Self::Gas => "gaseoso",
            Self::Dendro => "dendro",
            Self::Ice => "helado",
            Self::Desert => "desertico",
        }
    }

    fn radius(self, extras: bool) -> f32 {
        match (self, extras) {
            (Self::Rocky, true) => 2.0,
            (Self::Gas, true) => 2.3,
            _ => 1.18,
        }
    }
}

pub struct Models {
    star: Obj,
    rocky: Obj,
    gas: Obj,
    moon: Obj,
    rings: Obj,
}

impl Models {
    pub fn load() -> Result<Self, Box<dyn Error>> {
        let load =
            |name: &str| Obj::load(&format!("{}/models/{name}.obj", env!("CARGO_MANIFEST_DIR")));
        Ok(Self {
            star: load("estrella")?,
            rocky: load("rocoso")?,
            gas: load("gaseoso")?,
            moon: load("luna")?,
            rings: load("anillos")?,
        })
    }
}

pub fn camera() -> Camera {
    Camera::new(Vec3::new(0.0, 2.3, 6.0), Vec3::zeros(), Vec3::y())
}

struct Object<'a> {
    mesh: &'a Obj,
    shader: Shader,
    center: Vec3,
    scale: f32,
    angle: f32,
}

fn rotate(p: Vec3, angle: f32) -> Vec3 {
    let (s, c) = angle.sin_cos();
    Vec3::new(c * p.x + s * p.z, p.y, -s * p.x + c * p.z)
}

/// Una sola escala para toda la escena; luna, anillos y planeta comparten z-buffer.
pub fn render(
    fb: &mut Framebuffer,
    models: &Models,
    body: Body,
    camera: &Camera,
    time: f32,
    extras: bool,
    hologram: bool,
) {
    fb.clear();
    let mut objects = vec![Object {
        mesh: match body {
            Body::Star => &models.star,
            Body::Rocky | Body::Dendro | Body::Ice | Body::Desert => &models.rocky,
            Body::Gas => &models.gas,
        },
        shader: match body {
            Body::Star => Shader::Star,
            Body::Rocky => Shader::Rocky,
            Body::Gas => Shader::Gas,
            Body::Dendro => Shader::Dendro,
            Body::Ice => Shader::Ice,
            Body::Desert => Shader::Desert,
        },
        center: Vec3::zeros(),
        scale: 1.0,
        angle: time * 0.10,
    }];
    if extras && body == Body::Rocky {
        let angle = time * 0.22 + 0.3;
        objects.push(Object {
            mesh: &models.moon,
            shader: Shader::Moon,
            center: Vec3::new(1.60 * angle.cos(), 0.32, 1.60 * angle.sin()),
            scale: 0.24,
            angle: time * 0.10,
        });
    }
    if extras && body == Body::Gas {
        objects.push(Object {
            mesh: &models.rings,
            shader: Shader::Rings,
            center: Vec3::zeros(),
            scale: 1.0,
            angle: 0.0,
        });
    }
    let right = camera.basis_change(&Vec3::x());
    let up = camera.basis_change(&Vec3::y());
    let back = camera.basis_change(&Vec3::z());
    let scale = 0.86 * fb.width.min(fb.height) as f32 / (2.0 * body.radius(extras));
    for object in objects {
        let world: Vec<Vec3> = object
            .mesh
            .vertices
            .iter()
            .map(|p| rotate(*p * object.scale, object.angle) + object.center)
            .collect();
        let screen: Vec<Vec3> = world
            .iter()
            .map(|p| {
                let relative = p - camera.eye;
                Vec3::new(
                    fb.width as f32 * 0.5 + relative.dot(&right) * scale,
                    fb.height as f32 * 0.5 - relative.dot(&up) * scale,
                    -relative.dot(&back),
                )
            })
            .collect();
        for face in &object.mesh.faces {
            for i in 1..face.len() - 1 {
                let indices = [face[0], face[i], face[i + 1]];
                let projected = indices.map(|j| screen[j]);
                if hologram {
                    fb.set_current_color(Color::new(35, 195, 255).to_hex());
                    wireframe(fb, &projected[0], &projected[1], &projected[2]);
                    continue;
                }
                let local = indices.map(|j| object.mesh.vertices[j]);
                let face_normal = (local[1] - local[0]).cross(&(local[2] - local[0]));
                if face_normal.norm_squared() < 1e-12 {
                    continue;
                }
                let face_normal = rotate(face_normal.normalize(), object.angle);
                // Solo los anillos necesitan las dos caras; las esferas son cerradas.
                let view = back;
                if object.shader != Shader::Rings && face_normal.dot(&view) <= 0.0 {
                    continue;
                }
                shaded_triangle(fb, &projected, &local, |p| {
                    let is_ring = object.shader == Shader::Rings;
                    let normal = if is_ring {
                        if face_normal.dot(&view) < 0.0 {
                            -face_normal
                        } else {
                            face_normal
                        }
                    } else {
                        rotate(p.normalize(), object.angle)
                    };
                    // La proyección es ortográfica: todos los rayos de vista son paralelos.
                    let mut color = object.shader.shade(
                        if is_ring { p } else { p.normalize() },
                        normal,
                        back,
                        time,
                    );
                    if is_ring {
                        // Sombra del gigante sobre los anillos mediante intersección rayo-esfera.
                        let light = light_direction();
                        let along = p.dot(&light);
                        if along < 0.0 && p.norm_squared() - along * along < 1.0 {
                            color = color * 0.18;
                        }
                    }
                    color
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn background_is_stable_between_scenes_and_never_overwrites_planets() {
        let models = Models::load().unwrap();
        let background = crate::cosmos::generate(160, 120);
        let mut composed = Framebuffer::new(160, 120);
        composed.set_background(background.clone());
        let mut foreground = Framebuffer::new(160, 120);
        for body in Body::ALL {
            for time in [0.0, 5.0] {
                render(&mut foreground, &models, body, &camera(), time, true, false);
                render(&mut composed, &models, body, &camera(), time, true, false);
                for (i, pixel) in foreground.buffer.iter().enumerate() {
                    let expected = if *pixel == 0 { background[i] } else { *pixel };
                    assert_eq!(
                        composed.buffer[i], expected,
                        "{body:?}, tiempo {time}, pixel {i}"
                    );
                }
            }
        }
    }

    #[test]
    fn planet_hologram_shows_the_mesh_and_can_return_to_the_surface() {
        let models = Models::load().unwrap();
        let mut fb = Framebuffer::new(320, 240);
        render(&mut fb, &models, Body::Rocky, &camera(), 0.0, true, false);
        let surface = fb.buffer.clone();
        render(&mut fb, &models, Body::Rocky, &camera(), 0.0, true, true);
        assert_ne!(fb.buffer, surface);
        assert!(fb.buffer.iter().any(|pixel| *pixel != 0));
        assert!(
            fb.buffer
                .iter()
                .all(|pixel| (pixel & 255) >= ((pixel >> 16) & 255))
        );
        render(&mut fb, &models, Body::Rocky, &camera(), 0.0, true, false);
        assert_eq!(fb.buffer, surface);
    }

    #[test]
    fn planet_occludes_rear_rings_and_front_rings_occlude_planet() {
        let mut models = Models::load().unwrap();
        let mut fb = Framebuffer::new(320, 240);
        let faces = std::mem::take(&mut models.rings.faces);
        render(&mut fb, &models, Body::Gas, &camera(), 0.0, true, false);
        let planet = fb.buffer.clone();
        models.rings.faces = faces;
        render(&mut fb, &models, Body::Gas, &camera(), 0.0, true, false);
        assert_eq!(
            fb.buffer[96 * 320 + 160],
            planet[96 * 320 + 160],
            "el anillo trasero debe quedar detrás del planeta"
        );
        assert_ne!(
            fb.buffer[144 * 320 + 160],
            planet[144 * 320 + 160],
            "el anillo delantero debe quedar sobre el planeta"
        );
    }

    #[test]
    fn scenes_render_distinct_surfaces_and_optional_separate_objects() {
        let models = Models::load().unwrap();
        let mut fb = Framebuffer::new(320, 240);
        let mut images = Vec::new();
        for body in Body::ALL {
            render(&mut fb, &models, body, &camera(), 0.0, true, false);
            let painted = fb.buffer.iter().filter(|p| **p != 0).count();
            assert!(
                painted > 2000 && painted < 320 * 240 / 2,
                "{body:?}: {painted}"
            );
            assert!(fb.buffer[..320].iter().all(|p| *p == 0));
            images.push(fb.buffer.clone());
            if matches!(body, Body::Rocky | Body::Gas) {
                render(&mut fb, &models, body, &camera(), 0.0, false, false);
                assert_ne!(fb.buffer, *images.last().unwrap());
            }
        }
        for (i, image) in images.iter().enumerate() {
            for other in images.iter().skip(i + 1) {
                assert_ne!(
                    image, other,
                    "Los seis cuerpos deben tener diseños distintos"
                );
            }
        }
        render(&mut fb, &models, Body::Rocky, &camera(), 0.0, true, false);
        let initial = fb.buffer.clone();
        render(&mut fb, &models, Body::Rocky, &camera(), 8.0, true, false);
        assert_ne!(initial, fb.buffer);
    }

    #[test]
    fn moon_and_rings_paint_pixels_outside_the_planet_and_remain_in_frame_when_orbiting() {
        let models = Models::load().unwrap();
        let mut fb = Framebuffer::new(320, 240);
        let mut camera = camera();
        for body in [Body::Rocky, Body::Gas] {
            render(&mut fb, &models, body, &camera, 0.0, true, false);
            // El planeta solo nunca llega a x >= 220 con este encuadre.
            assert!(
                fb.buffer
                    .chunks(320)
                    .any(|row| row[220..].iter().any(|p| *p != 0))
            );
        }
        for _ in 0..12 {
            camera.orbit(0.5, 0.1);
            render(&mut fb, &models, Body::Gas, &camera, 2.0, true, false);
            assert!(fb.buffer[..320].iter().all(|p| *p == 0));
            assert!(fb.buffer[320 * 239..].iter().all(|p| *p == 0));
            assert!(
                fb.buffer
                    .chunks(320)
                    .all(|row| row[0] == 0 && row[319] == 0)
            );
        }
    }
}
