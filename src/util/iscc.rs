use digest::Digest;
use image::DynamicImage;
use sha2::Sha256;

#[derive(Debug, Clone)]
pub struct Iscc {
    pub meta: Option<String>,
    pub content: String,
    pub data: String,
    pub instance: String,
    pub full: String,
}

impl Iscc {
    pub fn from_image(img: &DynamicImage) -> Self {
        let normalized = normalize_image(img);
        let pixels = extract_grayscale_pixels(&normalized);

        let content_code = compute_image_code(&pixels);
        let data_code = compute_data_code(img);

        let full = format!("ISCC:{}", content_code);

        Self {
            meta: None,
            content: content_code.clone(),
            data: data_code.clone(),
            instance: data_code,
            full,
        }
    }

    pub fn content_bytes(&self) -> &[u8] {
        let bytes = self.content.as_bytes();
        &bytes[..bytes.len().min(8)]
    }
}

fn normalize_image(img: &DynamicImage) -> DynamicImage {
    let gray = img.to_luma8();

    let resized = image::imageops::resize(&gray, 32, 32, image::imageops::FilterType::Lanczos3);

    DynamicImage::ImageLuma8(resized)
}

fn extract_grayscale_pixels(img: &DynamicImage) -> Vec<u8> {
    let gray = img.to_luma8();
    gray.into_raw()
}

fn compute_image_code(pixels: &[u8]) -> String {
    let dct_result = compute_dct_2d(pixels);

    let hash = dct_to_hash(&dct_result);

    encode_iscc_component(0x12, &hash)
}

fn compute_dct_2d(pixels: &[u8]) -> Vec<f64> {
    let input: Vec<f64> = pixels.iter().map(|&p| p as f64 - 128.0).collect();

    // Pre-compute cos table as flat array to avoid 1024 heap allocations
    let mut cos_table = [0.0f64; 1024];
    for i in 0..32 {
        for j in 0..32 {
            cos_table[i * 32 + j] =
                ((2.0 * j as f64 + 1.0) * i as f64 * std::f64::consts::PI / 64.0).cos();
        }
    }

    let alpha: Vec<f64> = (0..32)
        .map(|i: usize| {
            if i == 0 {
                1.0 / 32.0_f64.sqrt()
            } else {
                (2.0 / 32.0_f64).sqrt()
            }
        })
        .collect();

    let mut temp = vec![0.0; 1024];
    for y in 0..32 {
        for u in 0..32 {
            let mut sum = 0.0;
            for x in 0..32 {
                sum += input[y * 32 + x] * cos_table[u * 32 + x];
            }
            temp[y * 32 + u] = alpha[u] * sum;
        }
    }

    let mut result = vec![0.0; 1024];
    for v in 0..32 {
        for u in 0..32 {
            let mut sum = 0.0;
            for y in 0..32 {
                sum += temp[y * 32 + u] * cos_table[v * 32 + y];
            }
            result[v * 32 + u] = alpha[v] * sum;
        }
    }

    result
}

fn dct_to_hash(dct: &[f64]) -> [u8; 32] {
    let mut hash = [0u8; 32];

    let quadrants = [(0, 0), (8, 0), (0, 8), (8, 8)];

    let mut bit_idx = 0;
    for (qx, qy) in quadrants {
        let mut values = Vec::with_capacity(64);
        for y in qy..(qy + 8) {
            for x in qx..(qx + 8) {
                if x + y > 0 {
                    values.push(dct[y * 32 + x]);
                }
            }
        }

        if values.is_empty() {
            continue;
        }

        let median = {
            let mut sorted = values.clone();
            sorted.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            sorted[sorted.len() / 2]
        };

        for &v in &values {
            if bit_idx < 256 {
                let byte_idx = bit_idx / 8;
                let bit_pos = 7 - (bit_idx % 8);
                if v > median {
                    hash[byte_idx] |= 1 << bit_pos;
                }
                bit_idx += 1;
            }
        }
    }

    hash
}

fn compute_data_code(img: &DynamicImage) -> String {
    let rgba = img.to_rgba8();
    let bytes = rgba.into_raw();

    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let result = hasher.finalize();

    encode_iscc_component(0x33, &result)
}

fn encode_iscc_component(header: u8, digest: &[u8]) -> String {
    use base58::ToBase58;

    let mut component = vec![header];
    component.extend_from_slice(&digest[..8]);

    component.to_base58()
}

pub fn compute_iscc(img: &DynamicImage) -> Iscc {
    Iscc::from_image(img)
}

pub fn compute_iscc_from_bytes(bytes: &[u8]) -> Option<Iscc> {
    let img = image::load_from_memory(bytes).ok()?;
    Some(Iscc::from_image(&img))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iscc_deterministic() {
        let img = DynamicImage::new_rgb8(100, 100);
        let iscc1 = Iscc::from_image(&img);
        let iscc2 = Iscc::from_image(&img);

        assert_eq!(iscc1.content, iscc2.content);
    }
}
