use crate::error::{Error, Result};
use crate::protected::constants::PRECOMPUTED_CACHE_CAPACITY;
use crate::traits::{Protector, VariantLoader};
use crate::types::{ProtectedVariant, ProtectionContext, ProtectionLevel};
use crate::util::image::{
    apply_perturbation, apply_perturbation_par, parallel_threshold, XorShiftRng,
};
use image::DynamicImage;
use lru::LruCache;
use std::borrow::Cow;
use std::num::NonZeroUsize;
use std::sync::RwLock;

/// Precomputed perturbation protector for CDN/WAF edge deployment.
///
/// Stores pre-generated perturbation data keyed by `(hash, level, intensity)`,
/// avoiding the cost of noise generation at request time. On cache miss, generates
/// the perturbation on-the-fly and auto-registers it for future requests.
///
/// Can be backed by a [`VariantLoader`](crate::traits::VariantLoader) for persistent
/// storage (Redis, database, filesystem).
///
/// # Usage
///
/// ```ignore
/// let precomputed = PrecomputedProtector::new();
///
/// // Generate perturbation data for an image
/// let perturbation = precomputed.generate_perturbation_data(width, height, &ctx)?;
///
/// // Register for later lookup
/// let variant = ProtectedVariant::new(hash, ProtectionLevel::Strong, perturbation, 0.5, width, height);
/// precomputed.register_variant(variant)?;
///
/// // At request time, apply() auto-looks up the variant
/// let protected = precomputed.apply(&img, &ctx)?;
/// ```
pub struct PrecomputedProtector {
    variants: RwLock<LruCache<String, ProtectedVariant>>,
    loader: Option<Box<dyn VariantLoader>>,
}

impl PrecomputedProtector {
    pub fn new() -> Self {
        Self {
            variants: RwLock::new(LruCache::new(
                NonZeroUsize::new(PRECOMPUTED_CACHE_CAPACITY).unwrap(),
            )),
            loader: None,
        }
    }

    pub fn with_loader(loader: Box<dyn VariantLoader>) -> Self {
        Self {
            variants: RwLock::new(LruCache::new(
                NonZeroUsize::new(PRECOMPUTED_CACHE_CAPACITY).unwrap(),
            )),
            loader: Some(loader),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            variants: RwLock::new(LruCache::new(NonZeroUsize::new(capacity).unwrap())),
            loader: None,
        }
    }

    /// Register a single variant in the in-memory cache and persist it
    /// to the loader if configured.
    ///
    /// Two-phase design: the loader I/O (persist) runs without the write
    /// lock held, then the write lock is acquired only for the fast
    /// in-memory insert. Holding the lock during I/O would block readers.
    pub fn register_variant(&self, variant: ProtectedVariant) -> Result<()> {
        let key = variant.cache_key();

        if let Some(ref loader) = self.loader {
            loader.store_variant(&variant)?;
        }

        let mut variants = self
            .variants
            .write()
            .map_err(|e| Error::Config(format!("Lock error: {}", e)))?;
        variants.put(key, variant);
        Ok(())
    }

    /// Register multiple variants in a single batch.
    ///
    /// Persists all variants to the loader (if configured) before acquiring
    /// the write lock, then inserts all entries atomically.
    pub fn register_variants(&self, variants: Vec<ProtectedVariant>) -> Result<()> {
        if let Some(ref loader) = self.loader {
            for variant in &variants {
                loader.store_variant(variant)?;
            }
        }

        let entries: Vec<(String, ProtectedVariant)> =
            variants.into_iter().map(|v| (v.cache_key(), v)).collect();

        let mut write_guard = self
            .variants
            .write()
            .map_err(|e| Error::Config(format!("Lock error: {}", e)))?;
        for (key, variant) in entries {
            write_guard.put(key, variant);
        }
        Ok(())
    }

    fn get_cached_variant(
        &self,
        ctx: &ProtectionContext,
        original_hash: &str,
    ) -> Result<Option<ProtectedVariant>> {
        let intensity_rounded = (ctx.intensity() * 10000.0).round() / 10000.0;
        let key = format!(
            "{}_{}_{}",
            original_hash,
            ctx.protection_level()
                .unwrap_or(ProtectionLevel::Strong)
                .as_str(),
            intensity_rounded
        );

        {
            let variants = self
                .variants
                .read()
                .map_err(|e| Error::Config(format!("Lock error: {}", e)))?;
            if let Some(v) = variants.peek(&key) {
                return Ok(Some(v.clone()));
            }
        }

        if let Some(ref loader) = self.loader {
            if let Some(variant) = loader.load_variant(&key)? {
                let mut variants = self
                    .variants
                    .write()
                    .map_err(|e| Error::Config(format!("Lock error: {}", e)))?;
                variants.put(key, variant.clone());
                return Ok(Some(variant));
            }
        }

        Ok(None)
    }

    /// Generate perturbation data for a given image dimensions and context.
    ///
    /// Returns a `Vec<u8>` of length `width * height * 4` (RGBA), where each
    /// pixel's perturbation is stored as `[R, G, B, A]` with A=128.
    ///
    /// The data is deterministic for a given `(width, height, seed, intensity)`
    /// combination, allowing precomputation at upload time and lookup at serve time.
    pub fn generate_perturbation_data(
        &self,
        width: u32,
        height: u32,
        ctx: &ProtectionContext,
    ) -> Result<Vec<u8>> {
        let mut rng = XorShiftRng::new(ctx.seed());

        let mut perturbation = Vec::with_capacity((width * height * 4) as usize);

        let intensity = ctx.intensity();

        for _y in 0..height {
            for _x in 0..width {
                let noise_r =
                    (rng.gen_range(-1.0f32..1.0) * intensity * 64.0 + 128.0).clamp(0.0, 255.0);
                let noise_g =
                    (rng.gen_range(-1.0f32..1.0) * intensity * 64.0 + 128.0).clamp(0.0, 255.0);
                let noise_b =
                    (rng.gen_range(-1.0f32..1.0) * intensity * 64.0 + 128.0).clamp(0.0, 255.0);
                let noise_a = 128u8;

                perturbation.push(noise_r as u8);
                perturbation.push(noise_g as u8);
                perturbation.push(noise_b as u8);
                perturbation.push(noise_a);
            }
        }

        Ok(perturbation)
    }

    fn apply_cached_perturbation(
        &self,
        img: &DynamicImage,
        variant: &ProtectedVariant,
    ) -> Result<DynamicImage> {
        let img_rgba = img.to_rgba8();
        let (width, height) = img_rgba.dimensions();

        if variant.width() != width || variant.height() != height {
            return Err(Error::InvalidVariant(format!(
                "Dimension mismatch: expected {}x{}, got {}x{}",
                variant.width(),
                variant.height(),
                width,
                height
            )));
        }

        let perturbation = variant.perturbation_data();

        if perturbation.len() != (width * height * 4) as usize {
            return Err(Error::InvalidVariant(
                "Perturbation size mismatch".to_string(),
            ));
        }

        let total_pixels = (width * height) as usize;
        let output = if total_pixels >= parallel_threshold() {
            apply_perturbation_par(&img_rgba, perturbation, 4)?
        } else {
            apply_perturbation(&img_rgba, perturbation, 4)?
        };
        Ok(DynamicImage::ImageRgba8(output))
    }
}

impl Default for PrecomputedProtector {
    fn default() -> Self {
        Self::new()
    }
}

impl Protector for PrecomputedProtector {
    fn apply<'a>(
        &self,
        img: &'a DynamicImage,
        ctx: &ProtectionContext,
    ) -> Result<Cow<'a, DynamicImage>> {
        let original_hash = crate::util::image::compute_image_hash(img);

        if let Some(variant) = self.get_cached_variant(ctx, &original_hash)? {
            let result = self.apply_cached_perturbation(img, &variant)?;
            return Ok(Cow::Owned(result));
        }

        let img_rgba = img.to_rgba8();
        let (width, height) = img_rgba.dimensions();
        let perturbation = self.generate_perturbation_data(width, height, ctx)?;

        // Apply perturbation before building the variant to avoid cloning.
        let total_pixels = (width * height) as usize;
        let output = if total_pixels >= parallel_threshold() {
            apply_perturbation_par(&img_rgba, &perturbation, 4)?
        } else {
            apply_perturbation(&img_rgba, &perturbation, 4)?
        };

        let variant = crate::types::ProtectedVariant::new(
            original_hash,
            crate::types::ProtectionLevel::Strong,
            perturbation,
            ctx.intensity(),
            width,
            height,
        );
        let _ = self.register_variant(variant);
        // Registration failure is silently ignored by design: caching is
        // best-effort. The perturbation is still applied even if the
        // VariantLoader cannot persist it.

        Ok(Cow::Owned(DynamicImage::ImageRgba8(output)))
    }

    fn name(&self) -> &'static str {
        "precomputed"
    }

    fn protection_level(&self) -> ProtectionLevel {
        ProtectionLevel::Strong
    }

    fn estimated_latency_ms(&self) -> u32 {
        2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_image(w: u32, h: u32) -> DynamicImage {
        DynamicImage::ImageRgba8(image::RgbaImage::from_fn(w, h, |x, y| {
            image::Rgba([x as u8, y as u8, 128, 255])
        }))
    }

    #[test]
    fn generate_perturbation_data_correct_size() {
        let protector = PrecomputedProtector::new();
        let ctx = ProtectionContext::new(0.5, 42);
        let data = protector.generate_perturbation_data(8, 8, &ctx).unwrap();
        assert_eq!(data.len(), 8 * 8 * 4);
    }

    #[test]
    fn dimension_mismatch_returns_error() {
        let protector = PrecomputedProtector::new();
        let img = make_test_image(8, 8);
        let ctx = ProtectionContext::new(0.5, 42);
        let hash = crate::util::image::compute_image_hash(&img);
        let perturbation = protector.generate_perturbation_data(16, 16, &ctx).unwrap();
        let variant = crate::types::ProtectedVariant::new(
            hash,
            crate::types::ProtectionLevel::Strong,
            perturbation,
            0.5,
            16,
            16,
        );
        protector.register_variant(variant).unwrap();
        let result = protector.apply(&img, &ctx);
        assert!(result.is_err());
    }

    #[test]
    fn lru_eviction_removes_old_entries() {
        let protector = PrecomputedProtector::with_capacity(3);

        for w in 4..9u32 {
            let img = make_test_image(w, w);
            let hash = crate::util::image::compute_image_hash(&img);
            let ctx = ProtectionContext::new(0.5, w as u64);
            let perturbation = protector.generate_perturbation_data(w, w, &ctx).unwrap();
            let variant = crate::types::ProtectedVariant::new(
                hash,
                crate::types::ProtectionLevel::Strong,
                perturbation,
                0.5,
                w,
                w,
            );
            protector.register_variant(variant).unwrap();
        }

        let variants = protector.variants.read().unwrap();
        assert_eq!(variants.len(), 3);
    }
}
