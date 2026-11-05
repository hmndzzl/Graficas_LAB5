use crate::color::Color;

pub struct Framebuffer {
    pub width: usize,
    pub height: usize,
    pub buffer: Vec<u32>,
    depth_buffer: Vec<f32>,
    background_color: u32,
    current_color: u32,
}

impl Framebuffer {
    pub fn save_png(&self, path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
        let file = std::io::BufWriter::new(std::fs::File::create(path)?);
        let mut encoder = png::Encoder::new(file, self.width as u32, self.height as u32);
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header()?;
        let pixels: Vec<u8> = self
            .buffer
            .iter()
            .flat_map(|pixel| [(pixel >> 16) as u8, (pixel >> 8) as u8, *pixel as u8])
            .collect();
        writer.write_image_data(&pixels)?;
        Ok(())
    }

    pub fn new(width: usize, height: usize) -> Self {
        Framebuffer {
            width,
            height,
            buffer: vec![0; width * height],
            depth_buffer: vec![f32::INFINITY; width * height],
            background_color: 0x000000,
            current_color: 0xFFFFFF,
        }
    }

    pub fn clear(&mut self) {
        for pixel in self.buffer.iter_mut() {
            *pixel = self.background_color;
        }
        self.depth_buffer.fill(f32::INFINITY);
    }

    pub fn point(&mut self, x: usize, y: usize) {
        if x < self.width && y < self.height {
            self.buffer[y * self.width + x] = self.current_color;
        }
    }

    pub fn set_background_color(&mut self, color: u32) {
        self.background_color = color;
    }

    pub fn point_with_depth(&mut self, x: usize, y: usize, depth: f32) {
        if x < self.width && y < self.height {
            let index = y * self.width + x;
            if depth < self.depth_buffer[index] {
                self.depth_buffer[index] = depth;
                self.point(x, y);
            }
        }
    }

    pub fn is_visible(&self, x: usize, y: usize, depth: f32) -> bool {
        x < self.width && y < self.height && depth < self.depth_buffer[y * self.width + x]
    }

    pub fn set_current_color(&mut self, color: u32) {
        self.current_color = color;
    }

    pub fn blend_point(&mut self, x: i32, y: i32, alpha: f32) {
        if x >= 0 && y >= 0 && (x as usize) < self.width && (y as usize) < self.height {
            let index = y as usize * self.width + x as usize;
            let background = Color::from_hex(self.buffer[index]);
            let color = Color::from_hex(self.current_color);
            self.buffer[index] = (background * (1.0 - alpha) + color * alpha).to_hex();
        }
    }
}
