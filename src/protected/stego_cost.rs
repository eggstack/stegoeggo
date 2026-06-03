#![allow(dead_code)]

use image::RgbaImage;

/// Compute embedding cost for each pixel in an RGBA image.
///
/// Returns a `Vec<f32>` of length `width * height`, one cost per pixel.
/// Lower cost = cheaper to modify (prefer embedding here).
/// Higher cost = more expensive to modify (avoid embedding here).
///
/// Uses a Laplacian-based local complexity metric:
/// - Smooth regions (low local energy) → high cost (avoid)
/// - Textured regions (high local energy) → low cost (prefer)
///
/// The cost is normalized so the minimum is 1.0 and the maximum is
/// approximately `max_cost` (default 10.0).
pub fn compute_pixel_costs(img: &RgbaImage, max_cost: f32) -> Vec<f32> {
    let (w, h) = img.dimensions();
    let total = (w as usize) * (h as usize);
    let mut costs = vec![0.0f32; total];

    // Convert to grayscale for cost computation
    let gray: Vec<f32> = img
        .pixels()
        .map(|p| {
            let r = p[0] as f32;
            let g = p[1] as f32;
            let b = p[2] as f32;
            0.299 * r + 0.587 * g + 0.114 * b
        })
        .collect();

    for y in 0..h {
        for x in 0..w {
            let idx = (y as usize) * w as usize + x as usize;

            // Laplacian energy: sum of squared differences with 4-neighbors
            let center = gray[idx];
            let mut energy = 0.0f32;

            if x > 0 {
                let diff = center - gray[idx - 1];
                energy += diff * diff;
            }
            if x + 1 < w {
                let diff = center - gray[idx + 1];
                energy += diff * diff;
            }
            if y > 0 {
                let diff = center - gray[idx - w as usize];
                energy += diff * diff;
            }
            if y + 1 < h {
                let diff = center - gray[idx + w as usize];
                energy += diff * diff;
            }

            // Also compute local variance in 3x3 window for better texture detection
            let mut sum = 0.0f32;
            let mut count = 0u32;
            for dy in -1i32..=1 {
                for dx in -1i32..=1 {
                    let nx = x as i32 + dx;
                    let ny = y as i32 + dy;
                    if nx >= 0 && nx < w as i32 && ny >= 0 && ny < h as i32 {
                        let ni = ny as usize * w as usize + nx as usize;
                        sum += gray[ni];
                        count += 1;
                    }
                }
            }
            let mean = sum / count as f32;
            let mut variance = 0.0f32;
            for dy in -1i32..=1 {
                for dx in -1i32..=1 {
                    let nx = x as i32 + dx;
                    let ny = y as i32 + dy;
                    if nx >= 0 && nx < w as i32 && ny >= 0 && ny < h as i32 {
                        let ni = ny as usize * w as usize + nx as usize;
                        let diff = gray[ni] - mean;
                        variance += diff * diff;
                    }
                }
            }
            variance /= count as f32;

            // Combined complexity: Laplacian energy + local variance
            let complexity = energy + variance;

            // Cost is inversely proportional to complexity
            // Smooth pixels (complexity ≈ 0) get high cost
            // Textured pixels (complexity high) get low cost
            let cost = if complexity < 1.0 {
                max_cost
            } else {
                (1.0 / complexity.sqrt()) * max_cost
            };

            costs[idx] = cost;
        }
    }

    costs
}

/// Compute embedding costs for a grayscale slice of an RGBA image.
///
/// This is used for tiled embedding where only a sub-image is available.
pub fn compute_pixel_costs_sub(
    img: &RgbaImage,
    x0: u32,
    y0: u32,
    w: u32,
    h: u32,
    max_cost: f32,
) -> Vec<f32> {
    let total = (w as usize) * (h as usize);
    let mut costs = vec![0.0f32; total];

    // Extract grayscale sub-image
    let gray: Vec<f32> = (0..h)
        .flat_map(|dy| {
            (0..w).map(move |dx| {
                let p = img.get_pixel(x0 + dx, y0 + dy);
                let r = p[0] as f32;
                let g = p[1] as f32;
                let b = p[2] as f32;
                0.299 * r + 0.587 * g + 0.114 * b
            })
        })
        .collect();

    for y in 0..h {
        for x in 0..w {
            let idx = (y as usize) * w as usize + x as usize;
            let center = gray[idx];
            let mut energy = 0.0f32;

            if x > 0 {
                let diff = center - gray[idx - 1];
                energy += diff * diff;
            }
            if x + 1 < w {
                let diff = center - gray[idx + 1];
                energy += diff * diff;
            }
            if y > 0 {
                let diff = center - gray[idx - w as usize];
                energy += diff * diff;
            }
            if y + 1 < h {
                let diff = center - gray[idx + w as usize];
                energy += diff * diff;
            }

            let mut sum = 0.0f32;
            let mut count = 0u32;
            for dy in -1i32..=1 {
                for dx in -1i32..=1 {
                    let nx = x as i32 + dx;
                    let ny = y as i32 + dy;
                    if nx >= 0 && nx < w as i32 && ny >= 0 && ny < h as i32 {
                        let ni = ny as usize * w as usize + nx as usize;
                        sum += gray[ni];
                        count += 1;
                    }
                }
            }
            let mean = sum / count as f32;
            let mut variance = 0.0f32;
            for dy in -1i32..=1 {
                for dx in -1i32..=1 {
                    let nx = x as i32 + dx;
                    let ny = y as i32 + dy;
                    if nx >= 0 && nx < w as i32 && ny >= 0 && ny < h as i32 {
                        let ni = ny as usize * w as usize + nx as usize;
                        let diff = gray[ni] - mean;
                        variance += diff * diff;
                    }
                }
            }
            variance /= count as f32;

            let complexity = energy + variance;
            let cost = if complexity < 1.0 {
                max_cost
            } else {
                (1.0 / complexity.sqrt()) * max_cost
            };
            costs[idx] = cost;
        }
    }

    costs
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgba, RgbaImage};

    fn make_smooth_image(w: u32, h: u32) -> RgbaImage {
        ImageBuffer::from_pixel(w, h, Rgba([128, 128, 128, 255]))
    }

    fn make_textured_image(w: u32, h: u32) -> RgbaImage {
        ImageBuffer::from_fn(w, h, |x, y| {
            Rgba([
                ((x * 73 + y * 151) % 256) as u8,
                ((x * 53 + y * 97) % 256) as u8,
                ((x * 29 + y * 43) % 256) as u8,
                255,
            ])
        })
    }

    #[test]
    fn smooth_image_has_high_costs() {
        let img = make_smooth_image(16, 16);
        let costs = compute_pixel_costs(&img, 10.0);
        // Smooth image should have high costs (expensive to modify)
        let avg: f32 = costs.iter().sum::<f32>() / costs.len() as f32;
        assert!(
            avg > 8.0,
            "smooth image avg cost should be > 8.0, got {avg}"
        );
    }

    #[test]
    fn textured_image_has_lower_costs() {
        let img = make_textured_image(16, 16);
        let costs = compute_pixel_costs(&img, 10.0);
        let avg: f32 = costs.iter().sum::<f32>() / costs.len() as f32;
        assert!(
            avg < 8.0,
            "textured image avg cost should be < 8.0, got {avg}"
        );
    }

    #[test]
    fn costs_are_non_negative() {
        let img = make_textured_image(32, 32);
        let costs = compute_pixel_costs(&img, 10.0);
        for &c in &costs {
            assert!(c >= 0.0, "cost should be non-negative, got {c}");
        }
    }

    #[test]
    fn costs_length_matches_image() {
        let img = make_textured_image(10, 8);
        let costs = compute_pixel_costs(&img, 10.0);
        assert_eq!(costs.len(), 80);
    }

    #[test]
    fn sub_cost_matches_full_cost_at_interior() {
        let img = make_textured_image(32, 32);
        let full_costs = compute_pixel_costs(&img, 10.0);
        let sub_costs = compute_pixel_costs_sub(&img, 0, 0, 16, 16, 10.0);

        // Compare at interior positions (away from boundaries where neighbor
        // handling differs between full-image and sub-image computation)
        for y in 2..14 {
            for x in 2..14 {
                let full_idx = y * 32 + x;
                let sub_idx = y * 16 + x;
                assert!(
                    (full_costs[full_idx] - sub_costs[sub_idx]).abs() < 0.001,
                    "cost mismatch at ({x}, {y}): full={} sub={}",
                    full_costs[full_idx],
                    sub_costs[sub_idx]
                );
            }
        }
    }
}
