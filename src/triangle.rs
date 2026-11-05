use nalgebra_glm::Vec3;

use crate::framebuffer::Framebuffer;

pub fn triangle(framebuffer: &mut Framebuffer, a: &Vec3, b: &Vec3, c: &Vec3) {
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
            let w_a = edge(b, c, &point) / area;
            let w_b = edge(c, a, &point) / area;
            let w_c = edge(a, b, &point) / area;

            if w_a >= 0.0 && w_b >= 0.0 && w_c >= 0.0 {
                let depth = w_a * a.z + w_b * b.z + w_c * c.z;
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

        framebuffer.set_current_color(0xFFFF00);
        triangle(&mut framebuffer, &a, &b, &c);
        assert_eq!(framebuffer.buffer[0], 0xFFFF00);
        assert_eq!(framebuffer.buffer[63], 0);

        framebuffer.set_current_color(0xFF0000);
        let offset = Vec3::new(0.0, 0.0, 1.0);
        triangle(
            &mut framebuffer,
            &(c + offset),
            &(b + offset),
            &(a + offset),
        );
        assert_eq!(framebuffer.buffer[0], 0xFFFF00);

        framebuffer.clear();
        triangle(&mut framebuffer, &c, &b, &a);
        assert_eq!(framebuffer.buffer[0], 0xFF0000);

        framebuffer.clear();
        triangle(&mut framebuffer, &a, &a, &a);
        assert!(framebuffer.buffer.iter().all(|pixel| *pixel == 0));
    }
}
