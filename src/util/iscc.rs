use image::DynamicImage;

/// ISCC (International Standard Content Code) identifier.
///
/// Computed from an image's perceptual and data characteristics.
/// See the [ISCC specification](https://iscc-project.github.io/) for details.
#[derive(Debug, Clone)]
pub struct Iscc {
    /// Optional metadata code (not set by default).
    pub meta: Option<String>,
    /// Content-derived identifier (perceptual hash of normalized image).
    pub content: String,
    /// Data-derived identifier (instance code from raw image bytes).
    pub data: String,
    /// Per-file instance identifier (same as `data`).
    pub instance: String,
    /// Full ISCC URI (e.g., `ISCC:...`).
    pub full: String,
}

impl Iscc {
    pub fn from_image(img: &DynamicImage) -> Self {
        let normalized = normalize_image(img);
        let pixels = extract_grayscale_pixels(&normalized);

        let content_result =
            iscc_lib::gen_image_code_v0(&pixels, 256).expect("image code generation failed");
        let content_code = content_result
            .iscc
            .strip_prefix("ISCC:")
            .unwrap_or(&content_result.iscc);

        let raw_bytes = img.to_rgba8().into_raw();
        let instance_result = iscc_lib::gen_instance_code_v0(&raw_bytes, 256)
            .expect("instance code generation failed");
        let instance_code = instance_result
            .iscc
            .strip_prefix("ISCC:")
            .unwrap_or(&instance_result.iscc);

        let full = format!("ISCC:{}+{}", content_code, instance_code);

        Self {
            meta: None,
            content: content_code.to_string(),
            data: instance_code.to_string(),
            instance: instance_code.to_string(),
            full,
        }
    }

    pub fn content_bytes(&self) -> &[u8] {
        self.content.as_bytes()
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

/// Compute an ISCC identifier from a `DynamicImage`.
pub fn compute_iscc(img: &DynamicImage) -> Iscc {
    Iscc::from_image(img)
}

/// Compute an ISCC identifier from raw image bytes.
///
/// Returns `None` if the bytes cannot be decoded as an image.
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
        assert_eq!(iscc1.full, iscc2.full);
    }

    #[test]
    fn test_iscc_starts_with_ee_prefix() {
        let img = DynamicImage::new_rgb8(100, 100);
        let iscc = Iscc::from_image(&img);

        assert!(
            iscc.content.starts_with("EE"),
            "content code should start with EE (CONTENT-IMAGE prefix per ISO 24138:2024), got: {}",
            iscc.content
        );
    }
}
