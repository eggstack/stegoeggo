# StegoEggo Long-Term Specification

Status: canonical end-state specification. Stable; amend only for intentional
product-direction change, contradiction, accepted ADR, or explicit user
direction. A corrective pass never justifies changing these requirements.

StegoEggo writes rights-reservation metadata (primary channel) and optional
steganographic markers (best-effort redundant channel) into PNG, JPEG, and
WebP images. It is not DRM and not proof of training.

## 1. Product identity

1. The primary protection channel is visible/machine-readable
   rights-reservation metadata: canonical `plus:DataMining` XMP, IPTC DMI,
   copyright notice, usage terms, and legal-metadata fields.
2. The redundant channel is a hidden marker (V3 payload) embedded via LSB
   (PNG/WebP output) or F5-style DCT (JPEG output), optionally HMAC
   authenticated and optionally tiled for crop resistance.
3. Metadata provides legal evidence of intent and survives where markers
   are stripped; markers never substitute for metadata.
4. Without a MAC key, marker verification is CRC32 integrity only and is
   forgeable. This MUST be documented, never silently upgraded to an
   authentication claim.

## 2. Canonical API invariants

1. `ProtectionRequest` + `RightsPolicy` are canonical (Release 4+). New
   processing features go in `ProtectionRequest` / `ProcessingOptions` /
   `ProtectionChannels` first; legacy `ProtectionLevel` / `EvidenceProfile` /
   `ProtectionContext` builders only translate via `request_from_legacy()`
   and are removed at v1.0.0 at the earliest (see `DEPRECATIONS.md`).
2. `verify_image_bytes_report` is the canonical rich verification
   operation (one rights parse + one marker search). `verify_image_bytes`
   (`VerificationStatus`, direct return), `verify_image_bytes_detailed`
   (`VerificationResult`), and `verify_legal_notice`
   (`NoticeVerification`) are projections of the same facts.
3. `VerificationStatus` is live and NOT deprecated.
4. The generic carrier surface (`stegoeggo-stego`, re-exported as
   `stegoeggo::stego`) is format-agnostic stego for arbitrary payloads and
   knows nothing about rights protection.

## 3. Execution invariants

1. Byte paths carry metadata; pixel paths (`process_image`,
   `process_images_parallel`) embed stego only because container metadata
   does not survive pixel round-trips.
2. Carrier family follows the OUTPUT format (`JPEG ? DCT : LSB`).
   JPEG→JPEG uses a byte-only fast path (no pixel decode). JPEG→PNG/WebP
   is one pixel decode plus raster LSB, never transient DCT.
3. Reproducible output requires an explicit seed and
   `with_timestamp_override(...)`; `default()` uses CSPRNG.
4. `#![forbid(unsafe_code)]` holds in the library and carrier crates.
5. `MIN_PAYLOAD_SIZE` (28) is a parsing threshold, not an output size
   (V3 = 36 non-MAC / 48 MAC bytes). `LegalMetadata::MAX_FIELD_LEN` is
   8192 over 16 validated fields.

## 4. CLI invariants

1. Canonical commands: `protect`, `inspect`, `verify`, `version`, `update`.
   Root positional protection syntax and `--verify` are 0.x compatibility
   aliases. Exit codes: 0 ok, 1 error, 2 config, 3 integrity, 4
   verified-but-untrusted manifest, 5 internal.
2. `version` prints exactly `stegoeggo X.Y.Z` first, offline.
3. `update` authority is the crates.io stable `stegoeggo-cli` version, then
   the matching GitHub Release asset with checksum, candidate identity,
   and candidate version verified before self-replacement. Cargo fallback
   only for unsupported targets or exact-asset HTTP 404.

## 5. Release invariants

1. Manual only: no crates.io publication from Actions, no tag-triggered
   publication. Three crates share one version with exact `=X.Y.Z` deps;
   publish order carrier → library → CLI.
2. Binary assets use versionless names in `scripts/release-targets.txt`,
   each with a `.sha256` sidecar; the release feature set is the CLI
   package default (`signatures`).
3. Required CI is one job running `scripts/check.sh`. Specialist checks
   (external conformance, docs-rs, MSRV package, fuzz, deny, semver)
   never gate merges without a maintainer decision.

## 6. Non-goals

C2PA integration (deferred per `architecture/adr-c2pa.md`), DRM
enforcement, proof-of-training claims, standard-compliant ISCC, and
automated release publication.
