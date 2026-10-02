use crate::error::ErrorDto;

pub struct stegoeggo_v1_notice_t {
    pub(crate) copyright_holder: Option<String>,
    pub(crate) contact_email: Option<String>,
    pub(crate) license_url: Option<String>,
    pub(crate) usage_terms: Option<String>,
    pub(crate) usage_terms_lang: Option<String>,
    pub(crate) creation_date: Option<String>,
    pub(crate) ai_constraints: Option<String>,
    pub(crate) web_statement_of_rights: Option<String>,
    pub(crate) creator: Option<String>,
    pub(crate) credit_line: Option<String>,
    pub(crate) copyright_owner: Option<String>,
    pub(crate) licensor_name: Option<String>,
    pub(crate) licensor_email: Option<String>,
    pub(crate) licensor_url: Option<String>,
    pub(crate) metadata_date: Option<String>,
    pub(crate) notice_applied_at: Option<String>,
    pub(crate) dmi: Option<stegoeggo::DmiValue>,
    pub(crate) seed: Option<u64>,
}

impl stegoeggo_v1_notice_t {
    pub(crate) fn new() -> Self {
        Self {
            copyright_holder: None,
            contact_email: None,
            license_url: None,
            usage_terms: None,
            usage_terms_lang: None,
            creation_date: None,
            ai_constraints: None,
            web_statement_of_rights: None,
            creator: None,
            credit_line: None,
            copyright_owner: None,
            licensor_name: None,
            licensor_email: None,
            licensor_url: None,
            metadata_date: None,
            notice_applied_at: None,
            dmi: None,
            seed: None,
        }
    }

    pub(crate) fn to_notice(&self) -> stegoeggo::RightsNotice {
        let mut notice = stegoeggo::RightsNotice::new();
        if let Some(value) = &self.copyright_holder {
            notice = notice.with_copyright_holder(value.clone());
        }
        if let Some(value) = &self.contact_email {
            notice = notice.with_contact_email(value.clone());
        }
        if let Some(value) = &self.license_url {
            notice = notice.with_license_url(value.clone());
        }
        if let Some(value) = &self.usage_terms {
            notice = notice.with_usage_terms(value.clone());
        }
        if let Some(value) = &self.creation_date {
            notice = notice.with_creation_date(value.clone());
        }
        if let Some(value) = &self.ai_constraints {
            notice = notice.with_ai_constraints(value.clone());
        }
        if let Some(value) = &self.web_statement_of_rights {
            notice = notice.with_web_statement_of_rights(value.clone());
        }
        if let Some(value) = &self.creator {
            notice = notice.with_creator(value.clone());
        }
        if let Some(value) = &self.credit_line {
            notice = notice.with_credit_line(value.clone());
        }
        if let Some(value) = &self.copyright_owner {
            notice = notice.with_copyright_owner(value.clone());
        }
        if let Some(value) = &self.licensor_name {
            notice = notice.with_licensor_name(value.clone());
        }
        if let Some(value) = &self.licensor_email {
            notice = notice.with_licensor_email(value.clone());
        }
        if let Some(value) = &self.licensor_url {
            notice = notice.with_licensor_url(value.clone());
        }
        if let Some(value) = &self.metadata_date {
            notice = notice.with_metadata_date(value.clone());
        }
        if let Some(value) = &self.notice_applied_at {
            notice = notice.with_notice_applied_at(value.clone());
        }
        if let Some(value) = self.dmi {
            notice = notice.with_dmi(value);
        }
        if let Some(value) = self.seed {
            notice = notice.with_seed(value);
        }
        if let Some(lang) = &self.usage_terms_lang {
            let terms = self.usage_terms.clone().unwrap_or_default();
            let localized = stegoeggo::LocalizedText::with_lang(terms, lang.clone());
            let legal = stegoeggo::LegalMetadata::new().with_usage_terms_localized(localized);
            notice = notice.with_legal_metadata_fields(&legal);
        }
        notice
    }
}

pub struct stegoeggo_v1_request_t {
    pub(crate) notice: stegoeggo::RightsNotice,
    pub(crate) policy: stegoeggo::RightsPolicy,
    pub(crate) hidden_marker: stegoeggo::HiddenMarkerMode,
    pub(crate) authentication: stegoeggo::AuthenticationMode,
    pub(crate) seed: Option<u64>,
    pub(crate) intensity: f32,
    pub(crate) output_format: Option<stegoeggo::ImageOutputFormat>,
    pub(crate) jpeg_quality: u8,
    pub(crate) progressive_jpeg: bool,
    pub(crate) max_dimension: Option<u32>,
    pub(crate) metadata_update_policy: stegoeggo::MetadataUpdatePolicy,
    pub(crate) stego_redundancy: Option<usize>,
    pub(crate) content_hash: Option<[u8; 4]>,
    pub(crate) timestamp_override: Option<String>,
    pub(crate) mac_key: Option<Vec<u8>>,
    pub(crate) resource_limits: Option<stegoeggo::ResourceLimits>,
}

impl stegoeggo_v1_request_t {
    pub(crate) fn new(
        notice: stegoeggo::RightsNotice,
        policy: stegoeggo::RightsPolicy,
        channels: stegoeggo::ProtectionChannels,
    ) -> Self {
        Self {
            notice,
            policy,
            hidden_marker: channels.hidden_marker,
            authentication: channels.authentication,
            seed: None,
            intensity: 0.5,
            output_format: None,
            jpeg_quality: 90,
            progressive_jpeg: false,
            max_dimension: None,
            metadata_update_policy: stegoeggo::MetadataUpdatePolicy::default(),
            stego_redundancy: None,
            content_hash: None,
            timestamp_override: None,
            mac_key: None,
            resource_limits: None,
        }
    }

    pub(crate) fn to_request(&self) -> stegoeggo::ProtectionRequest {
        let channels = stegoeggo::ProtectionChannels {
            rights_metadata: true,
            hidden_marker: self.hidden_marker,
            authentication: self.authentication,
        };
        let mut request =
            stegoeggo::ProtectionRequest::new(self.notice.clone(), self.policy, channels);
        if let Some(seed) = self.seed {
            request = request.with_seed(seed);
        }
        request = request.with_intensity(self.intensity);
        if let Some(format) = self.output_format {
            request = request.with_output_format(format);
        }
        request = request.with_jpeg_quality(self.jpeg_quality);
        if self.progressive_jpeg {
            request = request.with_progressive_jpeg();
        }
        if let Some(max) = self.max_dimension {
            request = request.with_max_dimension(max);
        }
        request = request.with_metadata_update_policy(self.metadata_update_policy);
        if let Some(redundancy) = self.stego_redundancy {
            request = request.with_stego_redundancy(redundancy);
        }
        if let Some(hash) = self.content_hash {
            request = request.with_content_hash(hash);
        }
        if let Some(ts) = &self.timestamp_override {
            request = request.with_timestamp_override(ts.clone());
        }
        if let Some(key) = &self.mac_key {
            request = request.with_mac_key(key.clone());
        }
        if let Some(limits) = &self.resource_limits {
            request = request.with_resource_limits(limits.clone());
        }
        request
    }
}

impl Drop for stegoeggo_v1_request_t {
    fn drop(&mut self) {
        use zeroize::Zeroize as _;
        if let Some(key) = self.mac_key.as_mut() {
            key.zeroize();
        }
    }
}

pub struct stegoeggo_v1_resource_limits_t {
    pub(crate) inner: stegoeggo::ResourceLimits,
}

pub struct stegoeggo_v1_buffer_t {
    pub(crate) bytes: Vec<u8>,
}

pub struct stegoeggo_v1_execution_report_t {
    pub(crate) inner: stegoeggo::ExecutionReport,
}

pub struct stegoeggo_v1_verification_report_t {
    pub(crate) inner: stegoeggo::verification::VerificationReport,
}

pub struct stegoeggo_v1_error_t {
    pub(crate) inner: ErrorDto,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notice_materializes_staged_fields() {
        let mut handle = stegoeggo_v1_notice_t::new();
        handle.copyright_holder = Some("Acme".to_string());
        handle.dmi = Some(stegoeggo::DmiValue::Allowed);
        handle.seed = Some(7);
        let notice = handle.to_notice();
        assert_eq!(notice.copyright_holder(), Some("Acme"));
        assert_eq!(notice.dmi(), Some(stegoeggo::DmiValue::Allowed));
        assert_eq!(notice.seed(), Some(7));
        assert_eq!(notice.contact_email(), None);
    }

    #[test]
    fn notice_lang_without_terms_materializes_lang() {
        let mut handle = stegoeggo_v1_notice_t::new();
        handle.usage_terms_lang = Some("fr".to_string());
        let notice = handle.to_notice();
        assert_eq!(notice.usage_terms_lang(), Some("fr"));
    }

    #[test]
    fn request_materializes_staged_options() {
        let notice = stegoeggo::RightsNotice::new();
        let mut handle = stegoeggo_v1_request_t::new(
            notice,
            stegoeggo::RightsPolicy::Allowed,
            stegoeggo::ProtectionChannels::metadata_only(),
        );
        handle.seed = Some(0);
        handle.jpeg_quality = 80;
        handle.mac_key = Some(vec![1, 2, 3]);
        let request = handle.to_request();
        assert_eq!(request.seed(), Some(0));
        assert_eq!(request.processing().jpeg_quality, 80);
        assert_eq!(request.mac_key(), Some(&[1, 2, 3][..]));
        assert_eq!(request.intensity(), 0.5);
    }
}
