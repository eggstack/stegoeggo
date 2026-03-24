use crate::error::Result;
use crate::traits::Protector;
use crate::types::{ProtectionContext, ProtectionLevel};
use image::DynamicImage;
use std::borrow::Cow;

pub struct PassthroughProtector;

impl PassthroughProtector {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PassthroughProtector {
    fn default() -> Self {
        Self::new()
    }
}

impl Protector for PassthroughProtector {
    fn apply<'a>(
        &self,
        img: &'a DynamicImage,
        _ctx: &ProtectionContext,
    ) -> Result<Cow<'a, DynamicImage>> {
        Ok(Cow::Borrowed(img))
    }

    fn name(&self) -> &'static str {
        "passthrough"
    }

    fn protection_level(&self) -> ProtectionLevel {
        ProtectionLevel::Disabled
    }

    fn estimated_latency_ms(&self) -> u32 {
        0
    }

    fn is_enabled(&self) -> bool {
        true
    }

    fn modifies_pixels(&self) -> bool {
        false
    }
}
