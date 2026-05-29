use crate::error::Result;
use crate::traits::Protector;
use crate::types::{ProtectionContext, ProtectionLevel};
use image::DynamicImage;
use std::borrow::Cow;

/// No-op protector for the Disabled protection level.
///
/// Returns the input image unchanged. Used when no protection is desired
/// (e.g., whitelisted clients, testing, or as a pipeline placeholder).
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_is_passthrough() {
        let p = PassthroughProtector::new();
        assert_eq!(p.name(), "passthrough");
    }

    #[test]
    fn protection_level_is_disabled() {
        let p = PassthroughProtector::new();
        assert_eq!(p.protection_level(), ProtectionLevel::Disabled);
    }

    #[test]
    fn estimated_latency_is_zero() {
        let p = PassthroughProtector::new();
        assert_eq!(p.estimated_latency_ms(), 0);
    }

    #[test]
    fn does_not_modify_pixels() {
        let p = PassthroughProtector::new();
        assert!(!p.modifies_pixels());
    }

    #[test]
    fn apply_returns_borrowed() {
        let p = PassthroughProtector::new();
        let img = image::DynamicImage::new_rgb8(4, 4);
        let ctx = ProtectionContext::new(0.5, 42);
        let result = p.apply(&img, &ctx).unwrap();
        match result {
            std::borrow::Cow::Borrowed(_) => {}
            _ => panic!("PassthroughProtector::apply should return Cow::Borrowed"),
        }
    }
}
