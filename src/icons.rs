use crate::{battery::Status, settings::DisplayMode};
use fontdue::{Font, FontSettings};
use std::{fs, path::Path};

pub const SIZE: usize = 32;
pub type Color = [u8; 3];

pub fn battery_color(percent: u8, dark: bool) -> Color {
    match (percent, dark) {
        (0..=20, true) => [255, 102, 115],
        (21..=50, true) => [255, 196, 76],
        (_, true) => [94, 224, 153],
        (0..=20, false) => [185, 28, 46],
        (21..=50, false) => [155, 98, 0],
        (_, false) => [16, 122, 67],
    }
}

pub struct Renderer {
    font: Font,
}

impl Renderer {
    pub fn new() -> Result<Self, String> {
        let windows = std::env::var_os("WINDIR").unwrap_or_else(|| "C:\\Windows".into());
        let fonts = Path::new(&windows).join("Fonts");
        for filename in ["segoeuib.ttf", "seguisb.ttf", "arialbd.ttf"] {
            if let Ok(bytes) = fs::read(fonts.join(filename))
                && let Ok(font) = Font::from_bytes(bytes, FontSettings::default())
            {
                return Ok(Self { font });
            }
        }
        Err("Could not load a Windows system font for the tray icon".into())
    }

    pub fn render(&self, status: Status, mode: DisplayMode, dark: bool) -> Vec<u8> {
        let mut canvas = Canvas::new();
        let neutral = if dark { [224, 229, 237] } else { [47, 57, 71] };
        match (status, mode) {
            (Status::Battery { percent, .. }, DisplayMode::Percentage) => {
                self.text(
                    &mut canvas,
                    &percent.to_string(),
                    battery_color(percent, dark),
                );
            }
            (Status::Battery { percent, .. }, DisplayMode::Battery) => {
                let color = battery_color(percent, dark);
                canvas.battery_outline(if percent == 0 { color } else { neutral });
                let height = (usize::from(percent) * 20).div_ceil(100);
                if height > 0 {
                    canvas.rect(10, 27 - height, 10, height, color);
                }
            }
            (Status::Checking, _) => self.text(&mut canvas, "…", neutral),
            (Status::Disconnected, _) => self.text(&mut canvas, "–", neutral),
            (Status::Unavailable, _) => self.text(&mut canvas, "?", neutral),
        }
        canvas.pixels
    }

    fn text(&self, canvas: &mut Canvas, text: &str, color: Color) {
        // Fit the ink, not the font's invisible margins. Horizontal compression
        // lets three digits retain the same readable height as two digits.
        let glyphs: Vec<_> = text.chars().map(|c| self.font.rasterize(c, 40.0)).collect();
        let top = glyphs
            .iter()
            .map(|(m, _)| m.ymin + m.height as i32)
            .max()
            .unwrap_or(0);
        let bottom = glyphs.iter().map(|(m, _)| m.ymin).min().unwrap_or(0);
        let height = (top - bottom) as usize;
        let width = glyphs
            .iter()
            .map(|(m, _)| m.advance_width)
            .sum::<f32>()
            .ceil() as usize
            + 2;
        if height == 0 {
            return;
        }
        let mut ink = vec![0u8; width * height];
        let mut pen = 0.0f32;
        for (metrics, bitmap) in glyphs {
            for y in 0..metrics.height {
                for x in 0..metrics.width {
                    let xx = pen.round() as i32 + metrics.xmin + x as i32;
                    let yy = top - metrics.ymin - metrics.height as i32 + y as i32;
                    if xx >= 0 && xx < width as i32 && yy >= 0 && yy < height as i32 {
                        let offset = yy as usize * width + xx as usize;
                        ink[offset] = ink[offset].max(bitmap[y * metrics.width + x]);
                    }
                }
            }
            pen += metrics.advance_width;
        }
        let left = (0..width)
            .find(|&x| (0..height).any(|y| ink[y * width + x] > 0))
            .unwrap_or(0);
        let right = (0..width)
            .rfind(|&x| (0..height).any(|y| ink[y * width + x] > 0))
            .unwrap_or(left);
        let ink_width = right - left + 1;
        let out_height = height.min(28);
        let out_width = ((ink_width * out_height).div_ceil(height)).min(30);
        let x0 = (SIZE - out_width) / 2;
        let y0 = (SIZE - out_height) / 2;
        // Area averaging keeps the compressed glyph edges antialiased.
        for y in 0..out_height {
            for x in 0..out_width {
                let sx0 = x as f32 * ink_width as f32 / out_width as f32;
                let sx1 = (x + 1) as f32 * ink_width as f32 / out_width as f32;
                let sy0 = y as f32 * height as f32 / out_height as f32;
                let sy1 = (y + 1) as f32 * height as f32 / out_height as f32;
                let mut alpha = 0.0;
                for sy in sy0.floor() as usize..(sy1.ceil() as usize).min(height) {
                    for sx in sx0.floor() as usize..(sx1.ceil() as usize).min(ink_width) {
                        let weight = (sx1.min((sx + 1) as f32) - sx0.max(sx as f32))
                            * (sy1.min((sy + 1) as f32) - sy0.max(sy as f32));
                        alpha += f32::from(ink[sy * width + left + sx]) * weight;
                    }
                }
                let alpha = (alpha / ((sx1 - sx0) * (sy1 - sy0))).round() as u8;
                canvas.pixel((x0 + x) as i32, (y0 + y) as i32, color, alpha);
            }
        }
    }
}

struct Canvas {
    pixels: Vec<u8>,
}

impl Canvas {
    fn new() -> Self {
        Self {
            pixels: vec![0; SIZE * SIZE * 4],
        }
    }

    fn pixel(&mut self, x: i32, y: i32, color: Color, alpha: u8) {
        if x < 0 || y < 0 || x >= SIZE as i32 || y >= SIZE as i32 || alpha == 0 {
            return;
        }
        let offset = (y as usize * SIZE + x as usize) * 4;
        self.pixels[offset..offset + 3].copy_from_slice(&color);
        self.pixels[offset + 3] = alpha;
    }

    fn rect(&mut self, x: usize, y: usize, width: usize, height: usize, color: Color) {
        for yy in y..y + height {
            for xx in x..x + width {
                self.pixel(xx as i32, yy as i32, color, 255);
            }
        }
    }

    fn battery_outline(&mut self, color: Color) {
        self.rect(12, 1, 8, 2, color);
        self.rect(8, 3, 14, 2, color);
        self.rect(6, 5, 2, 24, color);
        self.rect(22, 5, 2, 24, color);
        self.rect(8, 29, 14, 2, color);
    }
}

pub fn write_preview(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let renderer = Renderer::new()?;
    let levels = [100, 83, 50, 20, 5, 0];
    let width = 576;
    let height = 384;
    let mut pixels = vec![0u8; width * height * 4];
    for row in 0..4 {
        let dark = row < 2;
        let mode = if row % 2 == 0 {
            DisplayMode::Percentage
        } else {
            DisplayMode::Battery
        };
        let background = if dark { [26, 30, 37] } else { [242, 244, 247] };
        for y in row * 96..(row + 1) * 96 {
            for x in 0..width {
                let offset = (y * width + x) * 4;
                pixels[offset..offset + 3].copy_from_slice(&background);
                pixels[offset + 3] = 255;
            }
        }
        for (column, percent) in levels.iter().enumerate() {
            let icon = renderer.render(
                Status::Battery {
                    percent: *percent,
                    charging: false,
                },
                mode,
                dark,
            );
            for y in 0..64 {
                for x in 0..64 {
                    let source = ((y / 2) * SIZE + x / 2) * 4;
                    let target = ((row * 96 + 16 + y) * width + column * 96 + 16 + x) * 4;
                    let alpha = u16::from(icon[source + 3]);
                    for channel in 0..3 {
                        pixels[target + channel] = ((u16::from(icon[source + channel]) * alpha
                            + u16::from(background[channel]) * (255 - alpha))
                            / 255) as u8;
                    }
                }
            }
        }
    }
    let file = std::io::BufWriter::new(fs::File::create(path)?);
    let mut encoder = png::Encoder::new(file, width as u32, height as u32);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header()?.write_image_data(&pixels)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colors_change_at_documented_thresholds() {
        assert_ne!(battery_color(20, true), battery_color(21, true));
        assert_ne!(battery_color(50, true), battery_color(51, true));
        assert_eq!(battery_color(51, true), battery_color(100, true));
    }

    #[test]
    fn every_percentage_fits_both_modes_and_themes() {
        let renderer = Renderer::new().unwrap();
        for dark in [false, true] {
            for mode in [DisplayMode::Percentage, DisplayMode::Battery] {
                for percent in 0..=100 {
                    let pixels = renderer.render(
                        Status::Battery {
                            percent,
                            charging: false,
                        },
                        mode,
                        dark,
                    );
                    assert_eq!(pixels.len(), SIZE * SIZE * 4);
                    assert!(pixels.as_chunks::<4>().0.iter().any(|p| p[3] > 0));
                    assert!(pixels.as_chunks::<4>().0.iter().any(|p| p[3] == 0));
                }
            }
        }
    }
}
