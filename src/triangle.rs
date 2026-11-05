use nalgebra_glm::Vec3;

use crate::color::Color;
use crate::framebuffer::Framebuffer;

/// Proyección ortográfica: las coordenadas locales se interpolan linealmente.
pub fn shaded_triangle(
    framebuffer: &mut Framebuffer,
    screen: &[Vec3; 3],
    local: &[Vec3; 3],
    shader: impl Fn(Vec3) -> Color,
) {
    let [a, b, c] = screen;
    let area = edge(a, b, c);
    if area.abs() < f32::EPSILON {
        return;
    }
    let min_x = a.x.min(b.x).min(c.x).floor().max(0.0) as usize;
    let max_x = a.x.max(b.x).max(c.x).ceil().min(framebuffer.width as f32) as usize;
    let min_y = a.y.min(b.y).min(c.y).floor().max(0.0) as usize;
    let max_y = a.y.max(b.y).max(c.y).ceil().min(framebuffer.height as f32) as usize;
    for y in min_y..max_y {
        for x in min_x..max_x {
            let point = Vec3::new(x as f32 + 0.5, y as f32 + 0.5, 0.0);
            let wa = edge(b, c, &point) / area;
            let wb = edge(c, a, &point) / area;
            let wc = 1.0 - wa - wb;
            let depth = wa * a.z + wb * b.z + wc * c.z;
            if wa >= 0.0 && wb >= 0.0 && wc >= 0.0 && framebuffer.is_visible(x, y, depth) {
                let color = shader(local[0] * wa + local[1] * wb + local[2] * wc);
                framebuffer.set_current_color(color.to_hex());
                framebuffer.point_with_depth(x, y, depth);
            }
        }
    }
}

fn edge(a: &Vec3, b: &Vec3, point: &Vec3) -> f32 {
    (b.x - a.x) * (point.y - a.y) - (b.y - a.y) * (point.x - a.x)
}

pub fn wireframe(framebuffer: &mut Framebuffer, a: &Vec3, b: &Vec3, c: &Vec3) {
    for (start, end) in [(a, b), (b, c), (c, a)] {
        let delta = end - start;
        let steps = delta.x.abs().max(delta.y.abs()).ceil() as usize;
        for step in 0..=steps {
            let point = start + delta * (step as f32 / steps.max(1) as f32);
            let x = point.x.round() as i32;
            let y = point.y.round() as i32;

            // Un halo suave alrededor de las aristas da el aspecto de holograma.
            for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                framebuffer.blend_point(x + dx, y + dy, 0.12);
            }
            framebuffer.blend_point(x, y, 0.9);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interpolates_fragment_positions_and_rejects_hidden_fragments() {
        use std::cell::Cell;
        let mut framebuffer = Framebuffer::new(8, 8);
        let screen = [
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(8.0, 0.0, 1.0),
            Vec3::new(0.0, 8.0, 1.0),
        ];
        let local = [
            Vec3::zeros(),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ];
        shaded_triangle(&mut framebuffer, &screen, &local, |p| {
            crate::color::Color::new((p.x * 255.0) as u8, (p.y * 255.0) as u8, 0)
        });
        assert_eq!(framebuffer.buffer[0], 0x0F0F00);
        assert_eq!(framebuffer.buffer[63], 0);
        let calls = Cell::new(0);
        let behind = screen.map(|p| p + Vec3::new(0.0, 0.0, 1.0));
        shaded_triangle(&mut framebuffer, &behind, &local, |_| {
            calls.set(calls.get() + 1);
            crate::color::Color::new(255, 255, 255)
        });
        assert_eq!(calls.get(), 0);
        assert_eq!(framebuffer.buffer[0], 0x0F0F00);
    }

    #[test]
    fn wireframe_draws_edges_and_leaves_the_triangle_interior_empty() {
        let mut framebuffer = Framebuffer::new(10, 10);
        framebuffer.set_current_color(0x23C3FF);
        wireframe(
            &mut framebuffer,
            &Vec3::new(1.0, 1.0, 0.0),
            &Vec3::new(8.0, 1.0, 0.0),
            &Vec3::new(1.0, 8.0, 0.0),
        );

        assert_ne!(framebuffer.buffer[14], 0);
        assert_eq!(framebuffer.buffer[33], 0);
        assert_eq!(framebuffer.buffer[99], 0);

        // Las aristas que cruzan el borde no deben salir del framebuffer.
        wireframe(
            &mut framebuffer,
            &Vec3::new(-4.0, -4.0, 0.0),
            &Vec3::new(4.0, 4.0, 0.0),
            &Vec3::new(4.0, 4.0, 0.0),
        );
        assert_ne!(framebuffer.buffer[0], 0);
    }

    #[test]
    fn fills_triangles_and_keeps_the_closest_surface() {
        let mut framebuffer = Framebuffer::new(8, 8);
        let a = Vec3::new(-2.0, -2.0, 1.0);
        let b = Vec3::new(7.0, -2.0, 1.0);
        let c = Vec3::new(-2.0, 7.0, 1.0);

        let local = [Vec3::zeros(); 3];
        shaded_triangle(&mut framebuffer, &[a, b, c], &local, |_| {
            Color::from_hex(0xFFFF00)
        });
        assert_eq!(framebuffer.buffer[0], 0xFFFF00);
        assert_eq!(framebuffer.buffer[63], 0);

        let offset = Vec3::new(0.0, 0.0, 1.0);
        shaded_triangle(
            &mut framebuffer,
            &[c + offset, b + offset, a + offset],
            &local,
            |_| Color::from_hex(0xFF0000),
        );
        assert_eq!(framebuffer.buffer[0], 0xFFFF00);

        framebuffer.clear();
        shaded_triangle(&mut framebuffer, &[c, b, a], &local, |_| {
            Color::from_hex(0xFF0000)
        });
        assert_eq!(framebuffer.buffer[0], 0xFF0000);

        framebuffer.clear();
        shaded_triangle(&mut framebuffer, &[a, a, a], &local, |_| {
            Color::from_hex(0xFF0000)
        });
        assert!(framebuffer.buffer.iter().all(|pixel| *pixel == 0));
    }
}
