# Plan 059: Generic Stego Carrier Core Extraction

Status: Ready for implementation

Roadmap: `plans/057-stego-carrier-library-and-pipeline-simplification-roadmap.md`

Depends on: Plan 058 complete with legacy/current LSB compatibility proven.

Audited planning baseline: `main` at `e60b801fda493ddd5e744f5de2a12d7b9489c078`

Authoritative implementation ledger to create before source edits: `plans/059-status.md`

---

## 1. Purpose

Separate application-neutral carrier mechanics from StegoEggo-specific rights/provenance payload construction and verification.

The current `SteganographyProtector` contains several layers in one large module:

- LSB carrier selection and bit embedding/extraction;
- tiled LSB traversal;
- JPEG DCT F5 orchestration;
- Q-table seed storage;
- StegoEggo payload-v1/v2/v3 generation and parsing;
- CRC/HMAC validation;
- seed discovery through rights metadata and fallback channels;
- compatibility probing and legal-notice verification behavior.

The low-level LSB and F5 functions already operate on arbitrary bytes. This plan makes that fact architectural: carrier code must accept caller payload bytes and carrier configuration without importing rights-policy/application payload types. `SteganographyProtector` remains as a StegoEggo application adapter and continues to own payload-v3 and compatibility behavior.

This plan is internal-first. It establishes a clean carrier boundary before Plan 062 stabilizes a public API.

---

## 2. Target module shape

Preferred structure:

```text
src/stego/
├── mod.rs
├── lsb.rs
└── jpeg.rs
```

Optional small shared files are allowed only when they reduce duplication, for example:

```text
src/stego/capacity.rs
src/stego/error.rs
```

Do not create a generalized carrier trait unless two concrete call sites demonstrably benefit. The pixel and JPEG domains have different inputs and should remain explicit.

The following application code remains outside the generic carrier modules:

```text
src/protected/steganography.rs
src/payload_v3/
src/protected/metadata_trap.rs
src/protected/notice_verification.rs
```

The existing `src/jpeg_transcoder/` stays crate-private implementation machinery behind the generic JPEG carrier facade.

---

## 3. Hard dependency boundary

Code under `src/stego/` must not import these application types:

```text
ProtectionContext
ProtectionRequest
ResolvedProtectionPlan
ProtectionLevel
EvidenceProfile
RightsPolicy
DmiValue
LegalMetadata
RightsNotice
StegoPayload
NoticeVerification
VerificationReport
```

The generic carrier may depend on narrow mechanical types such as:

- `image::RgbaImage` for pixel-domain operations;
- JPEG transcoder internals;
- resource-limit values where needed to bound parsing;
- a carrier-specific config/result/error type;
- byte slices, seeds, lengths, replication/tile options.

If `ResourceLimits` itself is too application-shaped, pass only the concrete limits needed by the carrier. Do not duplicate the full protection plan as a new carrier context.

Add a focused source/test guard if practical to prevent accidental imports of application modules into `src/stego/`; otherwise record the boundary in module docs and keep compiler-visible types sufficient to enforce it.

---

## 4. Phase 0 — inventory and status ledger

Before moving code:

1. Create `plans/059-status.md` and record the actual baseline SHA.
2. Inventory functions currently in `protected/steganography.rs` into three groups:
   - generic carrier mechanics;
   - StegoEggo payload/application logic;
   - compatibility/verification orchestration.
3. Record the intended destination for each moved function before editing.
4. Run focused LSB and DCT tests from Plan 058 and current JPEG tests.

The inventory must explicitly include:

```text
stego_permutation/current carrier mapping
embed/extract LSB bit logic
LSB capacity
tiled LSB carrier traversal
DCT capacity
DCT F5 embed/extract orchestration
Q-table seed hint mechanics
bit/byte conversion helpers
payload generation/parsing
CRC/HMAC validation
metadata seed discovery
legacy/current scheme probing
```

Do not move payload generation/parsing into `src/stego/` in this plan.

---

## 5. Phase 1 — extract the LSB carrier

Create an internal LSB carrier API that accepts arbitrary bytes and explicit mechanical configuration.

A suitable internal shape is approximately:

```rust
pub(crate) struct LsbConfig {
    pub seed: u64,
    pub replicas: usize,
    pub scheme: LsbCarrierScheme,
    pub tile_size: Option<u32>,
}

pub(crate) struct Capacity {
    pub required: usize,
    pub available: usize,
}

pub(crate) fn capacity(
    image: &RgbaImage,
    payload_len: usize,
    config: &LsbConfig,
) -> Result<Capacity>;

pub(crate) fn embed(
    image: &RgbaImage,
    payload: &[u8],
    config: &LsbConfig,
) -> Result<EmbedOutcome<RgbaImage>>;

pub(crate) fn extract(
    image: &RgbaImage,
    payload_len: usize,
    config: &LsbConfig,
) -> Result<Option<Vec<u8>>>;
```

Exact names may differ. Required semantics:

- no payload generation;
- no CRC/HMAC verification;
- no rights metadata lookup;
- no fallback seed guessing;
- raw extraction returns raw bytes or no carrier result;
- the caller supplies expected payload length;
- legacy/current scheme is explicit in the config;
- capacity is computed by the carrier itself;
- tiled operation, if retained in the same config, embeds the same arbitrary payload in tile regions without knowing its meaning.

`SteganographyProtector` becomes the caller that generates a StegoEggo payload and passes it to this API.

---

## 6. Phase 2 — extract the JPEG carrier facade

Create a crate-private encoded-JPEG carrier facade under `src/stego/jpeg.rs`.

The facade must accept raw JPEG bytes and arbitrary payload bytes while hiding `JpegHeader`, coefficient maps, Huffman tables, and entropy internals from future public callers.

Approximate internal shape:

```rust
pub(crate) struct JpegConfig {
    pub seed: u64,
    pub redundancy: usize,
    pub tile_size: Option<u32>,
    pub seed_hint: bool,
}

pub(crate) fn capacity(
    jpeg: &[u8],
    payload_len: usize,
    config: &JpegConfig,
) -> Result<Capacity>;

pub(crate) fn embed(
    jpeg: &[u8],
    payload: &[u8],
    config: &JpegConfig,
) -> Result<EmbedOutcome<Vec<u8>>>;

pub(crate) fn extract(
    jpeg: &[u8],
    payload_len: usize,
    config: &JpegConfig,
) -> Result<Option<Vec<u8>>>;
```

The facade may expose an internal support probe result so the application adapter can distinguish supported DCT embedding from progressive/unsupported fallback.

The Q-table seed mechanism should be mechanically separated from payload framing. Internally acceptable shape:

```rust
pub(crate) fn embed_seed_hint(jpeg: &[u8], seed: u64) -> Result<Vec<u8>>;
pub(crate) fn extract_seed_hint(jpeg: &[u8]) -> Result<Option<u64>>;
```

Do not make Q-table seed presence count as successful arbitrary-payload extraction.

All previously proven container-preservation behavior must remain intact.

---

## 7. Phase 3 — move only generic helpers

Move or consolidate helpers whose meaning is purely carrier-level:

- bit access/conversion;
- exact carrier capacity;
- carrier permutation;
- LSB matching mutation;
- DCT usable-coefficient counting;
- tile-region mapping;
- mechanical Q-table hint handling.

Do not move helpers whose meaning depends on the StegoEggo wire protocol:

- v3 prefix/header classification;
- payload version dispatch;
- DMI/channel field parsing;
- payload integrity/authentication classification;
- legacy ECC payload parsing;
- application verification status.

If a helper is used by both raw carrier extraction and StegoEggo framed extraction, prefer making the carrier return bytes and leaving protocol interpretation in the adapter rather than teaching the carrier about v3.

---

## 8. Phase 4 — make `SteganographyProtector` an adapter

After the carrier modules exist, rewrite application embedding conceptually as:

```text
Protection/application config
        |
        v
generate StegoEggo payload bytes
        |
        v
select carrier + mechanical config
        |
        v
stego::lsb::embed(...) or stego::jpeg::embed(...)
        |
        v
application metadata injection / reporting
```

Extraction similarly becomes:

```text
application seed discovery / compatibility choice
        |
        v
raw carrier extraction
        |
        v
StegoEggo payload integrity + parse
        |
        v
StegoPayload / VerificationStatus
```

Required result:

- `protected/steganography.rs` no longer contains a second implementation of LSB mapping or F5 carrier modification;
- existing public `SteganographyProtector` methods keep their behavior;
- application verification still distinguishes not-found, malformed, wrong/missing key, and verified states as before;
- payload-v1/v2/v3 compatibility remains in the application adapter.

Do not deprecate or remove public methods in this plan unless already covered by the repository's existing deprecation policy.

---

## 9. Phase 5 — keep result/error types narrow

Avoid carrying rights-oriented `EmbedPath`/`EmbedOutcomeSummary` into the generic carrier if doing so forces application semantics into `src/stego/`.

Preferred approach:

- carrier returns a small mechanical result containing output, capacity, and carrier method/status;
- application adapter converts that result into existing `EmbedOutcomeSummary`/warnings.

If reusing existing `EmbedOutcome<T>` is materially simpler and the type is mechanically neutral enough, document that decision in the status ledger. Do not duplicate identical outcome structures solely for architectural purity.

Error handling must preserve structured unsupported/malformed JPEG distinctions where callers need them. Avoid converting all JPEG failures to an opaque string inside the core.

---

## 10. Required tests

Add focused tests proving arbitrary payload behavior independently of StegoEggo payload-v3.

Required LSB tests:

```text
raw_lsb_arbitrary_bytes_roundtrip
raw_lsb_binary_zero_ff_payload_roundtrip
raw_lsb_exact_capacity_outcome
raw_lsb_wrong_seed_not_equal
raw_lsb_legacy_scheme_fixture_roundtrip
raw_lsb_current_scheme_roundtrip
raw_lsb_has_no_rights_metadata_dependency
```

Required JPEG tests:

```text
raw_jpeg_arbitrary_bytes_roundtrip_supported_fixture
raw_jpeg_binary_zero_ff_payload_roundtrip
raw_jpeg_capacity_matches_supported_coefficients
raw_jpeg_wrong_seed_not_equal
raw_jpeg_container_segments_preserved
raw_jpeg_progressive_reports_unsupported_payload_embedding
qtable_seed_hint_roundtrip_does_not_imply_payload_success
```

Application-adapter regression tests:

```text
stegoeggo_png_v3_roundtrip_after_carrier_extraction
stegoeggo_webp_v3_roundtrip_after_carrier_extraction
stegoeggo_jpeg_v3_roundtrip_after_carrier_extraction
stegoeggo_hmac_wrong_key_classification_unchanged
stegoeggo_legacy_lsb_fixture_still_extracts
```

Raw arbitrary-payload tests must not call `generate_payload_for_context()`.

---

## 11. Acceptance criteria

Plan 059 is complete only when:

1. `src/stego/` contains application-neutral LSB and JPEG carrier modules, or an equivalently clear internal module boundary.
2. Carrier modules accept arbitrary `&[u8]` payloads and explicit seed/configuration.
3. Carrier modules do not import rights-policy, legal-metadata, protection-level, `ProtectionContext`, or `StegoPayload` types.
4. Raw LSB extraction requires caller-provided payload length/configuration and returns raw bytes without interpreting StegoEggo payload versions.
5. Raw JPEG extraction likewise returns arbitrary raw payload bytes without parsing rights/provenance fields.
6. JPEG parser/coefficient/entropy types remain crate-private implementation details behind the carrier facade.
7. Q-table seed handling is an explicit mechanical hint operation and is not conflated with successful payload extraction.
8. `SteganographyProtector` delegates LSB and JPEG carrier mechanics to the extracted modules.
9. No duplicate LSB permutation/embed implementation remains in the application adapter.
10. No duplicate F5 carrier embed implementation is introduced outside the existing DCT engine/carrier facade.
11. Existing StegoEggo v1/v2/v3 extraction and HMAC/CRC behavior remains passing.
12. Plan 058 legacy/current LSB compatibility remains passing.
13. Supported JPEG container-preservation regressions remain passing.
14. No public generic API is stabilized yet; new carrier functions may remain `pub(crate)` until Plan 062.
15. No new dependencies, image formats, CI jobs, release actions, or version changes are introduced.
16. `./scripts/check.sh` passes.
17. `plans/059-status.md` records the moved-function inventory, implementation commits, focused test commands, and final dependency-boundary disposition.

---

## 12. Handoff notes for Plan 060

Plan 060 may optimize only the carrier internals established here. It must not move application payload parsing back into the carrier for convenience.

Before Plan 060 begins, a reviewer should be able to point to one function for raw LSB embedding and one function for raw encoded-JPEG embedding and demonstrate that both can embed arbitrary bytes without constructing a `ProtectionContext`.