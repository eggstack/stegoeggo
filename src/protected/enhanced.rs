use crate::error::Result;
use crate::protected::noise::NoiseProtector;
use crate::traits::Protector;
use crate::types::{ProtectionContext, ProtectionLevel};
use image::DynamicImage;
use std::borrow::Cow;

/// Enhanced adversarial noise protector for the Enhanced protection level.
///
/// Uses a higher intensity multiplier (12.0 vs the standard default) via
/// [`NoiseProtector::enhanced`], producing stronger perturbations that are
/// more disruptive to AI model training but may introduce visible artifacts.
pub struct EnhancedProtector {
    inner: NoiseProtector,
}

impl EnhancedProtector {
    /// Create a new enhanced protector with 12x intensity multiplier.
    pub fn new() -> Self {
        Self {
            inner: NoiseProtector::enhanced(),
        }
    }
}

impl Default for EnhancedProtector {
    fn default() -> Self {
        Self::new()
    }
}

impl Protector for EnhancedProtector {
    fn apply<'a>(
        &self,
        img: &'a DynamicImage,
        ctx: &ProtectionContext,
    ) -> Result<Cow<'a, DynamicImage>> {
        self.inner.apply(img, ctx)
    }

    fn name(&self) -> &'static str {
        "enhanced"
    }

    fn protection_level(&self) -> ProtectionLevel {
        ProtectionLevel::Enhanced
    }

    fn estimated_latency_ms(&self) -> u32 {
        5
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protected::noise::NoiseProtector;

    fn make_test_image(w: u32, h: u32) -> DynamicImage {
        DynamicImage::ImageRgba8(image::RgbaImage::from_fn(w, h, |x, y| {
            image::Rgba([x as u8, y as u8, 128, 255])
        }))
    }

    #[test]
    fn enhanced_differs_from_standard() {
        let img = make_test_image(32, 32);
        let ctx = ProtectionContext::new(0.5, 42);

        let standard = NoiseProtector::new().apply(&img, &ctx).unwrap();
        let enhanced = EnhancedProtector::new().apply(&img, &ctx).unwrap();

        assert_ne!(
            standard.to_rgba8(),
            enhanced.to_rgba8(),
            "Enhanced should produce different output than standard"
        );
    }
}
