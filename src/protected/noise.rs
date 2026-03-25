use crate::error::Result;
use crate::protected::constants::NOISE_INTENSITY_MULTIPLIER;
use crate::traits::Protector;
use crate::types::{ProtectionContext, ProtectionLevel};
use crate::util::image::{apply_perturbation_single_pass, apply_perturbation_single_pass_keyed};
use image::DynamicImage;
use std::borrow::Cow;

pub struct NoiseProtector {
    intensity_multiplier: f32,
}

impl NoiseProtector {
    pub fn new() -> Self {
        Self {
            intensity_multiplier: NOISE_INTENSITY_MULTIPLIER,
        }
    }

    pub fn enhanced() -> Self {
        Self {
            intensity_multiplier: 12.0,
        }
    }
}

impl Default for NoiseProtector {
    fn default() -> Self {
        Self::new()
    }
}

impl Protector for NoiseProtector {
    fn apply<'a>(
        &self,
        img: &'a DynamicImage,
        ctx: &ProtectionContext,
    ) -> Result<Cow<'a, DynamicImage>> {
        if ctx.intensity() <= 0.0 {
            return Ok(Cow::Borrowed(img));
        }

        let rgba = img.to_rgba8();
        let result = if let Some(mac_key) = ctx.mac_key() {
            apply_perturbation_single_pass_keyed(
                &rgba,
                ctx.seed(),
                ctx.intensity(),
                self.intensity_multiplier,
                mac_key,
            )
        } else {
            apply_perturbation_single_pass(
                &rgba,
                ctx.seed(),
                ctx.intensity(),
                self.intensity_multiplier,
            )
        };

        Ok(Cow::Owned(result))
    }

    fn name(&self) -> &'static str {
        "noise"
    }

    fn protection_level(&self) -> ProtectionLevel {
        ProtectionLevel::Standard
    }

    fn estimated_latency_ms(&self) -> u32 {
        3
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
    fn intensity_zero_returns_borrowed() {
        let img = make_test_image(8, 8);
        let ctx = ProtectionContext::new(0.0, 42);
        let protector = NoiseProtector::new();
        match protector.apply(&img, &ctx).unwrap() {
            Cow::Borrowed(_) => {}
            Cow::Owned(_) => panic!("Expected Borrowed for intensity=0"),
        }
    }

    #[test]
    fn noise_modifies_pixels() {
        let img = make_test_image(16, 16);
        let ctx = ProtectionContext::new(0.5, 42);
        let protector = NoiseProtector::new();
        let result = protector.apply(&img, &ctx).unwrap();
        let result_rgba = result.to_rgba8();
        let original_rgba = img.to_rgba8();
        assert_ne!(result_rgba, original_rgba, "Noise should modify pixels");
    }

    #[test]
    fn noise_stays_in_range() {
        let img = make_test_image(16, 16);
        let ctx = ProtectionContext::new(1.0, 42);
        let protector = NoiseProtector::new();
        let result = protector.apply(&img, &ctx).unwrap();
        let _rgba = result.to_rgba8();
        // u8 is always in 0..=255 by definition; just verify it succeeds
    }
}
