use crate::enums::{DmiValue, ImageOutputFormat, MetadataUpdatePolicy};
use crate::error::Result;
use crate::numeric::to_u64;
use napi::bindgen_prelude::BigInt;
use napi_derive::napi;
use stegoeggo::RightsNotice as RustRightsNotice;

/// The rights/legal notice written into output metadata.
///
/// Builders are immutable: each `with*` method returns a new notice, so a
/// partially configured notice can be shared safely.
#[derive(Clone, Debug)]
#[napi]
pub struct RightsNotice {
    inner: RustRightsNotice,
}

#[napi]
impl RightsNotice {
    /// Creates an empty rights notice.
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            inner: RustRightsNotice::new(),
        }
    }

    /// Sets the copyright holder name.
    #[must_use]
    #[napi]
    pub fn with_copyright_holder(&self, holder: String) -> Self {
        Self {
            inner: self.inner.clone().with_copyright_holder(holder),
        }
    }

    /// Sets the contact email for rights claims.
    #[must_use]
    #[napi]
    pub fn with_contact_email(&self, email: String) -> Self {
        Self {
            inner: self.inner.clone().with_contact_email(email),
        }
    }

    /// Sets the license URL.
    #[must_use]
    #[napi]
    pub fn with_license_url(&self, url: String) -> Self {
        Self {
            inner: self.inner.clone().with_license_url(url),
        }
    }

    /// Sets the usage terms text.
    #[must_use]
    #[napi]
    pub fn with_usage_terms(&self, terms: String) -> Self {
        Self {
            inner: self.inner.clone().with_usage_terms(terms),
        }
    }

    /// Sets the creation date string.
    #[must_use]
    #[napi]
    pub fn with_creation_date(&self, date: String) -> Self {
        Self {
            inner: self.inner.clone().with_creation_date(date),
        }
    }

    /// Sets the AI training constraints text.
    #[must_use]
    #[napi]
    pub fn with_ai_constraints(&self, constraints: String) -> Self {
        Self {
            inner: self.inner.clone().with_ai_constraints(constraints),
        }
    }

    /// Sets the web statement of rights URL.
    #[must_use]
    #[napi]
    pub fn with_web_statement_of_rights(&self, statement: String) -> Self {
        Self {
            inner: self.inner.clone().with_web_statement_of_rights(statement),
        }
    }

    /// Sets the creator name.
    #[must_use]
    #[napi]
    pub fn with_creator(&self, creator: String) -> Self {
        Self {
            inner: self.inner.clone().with_creator(creator),
        }
    }

    /// Sets the credit line.
    #[must_use]
    #[napi]
    pub fn with_credit_line(&self, line: String) -> Self {
        Self {
            inner: self.inner.clone().with_credit_line(line),
        }
    }

    /// Sets the copyright owner name.
    #[must_use]
    #[napi]
    pub fn with_copyright_owner(&self, owner: String) -> Self {
        Self {
            inner: self.inner.clone().with_copyright_owner(owner),
        }
    }

    /// Sets the licensor name.
    #[must_use]
    #[napi]
    pub fn with_licensor_name(&self, name: String) -> Self {
        Self {
            inner: self.inner.clone().with_licensor_name(name),
        }
    }

    /// Sets the licensor email.
    #[must_use]
    #[napi]
    pub fn with_licensor_email(&self, email: String) -> Self {
        Self {
            inner: self.inner.clone().with_licensor_email(email),
        }
    }

    /// Sets the licensor URL.
    #[must_use]
    #[napi]
    pub fn with_licensor_url(&self, url: String) -> Self {
        Self {
            inner: self.inner.clone().with_licensor_url(url),
        }
    }

    /// Sets the metadata date string.
    #[must_use]
    #[napi]
    pub fn with_metadata_date(&self, date: String) -> Self {
        Self {
            inner: self.inner.clone().with_metadata_date(date),
        }
    }

    /// Sets the notice-applied-at timestamp.
    #[must_use]
    #[napi]
    pub fn with_notice_applied_at(&self, timestamp: String) -> Self {
        Self {
            inner: self.inner.clone().with_notice_applied_at(timestamp),
        }
    }

    /// Sets the DMI value written alongside the notice.
    #[must_use]
    #[napi]
    pub fn with_dmi(&self, dmi: DmiValue) -> Self {
        Self {
            inner: self.inner.clone().with_dmi(dmi.into()),
        }
    }

    /// Sets the notice seed as a full-width `bigint`.
    #[napi]
    pub fn with_seed(&self, seed: BigInt) -> Result<Self> {
        Ok(Self {
            inner: self.inner.clone().with_seed(to_u64("seed", &seed)?),
        })
    }

    /// The copyright holder, or `null`.
    #[napi(getter, ts_return_type = "string | null")]
    pub fn copyright_holder(&self) -> Option<String> {
        self.inner.copyright_holder().map(ToString::to_string)
    }

    /// The contact email, or `null`.
    #[napi(getter, ts_return_type = "string | null")]
    pub fn contact_email(&self) -> Option<String> {
        self.inner.contact_email().map(ToString::to_string)
    }

    /// The license URL, or `null`.
    #[napi(getter, ts_return_type = "string | null")]
    pub fn license_url(&self) -> Option<String> {
        self.inner.license_url().map(ToString::to_string)
    }

    /// The usage terms, or `null`.
    #[napi(getter, ts_return_type = "string | null")]
    pub fn usage_terms(&self) -> Option<String> {
        self.inner.usage_terms().map(ToString::to_string)
    }

    /// The creation date, or `null`.
    #[napi(getter, ts_return_type = "string | null")]
    pub fn creation_date(&self) -> Option<String> {
        self.inner.creation_date().map(ToString::to_string)
    }

    /// The AI training constraints, or `null`.
    #[napi(getter, ts_return_type = "string | null")]
    pub fn ai_constraints(&self) -> Option<String> {
        self.inner.ai_constraints().map(ToString::to_string)
    }

    /// The web statement of rights URL, or `null`.
    #[napi(getter, ts_return_type = "string | null")]
    pub fn web_statement_of_rights(&self) -> Option<String> {
        self.inner
            .web_statement_of_rights()
            .map(ToString::to_string)
    }

    /// The creator, or `null`.
    #[napi(getter, ts_return_type = "string | null")]
    pub fn creator(&self) -> Option<String> {
        self.inner.creator().map(ToString::to_string)
    }

    /// The credit line, or `null`.
    #[napi(getter, ts_return_type = "string | null")]
    pub fn credit_line(&self) -> Option<String> {
        self.inner.credit_line().map(ToString::to_string)
    }

    /// The copyright owner, or `null`.
    #[napi(getter, ts_return_type = "string | null")]
    pub fn copyright_owner(&self) -> Option<String> {
        self.inner.copyright_owner().map(ToString::to_string)
    }

    /// The licensor name, or `null`.
    #[napi(getter, ts_return_type = "string | null")]
    pub fn licensor_name(&self) -> Option<String> {
        self.inner.licensor_name().map(ToString::to_string)
    }

    /// The licensor email, or `null`.
    #[napi(getter, ts_return_type = "string | null")]
    pub fn licensor_email(&self) -> Option<String> {
        self.inner.licensor_email().map(ToString::to_string)
    }

    /// The licensor URL, or `null`.
    #[napi(getter, ts_return_type = "string | null")]
    pub fn licensor_url(&self) -> Option<String> {
        self.inner.licensor_url().map(ToString::to_string)
    }

    /// The metadata date, or `null`.
    #[napi(getter, ts_return_type = "string | null")]
    pub fn metadata_date(&self) -> Option<String> {
        self.inner.metadata_date().map(ToString::to_string)
    }

    /// The notice-applied-at timestamp, or `null`.
    #[napi(getter, ts_return_type = "string | null")]
    pub fn notice_applied_at(&self) -> Option<String> {
        self.inner.notice_applied_at().map(ToString::to_string)
    }

    /// The DMI value, or `null`.
    #[napi(getter, ts_return_type = "DmiValue | null")]
    pub fn dmi(&self) -> Option<DmiValue> {
        self.inner.dmi().map(DmiValue::from)
    }

    /// The notice seed as a full-width `bigint`, or `null`.
    #[napi(getter, ts_return_type = "bigint | null")]
    pub fn seed(&self) -> Option<u64> {
        self.inner.seed()
    }

    /// Whether the notice carries any legal content.
    #[napi(getter)]
    pub fn has_legal_content(&self) -> bool {
        self.inner.has_legal_content()
    }
}

impl Default for RightsNotice {
    fn default() -> Self {
        Self::new()
    }
}

impl From<&RightsNotice> for RustRightsNotice {
    fn from(value: &RightsNotice) -> Self {
        value.inner.clone()
    }
}

impl From<RustRightsNotice> for RightsNotice {
    fn from(value: RustRightsNotice) -> Self {
        Self { inner: value }
    }
}

/// Encoding and output options applied to a protection request.
#[derive(Clone, Debug)]
#[napi]
pub struct ProcessingOptions {
    inner: stegoeggo::ProcessingOptions,
}

#[napi]
impl ProcessingOptions {
    /// Creates processing options with canonical defaults.
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            inner: stegoeggo::ProcessingOptions::default(),
        }
    }

    /// Sets the encoded output format.
    #[must_use]
    #[napi]
    pub fn with_output_format(&self, format: ImageOutputFormat) -> Self {
        let mut next = self.inner.clone();
        next.output_format = Some(format.into());
        Self { inner: next }
    }

    /// Sets the JPEG quality.
    #[napi]
    pub fn with_jpeg_quality(&self, quality: f64) -> Result<Self> {
        let mut next = self.inner.clone();
        next.jpeg_quality = crate::numeric::to_u8("jpegQuality", quality)?;
        Ok(Self { inner: next })
    }

    /// Enables progressive JPEG output.
    #[must_use]
    #[napi]
    pub fn with_progressive_jpeg(&self) -> Self {
        let mut next = self.inner.clone();
        next.progressive_jpeg = true;
        Self { inner: next }
    }

    /// Sets the maximum output dimension.
    #[napi]
    pub fn with_max_dimension(&self, max_dimension: f64) -> Result<Self> {
        let mut next = self.inner.clone();
        next.max_dimension = Some(crate::numeric::to_u32("maxDimension", max_dimension)?);
        Ok(Self { inner: next })
    }

    /// Sets the metadata update policy.
    #[must_use]
    #[napi]
    pub fn with_metadata_update_policy(&self, policy: MetadataUpdatePolicy) -> Self {
        let mut next = self.inner.clone();
        next.metadata_update_policy = policy.into();
        Self { inner: next }
    }

    /// Sets the stego redundancy override.
    #[napi]
    pub fn with_stego_redundancy(&self, redundancy: f64) -> Result<Self> {
        let mut next = self.inner.clone();
        next.stego_redundancy = Some(crate::numeric::to_usize("stegoRedundancy", redundancy)?);
        Ok(Self { inner: next })
    }

    /// Sets the 4-byte provenance content hash.
    #[napi]
    pub fn with_content_hash(&self, hash: &[u8]) -> Result<Self> {
        let bytes: [u8; 4] = hash.try_into().map_err(|_| {
            crate::error::config_error("contentHash must be exactly 4 bytes".to_string())
        })?;
        let mut next = self.inner.clone();
        next.content_hash = Some(bytes);
        Ok(Self { inner: next })
    }

    /// Sets the deterministic notice-applied-at timestamp override.
    #[must_use]
    #[napi]
    pub fn with_timestamp_override(&self, timestamp: String) -> Self {
        let mut next = self.inner.clone();
        next.timestamp_override = Some(timestamp);
        Self { inner: next }
    }

    /// The requested output format, or `null` when unset.
    #[napi(getter, ts_return_type = "ImageOutputFormat | null")]
    pub fn output_format(&self) -> Option<ImageOutputFormat> {
        self.inner.output_format.map(ImageOutputFormat::from)
    }

    /// The JPEG quality.
    #[napi(getter)]
    pub fn jpeg_quality(&self) -> u8 {
        self.inner.jpeg_quality
    }

    /// Whether progressive JPEG output is enabled.
    #[napi(getter)]
    pub fn progressive_jpeg(&self) -> bool {
        self.inner.progressive_jpeg
    }

    /// The maximum output dimension, or `null` when unset.
    #[napi(getter, ts_return_type = "number | null")]
    pub fn max_dimension(&self) -> Option<u32> {
        self.inner.max_dimension
    }

    /// The metadata update policy.
    #[napi(getter)]
    pub fn metadata_update_policy(&self) -> MetadataUpdatePolicy {
        self.inner.metadata_update_policy.into()
    }

    /// The stego redundancy override, or `null` when unset.
    #[napi(getter, ts_return_type = "number | null")]
    pub fn stego_redundancy(&self) -> Option<f64> {
        self.inner.stego_redundancy.map(crate::numeric::from_usize)
    }

    /// The provenance content hash bytes, or `null` when unset.
    #[napi(getter, ts_return_type = "Uint8Array | null")]
    pub fn content_hash(&self) -> Option<Vec<u8>> {
        self.inner.content_hash.map(|hash| hash.to_vec())
    }

    /// The timestamp override, or `null` when unset.
    #[napi(getter, ts_return_type = "string | null")]
    pub fn timestamp_override(&self) -> Option<String> {
        self.inner.timestamp_override.clone()
    }
}

impl Default for ProcessingOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl From<&ProcessingOptions> for stegoeggo::ProcessingOptions {
    fn from(value: &ProcessingOptions) -> Self {
        value.inner.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notice_builders_are_immutable() {
        let base = RightsNotice::new();
        let with_holder = base.with_copyright_holder("Acme".to_string());
        assert_eq!(base.copyright_holder(), None);
        assert_eq!(with_holder.copyright_holder(), Some("Acme".to_string()));
    }

    #[test]
    fn notice_seed_accepts_full_width_bigint() {
        let notice = RightsNotice::new()
            .with_seed(BigInt::from(u64::MAX))
            .unwrap();
        assert_eq!(notice.seed(), Some(u64::MAX));
    }

    #[test]
    fn notice_seed_rejects_negative_bigint() {
        let negative = BigInt {
            sign_bit: true,
            words: vec![1],
        };
        assert!(RightsNotice::new().with_seed(negative).is_err());
    }

    #[test]
    fn processing_options_reject_invalid_numbers() {
        assert!(ProcessingOptions::new().with_jpeg_quality(300.0).is_err());
        assert!(ProcessingOptions::new().with_max_dimension(-1.0).is_err());
        assert!(ProcessingOptions::new()
            .with_stego_redundancy(f64::NAN)
            .is_err());
    }

    #[test]
    fn content_hash_requires_four_bytes() {
        assert!(ProcessingOptions::new()
            .with_content_hash(&[1, 2, 3])
            .is_err());
        assert!(ProcessingOptions::new()
            .with_content_hash(&[1, 2, 3, 4])
            .is_ok());
    }
}
