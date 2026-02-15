use image::DynamicImage;

/// Bounding box with pixel coordinates (inclusive on all sides).
#[derive(Debug, Clone, Copy)]
pub struct BBox {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

/// Calculate the tightest bounding box that contains all non-transparent pixels
/// across every frame. Returns `None` if every frame is fully transparent.
pub fn calculate_tight_bbox(frames: &[DynamicImage], width: u32, height: u32) -> Option<BBox> {
    let mut min_x = width as i32;
    let mut min_y = height as i32;
    let mut max_x: i32 = -1;
    let mut max_y: i32 = -1;

    for frame in frames {
        let rgba = frame.to_rgba8();
        let pixels = rgba.as_raw();

        for y in 0..height {
            for x in 0..width {
                let idx = ((y * width + x) * 4) as usize;
                let alpha = pixels[idx + 3];
                if alpha > 0 {
                    let xi = x as i32;
                    let yi = y as i32;
                    if xi < min_x {
                        min_x = xi;
                    }
                    if yi < min_y {
                        min_y = yi;
                    }
                    if xi > max_x {
                        max_x = xi;
                    }
                    if yi > max_y {
                        max_y = yi;
                    }
                }
            }
        }
    }

    if max_x < 0 {
        return None;
    }

    Some(BBox {
        left: min_x,
        top: min_y,
        right: max_x,
        bottom: max_y,
    })
}
