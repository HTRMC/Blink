use log;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Icon {
    ChevronRight,
    ChevronDown,
}

#[derive(Debug, Clone)]
pub struct IconInfo {
    pub uv_x: f32,
    pub uv_y: f32,
    pub uv_w: f32,
    pub uv_h: f32,
    pub width: f32,
    pub height: f32,
}

pub struct IconAtlas {
    pub texture_data: Vec<u8>,
    pub texture_width: u32,
    pub texture_height: u32,
    pub icons: HashMap<Icon, IconInfo>,
}

struct RasterizedIcon {
    id: Icon,
    width: u32,
    height: u32,
    data: Vec<u8>, // alpha channel only
}

fn rasterize_svg(svg_str: &str, size: u32) -> Option<(u32, u32, Vec<u8>)> {
    let tree = usvg::Tree::from_str(svg_str, &usvg::Options::default()).ok()?;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(size, size)?;

    let svg_size = tree.size();
    let sx = size as f32 / svg_size.width();
    let sy = size as f32 / svg_size.height();
    let scale = sx.min(sy);

    let transform = resvg::tiny_skia::Transform::from_scale(scale, scale);
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    // Extract alpha channel only (single-channel, like our font atlas)
    let rgba = pixmap.data();
    let alpha: Vec<u8> = rgba.chunks(4).map(|px| px[3]).collect();

    Some((size, size, alpha))
}

impl IconAtlas {
    pub fn new(device_pixel_ratio: f32) -> Self {
        let icon_size = (10.0 * device_pixel_ratio).ceil() as u32;

        let svgs: Vec<(Icon, &str)> = vec![
            (Icon::ChevronRight, r#"<svg width="16" height="16" viewBox="0 0 16 16" xmlns="http://www.w3.org/2000/svg" fill="white"><path d="M6.14601 3.14579C5.95101 3.34079 5.95101 3.65779 6.14601 3.85279L10.292 7.99879L6.14601 12.1448C5.95101 12.3398 5.95101 12.6568 6.14601 12.8518C6.34101 13.0468 6.65801 13.0468 6.85301 12.8518L11.353 8.35179C11.548 8.15679 11.548 7.83979 11.353 7.64478L6.85301 3.14479C6.65801 2.94979 6.34101 2.95079 6.14601 3.14579Z"/></svg>"#),
            (Icon::ChevronDown, r#"<svg width="16" height="16" viewBox="0 0 16 16" xmlns="http://www.w3.org/2000/svg" fill="white"><path d="M3.14598 5.85423L7.64598 10.3542C7.84098 10.5492 8.15798 10.5492 8.35298 10.3542L12.853 5.85423C13.048 5.65923 13.048 5.34223 12.853 5.14723C12.658 4.95223 12.341 4.95223 12.146 5.14723L7.99998 9.29323L3.85398 5.14723C3.65898 4.95223 3.34198 4.95223 3.14698 5.14723C2.95198 5.34223 2.95098 5.65923 3.14598 5.85423Z"/></svg>"#),
        ];

        let mut rasterized: Vec<RasterizedIcon> = Vec::new();
        log::info!("IconAtlas: rasterizing {} icons at size {}px", svgs.len(), icon_size);
        for (id, svg_str) in &svgs {
            if let Some((w, h, data)) = rasterize_svg(svg_str, icon_size) {
                let non_zero = data.iter().filter(|&&b| b > 0).count();
                log::info!("IconAtlas: {:?} rasterized {}x{}, non-zero pixels: {}", id, w, h, non_zero);
                rasterized.push(RasterizedIcon {
                    id: *id,
                    width: w,
                    height: h,
                    data,
                });
            } else {
                log::error!("IconAtlas: failed to rasterize {:?}", id);
            }
        }

        // Pack into a horizontal strip atlas
        let total_width: u32 = rasterized.iter().map(|r| r.width + 2).sum();
        let max_height: u32 = rasterized.iter().map(|r| r.height).max().unwrap_or(1);
        let atlas_width = total_width.next_power_of_two().max(64);
        let atlas_height = (max_height + 2).next_power_of_two().max(64);

        let mut texture_data = vec![0u8; (atlas_width * atlas_height) as usize];
        let mut icons = HashMap::new();
        let mut cursor_x: u32 = 1;

        for icon in &rasterized {
            for row in 0..icon.height {
                for col in 0..icon.width {
                    let src = (row * icon.width + col) as usize;
                    let dst = ((row + 1) * atlas_width + cursor_x + col) as usize;
                    if src < icon.data.len() && dst < texture_data.len() {
                        texture_data[dst] = icon.data[src];
                    }
                }
            }

            icons.insert(
                icon.id,
                IconInfo {
                    uv_x: cursor_x as f32 / atlas_width as f32,
                    uv_y: 1.0 / atlas_height as f32,
                    uv_w: icon.width as f32 / atlas_width as f32,
                    uv_h: icon.height as f32 / atlas_height as f32,
                    width: icon.width as f32,
                    height: icon.height as f32,
                },
            );

            cursor_x += icon.width + 2;
        }

        IconAtlas {
            texture_data,
            texture_width: atlas_width,
            texture_height: atlas_height,
            icons,
        }
    }

    pub fn get(&self, icon: Icon) -> Option<&IconInfo> {
        self.icons.get(&icon)
    }
}
