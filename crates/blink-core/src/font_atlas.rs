use fontdue::{Font, FontSettings};
use std::collections::HashMap;

pub struct GlyphInfo {
    pub uv_x: f32,
    pub uv_y: f32,
    pub uv_w: f32,
    pub uv_h: f32,
    pub width: f32,
    pub height: f32,
    pub offset_x: f32,
    pub offset_y: f32,
}

pub struct FontAtlas {
    pub texture_data: Vec<u8>,
    pub texture_width: u32,
    pub texture_height: u32,
    pub glyphs: HashMap<char, GlyphInfo>,
    pub line_height: f32,
    pub cell_width: f32,
    pub ascent: f32,
}

impl FontAtlas {
    pub fn new(font_data: &[u8], font_size: f32) -> Self {
        let font = Font::from_bytes(font_data, FontSettings::default())
            .expect("Failed to parse font");

        let chars: Vec<char> = (32u8..127u8).map(|b| b as char).collect();

        let mut rasterized: Vec<(char, fontdue::Metrics, Vec<u8>)> = Vec::new();
        let mut max_height: u32 = 0;
        let mut total_width: u32 = 0;

        for &ch in &chars {
            let (metrics, bitmap) = font.rasterize(ch, font_size);
            let w = metrics.width as u32;
            let h = metrics.height as u32;
            total_width += w + 2;
            if h > max_height {
                max_height = h;
            }
            rasterized.push((ch, metrics, bitmap));
        }

        let atlas_width = total_width.next_power_of_two().max(256);
        let atlas_height = (max_height + 4).next_power_of_two().max(64);
        let mut texture_data = vec![0u8; (atlas_width * atlas_height) as usize];

        // Reserve a solid white pixel at (0, 0) for cursor/rectangles
        texture_data[0] = 255;

        let line_metrics = font.horizontal_line_metrics(font_size);
        let line_height = line_metrics
            .map(|m| m.new_line_size)
            .unwrap_or(font_size * 1.4);
        let ascent = line_metrics
            .map(|m| m.ascent)
            .unwrap_or(font_size * 0.8);

        let m_metrics = font.metrics('M', font_size);
        let cell_width = m_metrics.advance_width;

        let mut glyphs = HashMap::new();
        let mut cursor_x: u32 = 2; // start after the solid pixel

        for (ch, metrics, bitmap) in &rasterized {
            let w = metrics.width as u32;
            let h = metrics.height as u32;

            for row in 0..h {
                for col in 0..w {
                    let src = (row * w + col) as usize;
                    let dst = ((row + 1) * atlas_width + cursor_x + col) as usize;
                    if src < bitmap.len() && dst < texture_data.len() {
                        texture_data[dst] = bitmap[src];
                    }
                }
            }

            glyphs.insert(
                *ch,
                GlyphInfo {
                    uv_x: cursor_x as f32 / atlas_width as f32,
                    uv_y: 1.0 / atlas_height as f32,
                    uv_w: w as f32 / atlas_width as f32,
                    uv_h: h as f32 / atlas_height as f32,
                    width: w as f32,
                    height: h as f32,
                    offset_x: metrics.xmin as f32,
                    offset_y: metrics.ymin as f32,
                },
            );

            cursor_x += w + 2;
        }

        FontAtlas {
            texture_data,
            texture_width: atlas_width,
            texture_height: atlas_height,
            glyphs,
            line_height,
            cell_width,
            ascent,
        }
    }

    pub fn solid_uv(&self) -> [f32; 4] {
        [
            0.0,
            0.0,
            1.0 / self.texture_width as f32,
            1.0 / self.texture_height as f32,
        ]
    }
}
