use crate::color::Color;
use nalgebra_glm::Vec3;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Shader {
    Star,
    Rocky,
    Gas,
    Dendro,
    Ice,
    Desert,
    Moon,
    Rings,
}

pub fn light_direction() -> Vec3 {
    Vec3::new(-0.6, 0.8, 1.0).normalize()
}

fn rgb(hex: u32) -> Vec3 {
    Vec3::new(
        ((hex >> 16) & 255) as f32,
        ((hex >> 8) & 255) as f32,
        (hex & 255) as f32,
    )
}

fn mix(a: Vec3, b: Vec3, t: f32) -> Vec3 {
    a + (b - a) * t.clamp(0.0, 1.0)
}

pub(crate) fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

pub(crate) fn hash(x: i32, y: i32, z: i32) -> f32 {
    let mut h = (x as u32).wrapping_mul(374761393)
        ^ (y as u32).wrapping_mul(668265263)
        ^ (z as u32).wrapping_mul(2147483647);
    h = (h ^ (h >> 13)).wrapping_mul(1274126177);
    (h ^ (h >> 16)) as f32 / u32::MAX as f32
}

/// Value noise tridimensional: no hay costura de UV ni imágenes precalculadas.
pub(crate) fn noise(p: Vec3) -> f32 {
    let base = p.map(|v| v.floor());
    let f = (p - base).map(|v| v * v * (3.0 - 2.0 * v));
    let mut result = 0.0;
    for z in 0..2 {
        for y in 0..2 {
            for x in 0..2 {
                let weight = if x == 0 { 1.0 - f.x } else { f.x }
                    * if y == 0 { 1.0 - f.y } else { f.y }
                    * if z == 0 { 1.0 - f.z } else { f.z };
                result += weight * hash(base.x as i32 + x, base.y as i32 + y, base.z as i32 + z);
            }
        }
    }
    result
}

pub(crate) fn fbm(mut p: Vec3) -> f32 {
    let mut value = 0.0;
    let mut weight = 0.5;
    for _ in 0..4 {
        value += noise(p) * weight;
        p = p * 2.03 + Vec3::new(13.1, 7.7, 19.3);
        weight *= 0.5;
    }
    value / 0.9375
}

impl Shader {
    /// p: posición local. normal y view: direcciones unitarias en espacio mundo.
    pub fn shade(self, p: Vec3, normal: Vec3, view: Vec3, time: f32) -> Color {
        let light = light_direction();
        let diffuse = normal.dot(&light).max(0.0);
        let rim = (1.0 - normal.dot(&view).abs()).clamp(0.0, 1.0).powi(3);
        let (surface, emission) = match self {
            Self::Star => (Vec3::zeros(), star(p, time, rim)),
            Self::Rocky => (rocky(p, time, normal, view), Vec3::zeros()),
            Self::Gas => (gas(p), Vec3::zeros()),
            Self::Dendro => (dendro(p, time), Vec3::zeros()),
            Self::Ice => (ice(p, normal, view), Vec3::zeros()),
            Self::Desert => (desert(p, time), Vec3::zeros()),
            Self::Moon => (moon(p), Vec3::zeros()),
            Self::Rings => (rings(p), Vec3::zeros()),
        };
        let mut color = surface * (0.12 + 0.88 * diffuse) + emission;
        if self == Self::Rocky {
            color += rgb(0x399DFF) * rim * (0.18 + 0.55 * diffuse);
        } else if self == Self::Gas {
            color += rgb(0xF2B884) * rim * 0.28;
        } else if self == Self::Dendro {
            color += rgb(0x759BA1) * rim * 0.12 * diffuse;
        } else if self == Self::Ice {
            color += rgb(0x4ACDFF) * rim * 0.35;
        } else if self == Self::Desert {
            color += rgb(0xE89A52) * rim * 0.18;
        }
        Color::new(
            color.x.clamp(0.0, 255.0) as u8,
            color.y.clamp(0.0, 255.0) as u8,
            color.z.clamp(0.0, 255.0) as u8,
        )
    }
}

fn star(p: Vec3, time: f32, rim: f32) -> Vec3 {
    let flow = p * 5.0 + Vec3::new(time * 0.08, time * -0.04, 0.0);
    // 1. Plasma multiescala, de rojo profundo a naranja.
    let plasma = fbm(flow);
    let mut c = mix(rgb(0xA51A08), rgb(0xFFB52E), smoothstep(0.2, 0.7, plasma));
    // 2. Granulación caliente en la fotosfera.
    let grains = noise(p * 65.0 + Vec3::repeat(time * 0.12));
    c = mix(c, rgb(0xFFF2AF), smoothstep(0.52, 0.85, grains) * 0.7);
    // 3. Manchas solares oscuras con penumbra.
    let spots = 1.0 - smoothstep(0.24, 0.37, fbm(flow * 0.7 + Vec3::repeat(12.0)));
    c = mix(c, rgb(0x4B110E), spots * 0.92);
    // 4. Filamentos brillantes que siguen el campo de ruido.
    let filaments = (p.y * 24.0 + plasma * 22.0 + time * 0.2).sin().abs();
    c = mix(
        c,
        rgb(0xFFE47F),
        (1.0 - smoothstep(0.025, 0.12, filaments)) * 0.45,
    );
    // Oscurecimiento hacia el limbo sin luz externa; brillo cálido en el borde.
    c * (1.0 - 0.28 * rim) + rgb(0xFF690A) * rim * 0.14
}

fn rocky(p: Vec3, time: f32, normal: Vec3, view: Vec3) -> Vec3 {
    let height = fbm(p * 2.6 + Vec3::new(3.7, 1.1, -2.4));
    let land = smoothstep(0.48, 0.515, height);
    // 1. Océano profundo y aguas turquesas cerca de las costas.
    let sea = mix(rgb(0x052044), rgb(0x19A6B2), smoothstep(0.33, 0.50, height));
    // 2. Continentes verdes, desiertos cálidos y montañas.
    let detail = fbm(p * 14.0);
    let desert = smoothstep(0.46, 0.64, detail) * (1.0 - smoothstep(0.2, 0.65, p.y.abs()));
    let mut terrain = mix(rgb(0x296749), rgb(0xC8A06A), desert);
    terrain = mix(terrain, rgb(0x727779), smoothstep(0.61, 0.72, height));
    let mut c = mix(sea, terrain, land);
    // 3. Casquetes polares irregulares y nieve en altura.
    let ice = smoothstep(0.77, 0.90, p.y.abs() + (detail - 0.5) * 0.16);
    c = mix(c, rgb(0xDAF5FF), ice);
    // 4. Nubes procedurales con desplazamiento independiente.
    let clouds = fbm(p * 5.5 + Vec3::new(time * 0.025, 4.0, 0.0));
    c = mix(c, rgb(0xEDF5FF), smoothstep(0.54, 0.69, clouds) * 0.85);
    // Reflejo especular restringido al agua y atenuado bajo nubes/hielo.
    let halfway = (light_direction() + view).normalize();
    let specular = normal.dot(&halfway).max(0.0).powi(70);
    c + rgb(0xCDEEFF) * specular * (1.0 - land) * (1.0 - ice) * 0.7
}

fn gas(p: Vec3) -> Vec3 {
    let turbulence = fbm(p * 5.0 + Vec3::repeat(7.0));
    // 1. Bandas latitudinales deformadas por turbulencia.
    let bands = 0.5 + 0.5 * (p.y * 36.0 + turbulence * 6.0).sin();
    let mut c = mix(rgb(0x805076), rgb(0xF8D9A2), smoothstep(0.1, 0.9, bands));
    // 2. Corrientes finas y filamentos entre las bandas.
    let fine = 0.5 + 0.5 * (p.y * 135.0 + turbulence * 12.0).sin();
    c = mix(c, rgb(0xC08352), fine * 0.30);
    // 3. Remolinos nubosos con detalle multiescala.
    let clouds = fbm(p * 18.0);
    c = mix(c, rgb(0xFFF1CD), smoothstep(0.50, 0.76, clouds) * 0.38);
    // 4. Gran tormenta ovalada; distancia angular envuelta en la longitud.
    let longitude = p.x.atan2(p.z);
    let delta = (longitude - 0.45).sin().atan2((longitude - 0.45).cos());
    let x = delta / 0.32;
    let y = (p.y - 0.20) / 0.13;
    let radius = (x * x + y * y).sqrt();
    let swirl = 0.5 + 0.5 * (radius * 20.0 + y.atan2(x) * 2.0 + turbulence * 4.0).sin();
    let storm = mix(rgb(0xA52B41), rgb(0xF5A668), swirl);
    mix(c, storm, 1.0 - smoothstep(0.75, 1.15, radius))
}

/// Diferencia entre las dos semillas más cercanas de celdas Voronoi 3D.
/// Cerca de una frontera vale cero; permite dibujar placas sin costuras UV.
fn cell_edges(p: Vec3) -> f32 {
    let cell = p.map(|v| v.floor());
    let mut nearest = f32::INFINITY;
    let mut second = f32::INFINITY;
    for z in -1..=1 {
        for y in -1..=1 {
            for x in -1..=1 {
                let c = cell + Vec3::new(x as f32, y as f32, z as f32);
                let (cx, cy, cz) = (c.x as i32, c.y as i32, c.z as i32);
                let seed = c + Vec3::new(
                    hash(cx, cy, cz),
                    hash(cx + 53, cy, cz + 19),
                    hash(cx, cy + 71, cz + 37),
                );
                let d = (p - seed).norm_squared();
                if d < nearest {
                    second = nearest;
                    nearest = d;
                } else if d < second {
                    second = d;
                }
            }
        }
    }
    second.sqrt() - nearest.sqrt()
}

/// Un mundo de bosques y agua dulce: todo el color responde a la luz externa.
fn dendro(p: Vec3, time: f32) -> Vec3 {
    let height = fbm(p * 3.2 + Vec3::new(9.0, 21.0, 4.0));
    let humidity = fbm(p * 6.0 + Vec3::new(3.0, 17.0, 29.0));
    // 1. Bosques templados y selvas, con copas de árboles a distintas escalas.
    let forest = mix(rgb(0x527B3D), rgb(0x205A37), smoothstep(0.3, 0.7, humidity));
    let crowns = noise(p * 76.0 + Vec3::repeat(11.0));
    let groves = fbm(p * 22.0 + Vec3::new(5.0, 11.0, 31.0));
    let mut surface = forest * (0.76 + 0.38 * crowns + 0.24 * groves);
    // 2. Praderas y claros irregulares entre los bosques.
    let clearings = smoothstep(0.53, 0.68, groves) * (1.0 - smoothstep(0.54, 0.67, height));
    surface = mix(surface, rgb(0x8A9853), clearings * 0.85);
    // 3. Cordilleras rocosas: el relieve se representa únicamente con colores.
    let ridges = 1.0 - (fbm(p * 11.0 + Vec3::repeat(33.0)) * 2.0 - 1.0).abs();
    let mountains = smoothstep(0.59, 0.72, height + ridges * 0.07);
    let rock = mix(rgb(0x656F5D), rgb(0xA9AC96), smoothstep(0.5, 0.9, ridges));
    surface = mix(surface, rock, mountains * 0.85);
    // 4. Ríos y lagos azul verdoso que recorren los valles.
    let valley = fbm(p * 5.0 + Vec3::new(17.0, 3.0, 8.0));
    let river = (1.0 - smoothstep(0.002, 0.010, (valley - 0.48).abs()))
        * (1.0 - smoothstep(0.55, 0.64, height));
    let lakes = 1.0 - smoothstep(0.33, 0.39, height);
    let water = river.max(lakes);
    let banks = (1.0 - smoothstep(0.010, 0.022, (valley - 0.48).abs())) * (1.0 - mountains);
    surface = mix(surface, rgb(0x728B57), banks * 0.35);
    surface = mix(
        surface,
        mix(rgb(0x183F4B), rgb(0x35787D), humidity),
        water * 0.95,
    );
    // Bruma tenue sobre los bosques, sin filamentos ni emisión luminosa.
    let mist = smoothstep(
        0.55,
        0.72,
        fbm(p * 5.0 + Vec3::new(time * 0.018, 27.0, 13.0)),
    );
    mix(surface, rgb(0xD5DFCF), mist * 0.16)
}

fn ice(p: Vec3, normal: Vec3, view: Vec3) -> Vec3 {
    let terrain = fbm(p * 4.0 + Vec3::new(4.0, -17.0, 26.0));
    // 1. Glaciares de tonos azul pálido y blanco.
    let mut c = mix(
        rgb(0x74ABCE),
        rgb(0xDDEEF5),
        smoothstep(0.25, 0.72, terrain),
    );
    // 2. Grietas azules entre placas de hielo deformadas por ruido.
    let warp =
        Vec3::new(terrain, noise(p * 7.0), noise(p * 7.0 + Vec3::repeat(9.0))) - Vec3::repeat(0.5);
    let edge = cell_edges(p * 4.8 + warp * 0.75);
    let cracks = 1.0 - smoothstep(0.015, 0.10, edge);
    c = mix(c, rgb(0x146392), cracks * 0.9);
    // 3. Escarcha granular sobre los glaciares.
    let frost = smoothstep(0.47, 0.69, fbm(p * 28.0 + Vec3::repeat(13.0)));
    c = mix(c, rgb(0xF4FBFF), frost * (1.0 - cracks) * 0.65);
    // 4. Depósitos minerales violetas atrapados bajo el hielo.
    let minerals = smoothstep(0.54, 0.70, fbm(p * 3.0 + Vec3::repeat(43.0)));
    c = mix(c, rgb(0xA18DC7), minerals * 0.48);
    let halfway = (light_direction() + view).normalize();
    c + rgb(0xE0F6FF) * normal.dot(&halfway).max(0.0).powi(90) * (1.0 - cracks) * 0.65
}

fn desert(p: Vec3, time: f32) -> Vec3 {
    let terrain = fbm(p * 3.5 + Vec3::new(19.0, 4.0, 8.0));
    // 1. Estratos de arenisca y mesetas de terracota.
    let mut c = mix(
        rgb(0x934B35),
        rgb(0xE7B36B),
        smoothstep(0.25, 0.72, terrain),
    );
    // 2. Dunas finas y sinuosas orientadas por un campo de ruido.
    let dune = 0.5 + 0.5 * (p.x * 105.0 + p.z * 34.0 + fbm(p * 7.0) * 19.0).sin();
    c = mix(c, rgb(0xF4CC86), smoothstep(0.55, 0.96, dune) * 0.38);
    // 3. Cañones oscuros y salares en los polos.
    let canyon = 1.0 - smoothstep(0.012, 0.045, (terrain - 0.50).abs());
    c = mix(c, rgb(0x60312D), canyon * 0.78);
    let salt = smoothstep(0.88, 0.98, p.y.abs() + (terrain - 0.5) * 0.2);
    c = mix(c, rgb(0xE8D9B7), salt);
    // 4. Tormentas de polvo que cruzan lentamente las dunas.
    let dust = smoothstep(
        0.50,
        0.69,
        fbm(p * 5.0 + Vec3::new(time * 0.03, 23.0, -5.0)),
    );
    mix(c, rgb(0xD6A16C), dust * 0.6)
}

fn moon(p: Vec3) -> Vec3 {
    let maria = fbm(p * 4.0 + Vec3::repeat(31.0));
    let mut c = mix(rgb(0x555569), rgb(0xC4C0B7), smoothstep(0.3, 0.65, maria));
    c *= 0.8 + 0.35 * noise(p * 70.0);
    // Cráteres por distancia sobre la esfera, sin modificar su geometría.
    for i in 0..18 {
        let center = Vec3::new(
            hash(i, 0, 1) * 2.0 - 1.0,
            hash(i, 2, 3) * 2.0 - 1.0,
            hash(i, 4, 5) * 2.0 - 1.0,
        )
        .normalize();
        let radius = 0.06 + hash(i, 6, 7) * 0.16;
        let d = (p - center).norm() / radius;
        c *= 1.0 - (1.0 - smoothstep(0.65, 0.90, d)) * 0.45;
        c += rgb(0x9B9591) * (1.0 - smoothstep(0.05, 0.20, (d - 1.0).abs())) * 0.38;
    }
    c
}

fn rings(p: Vec3) -> Vec3 {
    let radius = (p.x * p.x + p.z * p.z).sqrt();
    let bands = 0.5 + 0.5 * (radius * 165.0).sin();
    let fine = noise(p * 45.0);
    mix(rgb(0x705A76), rgb(0xDCC5A3), bands) * (0.8 + fine * 0.25)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dendro_surface_is_dark_without_external_light() {
        let light = light_direction();
        for i in 0..100 {
            let p = Vec3::new((i as f32 * 2.4).sin(), (i as f32 * 0.7).cos(), 1.0).normalize();
            let brightness = |c: u32| ((c >> 16) & 255) + ((c >> 8) & 255) + (c & 255);
            let day = brightness(Shader::Dendro.shade(p, light, light, 0.0).to_hex());
            let night = brightness(Shader::Dendro.shade(p, -light, light, 0.0).to_hex());
            assert!(
                night as f32 <= day as f32 * 0.2 + 3.0,
                "La vegetación no debe emitir luz propia"
            );
        }
    }

    #[test]
    fn extra_planets_have_distinct_palettes() {
        use std::collections::HashSet;
        let view = Vec3::z();
        let mut patterns = Vec::new();
        for shader in [Shader::Dendro, Shader::Ice, Shader::Desert] {
            let mut colors = HashSet::new();
            let mut red = 0_u32;
            let mut blue = 0_u32;
            let mut green = 0_u32;
            for i in 0..200 {
                let p = Vec3::new((i as f32 * 2.4).sin(), (i as f32 * 0.7).cos(), 1.0).normalize();
                let color = shader.shade(p, p, view, 0.0).to_hex();
                colors.insert(color);
                red += (color >> 16) & 255;
                blue += color & 255;
                green += (color >> 8) & 255;
            }
            assert!(colors.len() > 100, "{shader:?}: variedad insuficiente");
            if shader == Shader::Dendro {
                assert!(
                    green > red && green > blue,
                    "Dendro debe tener una paleta verde"
                );
            } else if shader == Shader::Ice {
                assert!(blue > red);
            } else {
                assert!(red > blue);
            }
            patterns.push(colors);
        }
        assert_ne!(patterns[0], patterns[1]);
        assert_ne!(patterns[1], patterns[2]);
    }

    #[test]
    fn noise_is_continuous_bounded_and_repeatable_in_negative_coordinates() {
        for i in -20..20 {
            let p = Vec3::new(i as f32 * 0.13, -0.72, 1.31);
            let value = fbm(p);
            assert!((0.0..=1.0).contains(&value));
            assert_eq!(value, fbm(p));
            assert!((value - fbm(p + Vec3::new(0.0001, 0.0, 0.0))).abs() < 0.001);
        }
    }

    #[test]
    fn surfaces_have_distinct_patterns_and_the_star_does_not_need_external_light() {
        use std::collections::HashSet;
        let view = Vec3::new(0.0, 0.0, 1.0);
        let mut patterns = Vec::new();
        for shader in [Shader::Star, Shader::Rocky, Shader::Gas] {
            let mut colors = HashSet::new();
            for i in 0..100 {
                let p = Vec3::new((i as f32 * 2.4).sin(), (i as f32 * 0.7).cos(), 1.0).normalize();
                colors.insert(shader.shade(p, p, view, 0.0).to_hex());
            }
            assert!(colors.len() > 80);
            patterns.push(colors);
        }
        assert_ne!(patterns[0], patterns[1]);
        assert_ne!(patterns[1], patterns[2]);
        let night = -light_direction();
        let c = Shader::Star.shade(night, night, night, 0.0).to_hex();
        assert!(((c >> 16) & 255) > 80);
        assert_ne!(
            Shader::Star.shade(view, view, view, 0.0).to_hex(),
            Shader::Star.shade(view, view, view, 8.0).to_hex()
        );
    }
}
