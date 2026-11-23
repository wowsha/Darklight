//! Display list and a small software canvas.
//!
//! Layout produces a [`DisplayList`]; a backend (the software [`Canvas`] today,
//! the wgpu renderer later) turns it into pixels. Text commands are recorded but
//! not rasterized yet: that needs the font pipeline planned for Phase 1.

#![forbid(unsafe_code)]

use darklight_css::Color;
use darklight_layout::Rect;

#[derive(Debug, Clone, PartialEq)]
pub enum DisplayCommand {
    SolidRect {
        color: Color,
        rect: Rect,
    },
    Text {
        text: String,
        x: f32,
        y: f32,
        size: f32,
        color: Color,
    },
}

pub type DisplayList = Vec<DisplayCommand>;

/// An RGBA8 software canvas (premultiplied-free, straight alpha blending).
#[derive(Debug, Clone)]
pub struct Canvas {
    pub width: usize,
    pub height: usize,
    /// Row-major RGBA bytes, `width * height * 4` long.
    pub pixels: Vec<u8>,
}

impl Canvas {
    pub fn new(width: usize, height: usize, background: Color) -> Self {
        let mut pixels = Vec::with_capacity(width * height * 4);
        for _ in 0..width * height {
            pixels.extend_from_slice(&[background.r, background.g, background.b, background.a]);
        }
        Self {
            width,
            height,
            pixels,
        }
    }

    pub fn pixel(&self, x: usize, y: usize) -> Option<[u8; 4]> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let i = (y * self.width + x) * 4;
        Some([
            self.pixels[i],
            self.pixels[i + 1],
            self.pixels[i + 2],
            self.pixels[i + 3],
        ])
    }

    pub fn paint(&mut self, list: &DisplayList) {
        for cmd in list {
            if let DisplayCommand::SolidRect { color, rect } = cmd {
                self.fill_rect(*color, *rect);
            }
        }
    }

    fn fill_rect(&mut self, color: Color, rect: Rect) {
        let clamp = |v: f32, max: usize| v.max(0.0).min(max as f32) as usize;
        let x0 = clamp(rect.x, self.width);
        let y0 = clamp(rect.y, self.height);
        let x1 = clamp(rect.x + rect.width, self.width);
        let y1 = clamp(rect.y + rect.height, self.height);
        let a = color.a as u32;
        for y in y0..y1 {
            for x in x0..x1 {
                let i = (y * self.width + x) * 4;
                let src = [color.r, color.g, color.b];
                for (c, s) in src.iter().enumerate() {
                    let dst = self.pixels[i + c] as u32;
                    self.pixels[i + c] = ((*s as u32 * a + dst * (255 - a)) / 255) as u8;
                }
                // The canvas is opaque once anything has been painted over it.
                self.pixels[i + 3] = 255;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paints_and_clips_rects() {
        let mut c = Canvas::new(4, 4, Color::BLACK);
        c.paint(&vec![DisplayCommand::SolidRect {
            color: Color::rgb(255, 0, 0),
            rect: Rect {
                x: -2.0,
                y: 2.0,
                width: 4.0,
                height: 10.0,
            },
        }]);
        assert_eq!(c.pixel(0, 2), Some([255, 0, 0, 255]));
        assert_eq!(c.pixel(1, 3), Some([255, 0, 0, 255]));
        assert_eq!(c.pixel(2, 2), Some([0, 0, 0, 255]));
        assert_eq!(c.pixel(0, 1), Some([0, 0, 0, 255]));
        assert_eq!(c.pixel(9, 9), None);
    }

    #[test]
    fn blends_translucent_color() {
        let mut c = Canvas::new(1, 1, Color::BLACK);
        c.paint(&vec![DisplayCommand::SolidRect {
            color: Color {
                r: 255,
                g: 255,
                b: 255,
                a: 128,
            },
            rect: Rect {
                x: 0.0,
                y: 0.0,
                width: 1.0,
                height: 1.0,
            },
        }]);
        assert_eq!(c.pixel(0, 0), Some([128, 128, 128, 255]));
    }
}
