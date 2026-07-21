use crate::error::{Error, Result};
use crate::types::LegalMetadata;
use image::DynamicImage;

/// ISCC-**like** (International Standard Content Code) identifier.
///
/// Computed from an image's perceptual and data characteristics using
/// a custom DCT-based perceptual hash. **Not guaranteed to be interoperable**
/// with the standard [ISCC specification](https://iscc-project.github.io/) —
/// use these identifiers for in-application deduplication and provenance
/// tracking, not for cross-ISCC-tool interoperability.
#[deprecated(
    since = "0.4.0",
    note = "Renamed to `ContentIdentifiers` for clarity. This is not a standard ISCC identifier."
)]
pub type Iscc = ContentIdentifiers;

/// Content identifiers computed from an image's perceptual and data characteristics.
///
/// Uses a custom DCT-based perceptual hash. **Not guaranteed to be interoperable**
/// with the standard [ISCC specification](https://iscc-project.github.io/) —
/// use these identifiers for in-application deduplication and provenance
/// tracking, not for cross-ISCC-tool interoperability.
#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub struct ContentIdentifiers {
    meta: Option<String>,
    content: String,
    data: String,
    instance: String,
    full: String,
}

impl ContentIdentifiers {
    /// Computes content identifiers from an image.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Iscc`] if the underlying computation fails.
    pub fn from_image(img: &DynamicImage) -> Result<Self> {
        Self::from_image_with_metadata(img, None)
    }

    /// Computes content identifiers from an image with optional legal metadata.
    ///
    /// When `legal_metadata` is `Some`, a meta code is generated for
    /// full ISO 24138:2024 compliance.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Iscc`] if the underlying computation fails.
    pub fn from_image_with_metadata(
        img: &DynamicImage,
        legal_metadata: Option<&LegalMetadata>,
    ) -> Result<Self> {
        let normalized = normalize_image(img);
        let pixels = extract_grayscale_pixels(&normalized);

        let content_result = iscc_lib::gen_image_code_v0(&pixels, 256)
            .map_err(|e| Error::Iscc(format!("image code generation failed: {}", e)))?;
        let content_code = content_result
            .iscc
            .strip_prefix("ISCC:")
            .unwrap_or(&content_result.iscc);

        let raw_bytes = img.to_rgba8().into_raw();
        let instance_result = iscc_lib::gen_instance_code_v0(&raw_bytes, 256)
            .map_err(|e| Error::Iscc(format!("instance code generation failed: {}", e)))?;
        let instance_code = instance_result
            .iscc
            .strip_prefix("ISCC:")
            .unwrap_or(&instance_result.iscc);

        let meta_code = legal_metadata.and_then(|legal| {
            let name = legal.copyright_holder().unwrap_or("Unknown");
            let description = legal.usage_terms();
            let meta_json = build_meta_json(legal);
            let meta_payload = if meta_json.len() > 2 {
                Some(meta_json.as_str())
            } else {
                None
            };
            let result = iscc_lib::gen_meta_code_v0(name, description, meta_payload, 256).ok()?;
            Some(
                result
                    .iscc
                    .strip_prefix("ISCC:")
                    .unwrap_or(&result.iscc)
                    .to_string(),
            )
        });

        let full = match &meta_code {
            Some(meta) => format!("ISCC:{}+{}+{}", meta, content_code, instance_code),
            None => format!("ISCC:{}+{}", content_code, instance_code),
        };

        Ok(Self {
            meta: meta_code,
            content: content_code.to_string(),
            data: instance_code.to_string(),
            instance: instance_code.to_string(),
            full,
        })
    }

    /// Returns the meta code component, if legal metadata was provided.
    #[must_use]
    pub fn meta_code(&self) -> Option<&str> {
        self.meta.as_deref()
    }

    /// Returns the content code component (perceptual hash).
    #[must_use]
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Returns the data code component (instance code).
    #[must_use]
    pub fn data(&self) -> &str {
        &self.data
    }

    /// Returns the instance code component (exact byte hash).
    #[must_use]
    pub fn instance(&self) -> &str {
        &self.instance
    }

    /// Returns the full ISCC URI (e.g., `ISCC:AA...+EE...+II...`).
    #[must_use]
    pub fn full(&self) -> &str {
        &self.full
    }

    /// Returns the content code as raw bytes.
    #[must_use]
    pub fn content_bytes(&self) -> &[u8] {
        self.content.as_bytes()
    }
}

fn build_meta_json(legal: &LegalMetadata) -> String {
    let mut map = serde_json::Map::new();
    if let Some(h) = legal.copyright_holder() {
        map.insert(
            "copyrightHolder".into(),
            serde_json::Value::String(h.into()),
        );
    }
    if let Some(e) = legal.contact_email() {
        map.insert("contactEmail".into(), serde_json::Value::String(e.into()));
    }
    if let Some(u) = legal.license_url() {
        map.insert("licenseUrl".into(), serde_json::Value::String(u.into()));
    }
    if let Some(t) = legal.usage_terms() {
        map.insert("usageTerms".into(), serde_json::Value::String(t.into()));
    }
    if let Some(d) = legal.creation_date() {
        map.insert("creationDate".into(), serde_json::Value::String(d.into()));
    }
    if let Some(a) = legal.ai_constraints() {
        map.insert("aiConstraints".into(), serde_json::Value::String(a.into()));
    }
    if let Some(w) = legal.web_statement_of_rights() {
        map.insert(
            "webStatementOfRights".into(),
            serde_json::Value::String(w.into()),
        );
    }
    serde_json::Value::Object(map).to_string()
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

/// Compute content identifiers from a `DynamicImage`.
#[deprecated(
    since = "0.4.0",
    note = "Renamed to `compute_content_identifiers()` for clarity."
)]
pub fn compute_iscc(img: &DynamicImage) -> Result<ContentIdentifiers> {
    compute_content_identifiers(img)
}

/// Compute content identifiers from a `DynamicImage` with legal metadata.
///
/// Generates a meta code from the provided legal metadata for full ISO 24138:2024 compliance.
#[deprecated(
    since = "0.4.0",
    note = "Renamed to `compute_content_identifiers_with_metadata()` for clarity."
)]
pub fn compute_iscc_with_metadata(
    img: &DynamicImage,
    legal_metadata: &LegalMetadata,
) -> Result<ContentIdentifiers> {
    compute_content_identifiers_with_metadata(img, legal_metadata)
}

/// Compute content identifiers from raw image bytes.
///
/// Returns `None` if the bytes cannot be decoded as an image.
#[deprecated(
    since = "0.4.0",
    note = "Renamed to `compute_content_identifiers_from_bytes()` for clarity."
)]
pub fn compute_iscc_from_bytes(bytes: &[u8]) -> Option<Result<ContentIdentifiers>> {
    compute_content_identifiers_from_bytes(bytes)
}

/// Compute content identifiers from raw image bytes with legal metadata.
///
/// Returns `None` if the bytes cannot be decoded as an image.
#[deprecated(
    since = "0.4.0",
    note = "Renamed to `compute_content_identifiers_from_bytes_with_metadata()` for clarity."
)]
pub fn compute_iscc_from_bytes_with_metadata(
    bytes: &[u8],
    legal_metadata: &LegalMetadata,
) -> Option<Result<ContentIdentifiers>> {
    compute_content_identifiers_from_bytes_with_metadata(bytes, legal_metadata)
}

/// Compute content identifiers from a `DynamicImage`.
pub fn compute_content_identifiers(img: &DynamicImage) -> Result<ContentIdentifiers> {
    ContentIdentifiers::from_image(img)
}

/// Compute content identifiers from a `DynamicImage` with legal metadata.
///
/// Generates a meta code from the provided legal metadata for full ISO 24138:2024 compliance.
pub fn compute_content_identifiers_with_metadata(
    img: &DynamicImage,
    legal_metadata: &LegalMetadata,
) -> Result<ContentIdentifiers> {
    ContentIdentifiers::from_image_with_metadata(img, Some(legal_metadata))
}

/// Compute content identifiers from raw image bytes.
///
/// Returns `None` if the bytes cannot be decoded as an image.
pub fn compute_content_identifiers_from_bytes(bytes: &[u8]) -> Option<Result<ContentIdentifiers>> {
    let img = image::load_from_memory(bytes).ok()?;
    Some(ContentIdentifiers::from_image(&img))
}

/// Compute content identifiers from raw image bytes with legal metadata.
///
/// Returns `None` if the bytes cannot be decoded as an image.
pub fn compute_content_identifiers_from_bytes_with_metadata(
    bytes: &[u8],
    legal_metadata: &LegalMetadata,
) -> Option<Result<ContentIdentifiers>> {
    let img = image::load_from_memory(bytes).ok()?;
    Some(ContentIdentifiers::from_image_with_metadata(
        &img,
        Some(legal_metadata),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_identifiers_deterministic() {
        let img = DynamicImage::new_rgb8(100, 100);
        let ci1 = ContentIdentifiers::from_image(&img).unwrap();
        let ci2 = ContentIdentifiers::from_image(&img).unwrap();

        assert_eq!(ci1.content(), ci2.content());
        assert_eq!(ci1.full(), ci2.full());
    }

    #[test]
    fn test_content_identifiers_starts_with_ee_prefix() {
        let img = DynamicImage::new_rgb8(100, 100);
        let ci = ContentIdentifiers::from_image(&img).unwrap();

        assert!(
            ci.content().starts_with("EE"),
            "content code should start with EE (CONTENT-IMAGE prefix per ISO 24138:2024), got: {}",
            ci.content()
        );
    }

    #[test]
    fn test_content_identifiers_with_metadata_includes_meta_code() {
        use crate::types::LegalMetadata;

        let img = DynamicImage::new_rgb8(100, 100);
        let legal = LegalMetadata::new()
            .with_copyright_holder("Test Author")
            .with_usage_terms("CC BY 4.0");
        let ci = ContentIdentifiers::from_image_with_metadata(&img, Some(&legal)).unwrap();

        assert!(ci.meta_code().is_some(), "meta code should be present");
        let meta = ci.meta_code().unwrap();
        assert!(
            meta.starts_with("AA"),
            "meta code should start with AA (META prefix), got: {}",
            meta
        );
        assert!(
            ci.full().starts_with("ISCC:AA"),
            "full URI should start with ISCC:AA when meta is present, got: {}",
            ci.full()
        );
        assert!(
            ci.full().contains('+'),
            "full URI should contain + separators"
        );
    }

    #[test]
    fn test_content_identifiers_without_metadata_no_meta_code() {
        let img = DynamicImage::new_rgb8(100, 100);
        let ci = ContentIdentifiers::from_image(&img).unwrap();

        assert!(ci.meta_code().is_none());
    }

    #[test]
    fn test_meta_code_getter() {
        let img = DynamicImage::new_rgb8(100, 100);
        let ci = ContentIdentifiers::from_image(&img).unwrap();
        assert!(ci.meta_code().is_none());

        let legal = LegalMetadata::new().with_copyright_holder("Author");
        let ci = ContentIdentifiers::from_image_with_metadata(&img, Some(&legal)).unwrap();
        assert!(ci.meta_code().is_some());
    }

    #[test]
    fn test_compute_content_identifiers_returns_result() {
        let img = DynamicImage::new_rgb8(100, 100);
        let ci = compute_content_identifiers(&img).unwrap();
        assert!(!ci.full().is_empty());
    }

    #[test]
    fn test_compute_content_identifiers_from_bytes_returns_option_of_result() {
        let img = DynamicImage::new_rgb8(100, 100);
        let bytes = crate::util::image::encode_image(&img, image::ImageFormat::Png).unwrap();
        let result = compute_content_identifiers_from_bytes(&bytes).unwrap();
        assert!(result.is_ok());
    }

    #[test]
    fn test_content_identifiers_field_getters() {
        let img = DynamicImage::new_rgb8(100, 100);
        let ci = ContentIdentifiers::from_image(&img).unwrap();
        assert!(!ci.data().is_empty());
        assert!(!ci.instance().is_empty());
        assert_eq!(ci.data(), ci.instance());
        assert!(!ci.content_bytes().is_empty());
    }
}
