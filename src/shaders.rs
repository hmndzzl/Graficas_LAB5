use crate::color::Color;
use nalgebra_glm::Vec3;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Shader {
    Star,
    Rocky,
    Gas,
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

fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn hash(x: i32, y: i32, z: i32) -> f32 {
    let mut h = (x as u32).wrapping_mul(374761393)
        ^ (y as u32).wrapping_mul(668265263)
        ^ (z as u32).wrapping_mul(2147483647);
    h = (h ^ (h >> 13)).wrapping_mul(1274126177);
    (h ^ (h >> 16)) as f32 / u32::MAX as f32
}

/// Value noise tridimensional: no hay costura de UV ni imágenes precalculadas.
fn noise(p: Vec3) -> f32 {
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

fn fbm(mut p: Vec3) -> f32 {
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
            Self::Star => (star(p, time, rim), true),
            Self::Rocky => (rocky(p, time, normal, view), false),
            Self::Gas => (gas(p), false),
            Self::Moon => (moon(p), false),
            Self::Rings => (rings(p), false),
        };
        let mut color = if emission {
            surface
        } else {
            surface * (0.12 + 0.88 * diffuse)
        };
        if self == Self::Rocky {
            color += rgb(0x399DFF) * rim * (0.18 + 0.55 * diffuse);
        } else if self == Self::Gas {
            color += rgb(0xF2B884) * rim * 0.28;
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
