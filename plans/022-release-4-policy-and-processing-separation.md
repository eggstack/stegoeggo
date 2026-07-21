# Plan 022: Release 4 — Policy and Processing Architecture Separation

## Status

Blocked on completion of Plan 021.

## Release intent

Release 4 separates the legal meaning a caller wants to express from the processing cost and evidence channels used to carry that meaning.

The current API still routes behavior primarily through `ProtectionLevel` and a collection of partially overlapping booleans and overrides on `ProtectionContext`. `ProtectionLevel::Light` and `ProtectionLevel::Standard` select concrete pipelines, influence default metadata behavior, and indirectly affect DMI selection. `EvidenceProfile` mainly changes warning interpretation rather than reliably selecting actual channels. This creates several problems:

- A legal policy can be inferred from processing intensity.
- Metadata-only use is not the simplest or cheapest first-class path.
- The same conceptual request can be expressed through several conflicting combinations.
- Pipeline behavior is distributed across `ProtectionLevel`, `EvidenceProfile`, `inject_metadata`, `inject_legal_claims`, `dmi_value`, MAC-key presence, and format-specific branches.
- The public API does not expose one validated execution plan before image processing starts.
- Compatibility presets and normative configuration are not clearly separated.

Release 4 must establish a policy-first architecture while preserving Release 1–3 canonical metadata output and legacy read compatibility.

## Prerequisites

Before implementation begins:

- Plan 021 must be closed.
- Main CI must have a green strict-conformance run with an inspected versioned report artifact.
- Current canonical, legacy, malformed, conflict, preservation, and external fixtures must be retained as regression gates.
- The current public API surface and semver compatibility constraints must be inventoried.

## Objectives

At completion:

1. Rights policy is represented explicitly and never inferred from processing intensity.
2. Evidence channels are configured explicitly and map deterministically to actual work.
3. Metadata-only processing is a first-class fast path and the default path for a legal notice.
4. `EvidenceProfile` either becomes a real execution preset or is replaced by a clearer preset type.
5. `ProtectionLevel` remains available as a deprecated compatibility preset for one release cycle.
6. One validation stage resolves all caller input into an immutable execution plan before processing.
7. Equivalent policy input produces equivalent canonical metadata across PNG, JPEG, and WebP.
8. Existing v0.3 callers can migrate mechanically with documented examples.

## Non-goals

Release 4 must not:

- Change the hidden payload wire format.
- Add public-key signatures or detached manifests.
- Add C2PA.
- Add new image formats.
- Remove legacy metadata or payload readers.
- Implement web-distribution TDMRep artifacts.
- Optimize steganographic robustness beyond work required to honor explicit channel selection.
- Remove `ProtectionLevel` outright.

## Proposed conceptual model

The exact names may change during implementation, but the public model should express these concepts directly.

```rust
pub enum RightsPolicy {
    Unspecified,
    Allowed,
    ProhibitedAiMlTraining,
    ProhibitedGenerativeAiTraining,
    ProhibitedExceptSearchIndexing,
    ProhibitedAllDataMining,
    ProhibitedSeeConstraints,
}

pub enum HiddenMarkerMode {
    Disabled,
    BestEffort,
    Tiled { tile_size: u32 },
}

pub enum AuthenticationMode {
    None,
    Hmac,
}

pub struct ProtectionChannels {
    pub rights_metadata: bool,
    pub hidden_marker: HiddenMarkerMode,
    pub authentication: AuthenticationMode,
}

pub struct ProcessingOptions {
    pub output_format: Option<ImageOutputFormat>,
    pub jpeg_quality: u8,
    pub progressive_jpeg: bool,
    pub max_dimension: Option<u32>,
    pub metadata_update_policy: MetadataUpdatePolicy,
}

pub struct ProtectionRequest {
    pub notice: RightsNotice,
    pub policy: RightsPolicy,
    pub channels: ProtectionChannels,
    pub processing: ProcessingOptions,
    pub provenance: ProvenanceOptions,
}

pub struct ResolvedProtectionPlan {
    // validated, immutable, crate-owned execution contract
}
```

`ProvenanceOptions` in Release 4 should contain only existing HMAC/seed/content-hash configuration. Release 5 may extend it.

## Workstream A: Define explicit rights policy

### A1. Introduce `RightsPolicy`

Add a public, non-exhaustive enum that expresses legal/data-mining intent without encoding XMP details.

Requirements:

- One-to-one mapping to canonical `DmiValue` where possible.
- `Unspecified` means no DMI claim is emitted.
- `Allowed` is explicit permission, not the absence of prohibition.
- Unknown future values remain distinguishable during verification.
- Conversion methods are explicit and documented.

### A2. Clarify `DmiValue` ownership

`DmiValue` should become the serialization/interoperability representation, while `RightsPolicy` is the caller-facing policy representation.

Avoid implicit `From<ProtectionLevel>` or equivalent conversions. Any compatibility conversion must live in the deprecated preset adapter and be visibly named.

### A3. Integrate policy with `RightsNotice`

Determine whether `RightsNotice` should own `RightsPolicy` directly or whether policy remains adjacent in `ProtectionRequest`. Choose one authoritative location and reject duplicate/conflicting configuration.

The preferred direction is:

- `RightsNotice` carries descriptive legal fields.
- `RightsPolicy` carries data-mining policy.
- `ProtectionRequest` combines them.

### A4. Update verification vocabulary

Verification must continue to report canonical/legacy signal origin. Add a resolved policy observation only as a derived view; do not erase source-level evidence.

### Acceptance criteria

- No non-deprecated code path derives rights policy from `ProtectionLevel`, intensity, output format, or channel selection.
- Every `RightsPolicy` variant has exact canonical mapping tests.
- `Unspecified` emits no canonical DMI property.
- Explicit `Allowed` emits canonical allowed metadata.
- Verification preserves canonical, legacy, unknown, malformed, and conflict diagnostics.

## Workstream B: Define explicit protection channels

### B1. Add `ProtectionChannels`

Represent actual execution channels directly:

- Standards-based rights metadata.
- Hidden marker mode.
- Authentication mode.

Do not represent channels as unrelated booleans that permit invalid combinations without validation.

### B2. Define channel semantics

Document exact behavior:

- `rights_metadata = true`: canonical metadata is emitted/merged.
- `rights_metadata = false`: no rights metadata is added; existing metadata handling remains governed by update policy.
- `HiddenMarkerMode::Disabled`: no LSB, DCT, Q-table, or tiled hidden marker work.
- `BestEffort`: existing non-tiled LSB/DCT behavior.
- `Tiled`: existing tiled behavior with validated size.
- `AuthenticationMode::Hmac`: requires a non-empty key and an enabled hidden marker.

### B3. Reject invalid combinations

Examples:

- HMAC authentication with hidden marker disabled.
- `ProhibitedSeeConstraints` without any constraints or web statement.
- Metadata disabled while caller assumes metadata-only legal notice preset.
- Tiled mode with unsupported format or invalid tile size.
- Conflicting old and new API settings.

### B4. Make channel execution observable

Return or expose a resolved plan/result showing which channels were requested and which executed. Warnings must refer to requested channel contracts, not inferred profiles.

### Acceptance criteria

- Every channel setting maps to concrete pipeline work.
- No hidden marker is applied when disabled.
- Metadata-only mode performs no pixel/DCT modification.
- Invalid channel combinations fail before image decoding or mutation.
- Execution results identify requested, executed, degraded, and skipped channels.

## Workstream C: Introduce a metadata-only fast path

### C1. Make metadata-only the legal-notice default

The simplest standards-based workflow should require only a rights notice and policy. It should not require a seed, intensity, protection level, or hidden-marker configuration.

Provide a direct API such as:

```rust
protect_bytes(input, &ProtectionRequest::metadata_only(notice, policy))
```

or an equivalent builder.

### C2. Preserve image payload bytes where possible

For same-format metadata-only processing:

- PNG: insert/merge metadata chunks without pixel decode/re-encode.
- JPEG: insert/merge APP/COM segments without entropy decode/re-encode.
- WebP: insert/merge RIFF metadata chunks without pixel decode/re-encode.

The fast path must preserve unrelated metadata and image payload bytes except for required container-size/checksum changes.

### C3. Define conversion behavior

When output format differs from input format, decoding/re-encoding is expected. Report that a transcode occurred.

### C4. Benchmark the fast path

Add benchmarks for:

- Metadata-only same-format PNG/JPEG/WebP.
- Current Light/Standard compatibility paths.
- Large images where metadata-only work should remain approximately proportional to file size rather than pixel count.

### Acceptance criteria

- Metadata-only same-format processing does not invoke steganography.
- JPEG same-format metadata-only processing does not alter entropy-coded scan data.
- PNG/WebP same-format metadata-only processing does not decode pixels.
- External conformance remains green.
- Benchmarks demonstrate a material reduction in CPU and allocation versus Standard processing.

## Workstream D: Resolve caller configuration into one execution plan

### D1. Add a validation/resolution stage

Create a pure function or builder finalization step:

```rust
fn resolve_request(request: &ProtectionRequest, input: InputFacts)
    -> Result<ResolvedProtectionPlan>;
```

The resolved plan should include:

- Effective rights policy and canonical DMI value.
- Effective rights notice.
- Selected channels.
- Authentication requirements.
- Format path.
- Metadata update policy.
- Resource limits.
- Expected warnings/degradation conditions.

### D2. Make the plan immutable

Once resolved, pipeline stages must consume the immutable plan rather than repeatedly querying mutable/optional context fields.

### D3. Centralize validation

Move validation out of scattered format branches. Format-specific code may validate container constraints, but semantic configuration validation belongs in one place.

### D4. Add plan introspection

Provide a debug/inspection representation suitable for CLI dry-run output and tests. Do not expose keys or secret material.

### Acceptance criteria

- Validation runs once before mutation.
- Format encoders consume the resolved policy/notice graph.
- No stage derives new legal defaults independently.
- Plan debug output redacts secrets.
- Unit tests cover every invalid combination.

## Workstream E: Rework `EvidenceProfile` into executable presets

### E1. Decide retain versus replace

Choose one:

1. Retain `EvidenceProfile` as a preset that expands into `ProtectionChannels` plus validation expectations.
2. Replace it with a new `ProtectionPreset` and deprecate `EvidenceProfile`.

Do not retain a type that only changes warning severity while implying execution behavior.

### E2. Define preset expansions

Suggested behavior:

- Legal notice: metadata only.
- Legal notice with stego: metadata + best-effort hidden marker.
- Authenticated provenance: metadata + hidden marker + HMAC.
- Maximal: metadata + tiled/best-effort marker + HMAC where configured.

Preset expansion must be deterministic and inspectable.

### E3. Allow explicit override carefully

If callers can override preset fields, define precedence and conflict rules. Prefer presets as constructors followed by explicit builder changes on the resulting request.

### Acceptance criteria

- A preset changes actual processing behavior.
- Warning severity follows the resolved requested contract.
- Missing HMAC key is an error for a preset that requires authentication.
- Preset expansion tests cover all formats.

## Workstream F: Compatibility adapter and deprecation strategy

### F1. Retain `ProtectionLevel` as a compatibility preset

For one deprecation cycle:

- `Disabled` maps to no-op.
- `Light` maps to the documented legacy-equivalent request.
- `Standard` maps to the documented legacy-equivalent request.

The mapping must be isolated in one adapter module and tested as frozen behavior.

### F2. Deprecate ambiguous builders

Candidates include:

- `with_dmi()` in favor of `with_rights_policy()`.
- `with_metadata_injection()` in favor of channels.
- `with_inject_legal_claims()` or equivalent legal-claim toggles.
- Evidence-profile methods that do not represent execution.

Use Rust deprecation notes with replacement examples.

### F3. Preserve old entrypoints

Existing functions taking `ProtectionLevel` should remain and delegate to the new request pipeline. New request-based functions should be the canonical API.

### F4. Add migration documentation

Provide before/after examples for:

- Metadata-only legal notice.
- Current Light behavior.
- Current Standard behavior.
- Authenticated HMAC provenance.
- Explicitly allowed DMI policy.
- Metadata disabled, hidden marker only.

### Acceptance criteria

- Existing v0.3 examples compile with deprecation warnings only.
- Old and new APIs produce equivalent output for mapped compatibility cases.
- No duplicate implementation path exists; old APIs delegate to new resolution.
- Deprecation removals are assigned to the next major-version plan, not Release 4.

## Workstream G: CLI and configuration migration

### G1. Add policy/channel flags

The CLI should expose explicit options such as:

- `--rights-policy`.
- `--metadata` / `--no-metadata`.
- `--hidden-marker`.
- `--authentication`.
- `--preset`.
- `--dry-run` or `--explain-plan`.

Exact names should follow current CLI conventions.

### G2. Preserve legacy flags temporarily

Legacy level/profile flags should map through the compatibility adapter and produce deprecation notices where appropriate.

### G3. Define config-file schema versioning

If the CLI supports serialized config, introduce a schema version or migration layer. Reject ambiguous mixed old/new fields.

### G4. Protect secrets

HMAC keys must not appear in dry-run output, logs, serialized plans, or error messages.

### Acceptance criteria

- CLI can express every supported policy/channel combination.
- Dry-run displays the resolved plan without processing the image.
- Mixed conflicting legacy/new options fail clearly.
- Existing CLI invocations remain functional for the deprecation cycle.

## Workstream H: Documentation and architecture cleanup

### H1. Restructure documentation around workflows

Lead with:

1. Metadata-only rights notice.
2. Metadata plus best-effort hidden marker.
3. Authenticated provenance.

Do not lead with Light/Standard levels.

### H2. Correct stale type documentation

Remove outdated statements that:

- Describe custom legacy DMI properties as canonical IPTC output.
- Claim image-level `tdm:reserve_tdm` emission.
- State that supplied legal metadata does not auto-activate where that is no longer true.

### H3. Document legal versus technical semantics

For every policy and channel, state:

- What is encoded.
- What external tools can observe.
- What transformations may preserve it.
- Whether it is authenticated.
- What it does not prove.

### Acceptance criteria

- README examples use the new request API first.
- Architecture docs include the resolved-plan flow.
- Deprecated API documentation points to exact replacements.
- No documentation implies policy is determined by processing intensity.

## Workstream I: Testing and conformance preservation

### I1. Unit tests

Cover:

- Policy-to-DMI mapping.
- Preset expansion.
- Channel validation.
- Compatibility adapter mapping.
- Request resolution.
- Secret redaction.

### I2. Cross-format tests

For each format, test:

- Metadata-only.
- Metadata + best-effort marker.
- Metadata + HMAC marker.
- Hidden-marker-only.
- Explicit allowed policy.
- Unspecified policy.
- Format conversion.

### I3. Byte-preservation tests

- JPEG entropy scan unchanged in metadata-only same-format path.
- PNG IDAT data unchanged.
- WebP image payload chunks unchanged.
- Unrelated metadata preserved.

### I4. Compatibility tests

Freeze old `ProtectionLevel` output semantics and compare against new preset-expanded requests.

### I5. Conformance tests

Run the full Plan 021 strict conformance suite. Add request/preset identifiers to generated fixture provenance where useful.

### Acceptance criteria

- All Release 1–3 conformance gates remain green.
- Metadata-only output is externally visible in all formats.
- Compatibility mappings are deterministic.
- No hidden-marker code executes in metadata-only tests.

## Milestone sequence

### Milestone 1: Domain types and resolution

- Add `RightsPolicy`.
- Add channel types.
- Add request and resolved-plan types.
- Add validation and unit tests.
- No pipeline behavior changes yet.

Gate: domain model approved, exhaustive validation tests green.

### Milestone 2: Metadata-only execution path

- Add request-based processing API.
- Implement same-format metadata-only byte paths.
- Preserve canonical conformance.

Gate: byte-preservation and external conformance tests green.

### Milestone 3: Explicit hidden/authenticated channels

- Route existing stego/HMAC behavior through channel selection.
- Make presets executable.
- Add execution result reporting.

Gate: all channel combinations and degradation semantics tested.

### Milestone 4: Compatibility and CLI migration

- Add `ProtectionLevel` adapter.
- Deprecate ambiguous builders.
- Add CLI options and dry-run.
- Update examples/docs.

Gate: old API compatibility tests and CLI migration tests green.

### Milestone 5: Release validation

- Full Plan 021 conformance.
- Benchmarks.
- Package/API review.
- Status ledger and green CI artifact inspection.

## Release gate

Release 4 is complete only when:

1. Legal policy is explicit and independent of processing level.
2. Channel selection deterministically controls actual execution.
3. Metadata-only is the default legal-notice path.
4. Same-format metadata-only operations avoid pixel/DCT reprocessing.
5. Presets expand into actual channel behavior.
6. Old `ProtectionLevel` APIs delegate through one compatibility adapter.
7. Centralized validation produces an immutable resolved plan.
8. All canonical, legacy, preservation, malformed, conflict, and external conformance tests remain green.
9. Migration documentation is complete.
10. `plans/022-status.md` records implementation commits, commands, CI run, artifacts, benchmarks, deprecations, and residual risks.

## Handoff requirements

The implementing agent must create `plans/022-status.md` and update it throughout implementation. Every milestone must identify:

- Files changed.
- Public API additions/deprecations.
- Compatibility decisions.
- Tests added.
- Benchmark results.
- CI run and conformance artifact evidence.
- Any scope deferred to Release 5 or Release 6.
