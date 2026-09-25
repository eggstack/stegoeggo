# StegoEggo Terminology and Domain Model

Status: canonical language. Use these terms exactly; do not redefine them
per subsystem.

## Core request model

| Term | Meaning |
|---|---|
| `RightsNotice` | Resolved rights statement rendered into containers (DMI, PLUS, notice text, timestamps). |
| `RightsPolicy` | Explicit machine-readable reservation (`Unspecified`, `Allowed`, `ProhibitedAiMlTraining`, `ProhibitedGenerativeAiTraining`, `ProhibitedExceptSearchIndexing`, `ProhibitedAllDataMining`, `ProhibitedSeeConstraints`). Never inferred. |
| `ProtectionRequest` | Canonical user-constructed operation (notice + policy + channels + options). |
| `ProcessingOptions` | Output format, JPEG quality, progressive flag, max dimension, metadata-update policy, stego redundancy, content hash, timestamp override. |
| `ProtectionChannels` | Which channels run: metadata, hidden marker, or both. |
| `ResolvedProtectionPlan` | Immutable validated plan built by `resolve_request()`; executors in `src/pipeline.rs` run it. |
| `ExecutionReport` | What ran: effective policy/DMI, stego attempted, format transcoded, embed summary, resource usage, warnings. |

## Legacy adapters (deprecated, functional through 0.x)

`ProtectionLevel` (`Disabled`/`Light`/`Standard`), `EvidenceProfile`,
`ProtectionContext` (private fields, `Option<bool>` injection flags,
`#[serde(skip)]` config). They translate via `request_from_legacy()` and
gain no independent features. Removal at v1.0.0, never in 0.x.

## Metadata channel

| Term | Meaning |
|---|---|
| DMI | IPTC Data-Mining reservation value consumed/emitted per format mapping. `has_notice()` is true for ANY DMI value including `Allowed`/`Unspecified`. |
| PLUS | Canonical `plus:DataMining` XMP signal. |
| `LegalMetadata` | 16 validated fields (≤8192 bytes each); presence auto-enables legal-claims injection. Explicit `false` with legal metadata emits `ContradictoryLegalClaims`. |
| `MetadataUpdatePolicy` | Merge behavior for pre-existing metadata. |
| Metadata-only | `HiddenMarkerMode::Disabled`: inject rights without pixel/coefficient changes. |

## Marker channel

| Term | Meaning |
|---|---|
| V3 payload | Current wire format: 32-byte core + TLV extensions (36 bytes CRC, 48 bytes HMAC). Only format written. |
| V1/V2 payloads | Extraction-only legacy (V1 76 bytes, V2 ECC 100 bytes). |
| Seed hint | 96-bit seed embedded in JPEG quantization-table LSBs; transactional (all bits or `InsufficientHintCapacity`). |
| Redundancy | Validated `1..=10` (`Redundancy` type); JPEG capacity units are AC coefficients with `|coef| >= 2`. |
| `HiddenMarkerMode` | `Disabled` \| `SeedOnly` \| `BestEffort` \| `Tiled`. |
| Framed | Self-describing `frame::{encode,decode}` wrapper so extraction needs no retained length. |
| Tiled | Crop-resistant repetition over `TileConfig` with explicit `max_origins ≤ MAX_TILED_ORIGINS (4096)`. |
| `PreparedJpeg` | Opaque single-decode JPEG handle for repeated ops (borrows source bytes). |
| Best-effort | Application policy on `jpeg::embed`: auto-downgrade redundancy, seed-only fallback. `*_strict` variants embed exact redundancy or fail with no output. |
| `EmbedOutcome` / `EmbedStatus` / `EmbedPath` | Parent-owned application vocabulary (retained through 0.x), not generic carrier semantics. |

## Verification model

| Term | Meaning |
|---|---|
| `VerificationReport` | Canonical rich result (`verify_image_bytes_report`); includes evidence strength and trust evaluation. |
| `VerificationStatus` | Coarse stego-only projection (`verify_image_bytes`, direct return). |
| `VerificationResult` | Detailed projection (`Verified`/`Corrupted`/`MetadataOnly`/`NotFound`). |
| `NoticeVerification` | Legal-notice projection (`verify_legal_notice`). |
| `EvidenceStrength` | `NoNoticeFound` \| `MetadataNoticeOnly` \| `MetadataNoticeAndBestEffortStego` \| `MetadataNoticeAndAuthenticatedProvenance`. |
| Authenticated | HMAC payload found AND key supplied AND tag verified. `attempted` alone is not authentication. |

## Distribution

Detached manifest (signed JSON sidecar, `TrustPolicy` caller-owned),
conformance report (`stegoeggo-conformance`, exit 0–5), bootstrap
installers (`packaging/install.sh|.ps1`), eggfetch updater transport.
