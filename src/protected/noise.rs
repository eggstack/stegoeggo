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
        if ctx.intensity <= 0.0 {
            return Ok(Cow::Borrowed(img));
        }

        let rgba = img.to_rgba8();
        let result = if let Some(mac_key) = ctx.mac_key() {
            apply_perturbation_single_pass_keyed(
                &rgba,
                ctx.seed,
                ctx.intensity,
                self.intensity_multiplier,
                mac_key,
            )
        } else {
            apply_perturbation_single_pass(
                &rgba,
                ctx.seed,
                ctx.intensity,
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
