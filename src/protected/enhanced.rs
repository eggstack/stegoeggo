use crate::error::Result;
use crate::protected::noise::NoiseProtector;
use crate::traits::Protector;
use crate::types::{ProtectionContext, ProtectionLevel};
use image::DynamicImage;
use std::borrow::Cow;

pub struct EnhancedProtector {
    inner: NoiseProtector,
}

impl EnhancedProtector {
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
