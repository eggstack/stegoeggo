use crate::types::{
    DmiValue, HiddenMarkerMode, ImageOutputFormat, MetadataUpdatePolicy, ProtectionContext,
    ProtectionLevel, ResolvedProtectionPlan, RightsNotice, DEFAULT_OUTPUT_FORMAT,
};

pub(super) struct StructuredComParams {
    pub(super) level_byte: u8,
    pub(super) seed: u64,
    pub(super) intensity: f32,
}

impl StructuredComParams {
    pub(super) fn from_legacy(level: Option<ProtectionLevel>, seed: u64, intensity: f32) -> Self {
        Self {
            level_byte: level.map(|l| l.to_byte()).unwrap_or(2),
            seed,
            intensity,
        }
    }

    pub(super) fn for_plan(seed: u64, intensity: f32) -> Self {
        Self {
            level_byte: 2,
            seed,
            intensity,
        }
    }
}

pub(super) struct JpegRender<'a> {
    pub(super) limits: &'a crate::ResourceLimits,
    pub(super) structured: Option<&'a StructuredComParams>,
}

pub(super) struct MetadataWriteSpec {
    pub(super) metadata: Vec<(Vec<u8>, Vec<u8>)>,
    pub(super) notice: RightsNotice,
    pub(super) effective_dmi: Option<DmiValue>,
    pub(super) format: ImageOutputFormat,
    pub(super) policy: MetadataUpdatePolicy,
    pub(super) limits: crate::ResourceLimits,
    pub(super) structured: StructuredComParams,
}

impl super::RightsMetadataProtector {
    pub(super) fn spec_from_plan(
        &self,
        plan: &ResolvedProtectionPlan,
    ) -> Option<MetadataWriteSpec> {
        let should_inject = plan.channels().rights_metadata;
        let mut notice = plan.effective_notice().clone();
        if matches!(plan.channels().hidden_marker, HiddenMarkerMode::Disabled) {
            notice.seed = None;
        }
        let effective_dmi = plan.effective_dmi();
        let metadata = self.generate_rights_metadata_from_notice(&notice, should_inject, None);
        if metadata.is_empty() && effective_dmi.is_none() {
            return None;
        }
        if !should_inject {
            return None;
        }
        Some(MetadataWriteSpec {
            metadata,
            notice,
            effective_dmi,
            format: plan.output_format(),
            policy: plan.processing().metadata_update_policy,
            limits: plan.resource_limits().clone(),
            structured: StructuredComParams::for_plan(plan.seed(), plan.intensity()),
        })
    }

    pub(super) fn spec_from_legacy(
        &self,
        ctx: &ProtectionContext,
        img_bytes: &[u8],
    ) -> Option<MetadataWriteSpec> {
        let should_inject =
            Self::should_inject_metadata(ctx.inject_metadata(), ctx.protection_level());
        let notice = ctx.normalize_rights_notice();
        let metadata = self.generate_rights_metadata_from_notice(
            &notice,
            should_inject,
            ctx.inject_legal_claims(),
        );
        if metadata.is_empty() {
            return None;
        }
        let format = ctx
            .output_format()
            .or(ctx.input_format())
            .unwrap_or_else(|| {
                ImageOutputFormat::from_magic_bytes(img_bytes).unwrap_or(DEFAULT_OUTPUT_FORMAT)
            });
        Some(MetadataWriteSpec {
            metadata,
            effective_dmi: notice.dmi(),
            notice,
            format,
            policy: ctx.metadata_update_policy(),
            limits: ctx.resource_limits(),
            structured: StructuredComParams::from_legacy(
                ctx.protection_level(),
                ctx.seed(),
                ctx.intensity(),
            ),
        })
    }
}
