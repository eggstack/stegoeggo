# Plan 062: Public Generic Stego API and Framing

Status: Ready for implementation

Roadmap: `plans/057-stego-carrier-library-and-pipeline-simplification-roadmap.md`

Depends on: Plans 058-061 complete.

Audited planning baseline: `main` at `e60b801fda493ddd5e744f5de2a12d7b9489c078`

Authoritative implementation ledger to create before source edits: `plans/062-status.md`

---

## 1. Purpose

Expose the generic carrier layer established by Plans 058-061 as a small, usable Rust library API for arbitrary payload bytes.

The API must support two distinct use cases without conflating them:

1. **raw carrier operations** for callers that know payload length and seed out-of-band; and
2. **framed carrier operations** for callers that want a minimal self-describing payload length plus accidental-corruption detection.

The public generic API must not require callers to construct `ProtectionContext`, `ProtectionRequest`, `RightsPolicy`, DMI values, legal metadata, or StegoEggo `StegoPayload` objects.

This plan stabilizes a carrier facade, not JPEG codec internals and not a new watermarking protocol.

---

## 2. Public module boundary

Preferred root shape:

```rust
pub mod stego;
```

Preferred user-facing structure:

```text
stegoeggo::stego
├── lsb
├── jpeg
└── frame
```

The exact file layout may remain internal, but rustdoc should present a similarly small hierarchy.

Do not re-export the entire existing `protected::steganography` module as the generic API.

Do not make `jpeg_transcoder` public wholesale.

---

## 3. Design principles

The public API must follow these rules:

1. Payloads are arbitrary bytes.
2. Seeds are explicit caller inputs.
3. Raw extraction does not search XMP, EXIF, COM, Q-table hints, or fixed StegoEggo fallback bits for a seed.
4. Raw extraction requires an expected payload byte length.
5. Framed extraction obtains payload length from its own generic frame.
6. The API exposes capacity before embedding.
7. Capacity and embed outcomes use documented units.
8. PNG/WebP pixel-domain APIs operate on `RgbaImage` or a similarly explicit pixel type; they do not pretend to preserve encoded container metadata.
9. JPEG generic APIs operate on encoded JPEG bytes and preserve the supported container semantics.
10. Unsupported JPEG structures return a structured unsupported result/error; they do not silently claim payload embedding succeeded because a seed hint was written.
11. No public type includes rights-policy or legal-notice fields.
12. No public carrier configuration exposes the historical `intensity` field, because carrier mutation amplitude is not controlled by that value.
13. The API should be usable with default crate features; do not introduce a new mandatory feature flag unless Plan 063 later moves the implementation to a dedicated crate.
14. Public names should describe carrier mechanics, not legal evidence.

---

## 4. Phase 0 — API inventory and semver review

Before source edits:

1. Create `plans/062-status.md` and record the actual baseline SHA.
2. Inventory existing public stego-facing symbols, especially:
   - `SteganographyProtector`;
   - `StegoPayload`;
   - verification helpers;
   - public DCT support helpers already re-exported.
3. Identify what remains application-specific and what new API is additive.
4. Confirm this plan does not require removing current public symbols.
5. Record proposed public types/functions in the ledger before implementation.

The goal is an additive v0.x-compatible surface. Do not use this work as an excuse to perform unrelated deprecation cleanup.

---

## 5. Phase 1 — public raw LSB API

Expose a narrow configuration and result model.

A target API may look approximately like:

```rust
use image::RgbaImage;
use stegoeggo::stego::lsb::{LsbConfig, LsbScheme};

let config = LsbConfig::new(seed);
let capacity = stegoeggo::stego::lsb::capacity(&image, payload.len(), &config)?;
let embedded = stegoeggo::stego::lsb::embed(&image, payload, &config)?;
let recovered = stegoeggo::stego::lsb::extract(
    embedded.output(),
    payload.len(),
    &config,
)?;
```

Exact method names may differ, but public capability must include:

- constructor/default for current corrected carrier scheme;
- seed;
- replication/robustness setting with documented capacity cost;
- optional tile size if tiled arbitrary-payload operation is retained publicly;
- exact capacity query;
- embed raw bytes;
- extract raw bytes with expected length.

Legacy scheme exposure:

- It is acceptable to expose the legacy mapping only as an explicitly named compatibility option such as `LsbScheme::LegacyStegoEggoV1`.
- Do not make legacy behavior the default.
- If exposing it would create a misleading generic commitment, keep legacy probing application-private and expose only the corrected public scheme. Record the decision.

Raw API success must not imply authentication. Returned bytes are just carrier output.

---

## 6. Phase 2 — public encoded-JPEG API

Expose JPEG carrier operations as encoded-byte functions.

Approximate shape:

```rust
use stegoeggo::stego::jpeg::JpegConfig;

let config = JpegConfig::new(seed).with_redundancy(2);
let capacity = stegoeggo::stego::jpeg::capacity(&jpeg_bytes, payload.len(), &config)?;
let embedded = stegoeggo::stego::jpeg::embed(&jpeg_bytes, payload, &config)?;
let recovered = stegoeggo::stego::jpeg::extract(
    embedded.output(),
    payload.len(),
    &config,
)?;
```

Requirements:

- callers pass encoded JPEG bytes;
- successful supported input uses the container-preserving DCT path;
- capacity is in usable payload bits/bytes or another clearly documented mechanical unit;
- redundancy cost is explicit;
- current supported/unsupported JPEG subset is documented;
- progressive/restart-bearing/multi-scan unsupported conditions remain explicit;
- output contains no rights metadata unless the caller separately uses the StegoEggo protection API.

Do not expose these as stable public internals:

```text
JpegHeader
HuffmanTable
CoefficientDecoder
CoefficientEncoder
HashMap<u8, Vec<[i16; 64]>> coefficient representation
DctCoefficientRng
```

If users need low-level JPEG internals later, that is a separate API review.

---

## 7. Phase 3 — capacity and embed result types

Use one small mechanical result vocabulary across the public carrier APIs where it genuinely fits.

Suitable concepts:

```text
Capacity { required, available, unit }
EmbedStatus { Embedded, InsufficientCapacity, Unsupported }
EmbedReport { method, payload_bytes, capacity, ... }
```

Avoid exposing application `EvidenceStrength`, rights warnings, DMI, or legal-notice statuses.

If the existing `EmbedOutcome`/`EmbedOutcomeSummary` types are reused, verify their names/variants are carrier-neutral and document exact units. If they carry rights-specific assumptions, create a small `stego`-specific result instead.

Do not create a deep result object with telemetry fields that callers cannot act on.

---

## 8. Phase 4 — explicit JPEG seed-hint API

The JPEG Q-table seed channel may be useful independently, but it must remain clearly separate from payload success.

If exposed publicly, use explicit naming such as:

```rust
stegoeggo::stego::jpeg::embed_seed_hint(...)
stegoeggo::stego::jpeg::extract_seed_hint(...)
```

Rustdoc must state:

- it stores/discovers a seed-like hint in quantization tables;
- it is fragile under requantization/re-encoding;
- it does not prove a payload exists;
- it does not authenticate the payload;
- generic carrier APIs do not automatically use it unless the caller requests that behavior explicitly.

If exposing it materially enlarges the stable surface without helping generic use, keep it crate-private. The status ledger should record the decision.

---

## 9. Phase 5 — add a minimal generic frame

Raw extraction requires payload length. Add one optional generic frame so common callers can recover the length and detect accidental corruption.

The frame must be mechanically generic and independent of StegoEggo payload-v3.

Required fields:

```text
fixed magic
frame version
payload length
payload bytes
CRC32 or equivalent existing lightweight accidental-corruption check
```

Keep the wire format intentionally small. Do not include:

```text
rights policy
DMI
legal metadata
ProtectionLevel
StegoEggo evidence channels
creator identity
signatures
network identifiers
content provenance schema
```

Authentication is not required in the initial generic frame. A caller needing authenticity may embed an authenticated/encrypted payload or use the StegoEggo HMAC/provenance layer. Do not add another cryptographic protocol merely because HMAC dependencies already exist.

The frame specification must define:

- exact magic bytes;
- version value;
- integer endianness;
- maximum payload length;
- CRC coverage;
- malformed-length handling;
- trailing-byte policy.

Use checked length arithmetic and `ResourceLimits` or an equivalent explicit maximum before allocating payload-sized buffers.

---

## 10. Phase 6 — framed convenience APIs

Provide convenience functions or methods such as:

```text
lsb::embed_framed
lsb::extract_framed
jpeg::embed_framed
jpeg::extract_framed
```

or an equivalent `frame::encode/decode` plus carrier composition example.

Prefer composition over duplicating carrier logic.

Conceptual implementation:

```text
frame::encode(payload)
    -> raw bytes
    -> carrier::embed(raw frame)

carrier extraction of fixed frame prefix
    -> determine total frame length
    -> carrier extraction of complete frame
    -> frame::decode + CRC validation
    -> user payload
```

The prefix extraction mapping must be deterministic and independent of unknown final payload length. Plan 058's corrected carrier model should already guarantee this property.

For JPEG, extracting the frame prefix should not require exposing DCT internals publicly.

---

## 11. Phase 7 — error model

Generic callers need to distinguish at least:

```text
invalid configuration
insufficient capacity
malformed input image/JPEG
unsupported JPEG structure
frame not found
malformed frame
frame checksum mismatch
resource limit exceeded
```

Prefer a small structured `StegoError`/carrier error with conversion into the crate's existing `Error` if necessary.

Do not collapse all generic failures to `Error::Steganography(String)` if doing so prevents callers from handling unsupported vs capacity vs malformed-frame conditions.

Conversely, do not duplicate the full root `Error` enum merely to rename variants.

The chosen error boundary must be documented in the status ledger before public exposure.

---

## 12. Phase 8 — documentation and examples

Add rustdoc examples that compile for:

1. arbitrary binary LSB raw round-trip;
2. arbitrary binary JPEG raw round-trip;
3. framed LSB round-trip without caller-known length;
4. framed JPEG round-trip without caller-known length;
5. capacity preflight;
6. unsupported JPEG handling.

Add one repository example if useful, for example:

```text
examples/generic_stego.rs
```

Do not turn the main README into a steganography textbook. Add a concise “Generic carrier API” section pointing to rustdoc and clearly distinguishing it from the rights-protection API.

Security/limitations documentation must state:

- this is best-effort steganography, not encryption;
- seed knowledge is not equivalent to cryptographic secrecy;
- CRC detects accidental corruption, not forgery;
- LSB payloads are fragile under lossy re-encoding/resizing;
- JPEG DCT payloads are not guaranteed across arbitrary recompression;
- Q-table seed hints are weaker than full payload verification.

---

## 13. Required tests

Public API tests:

```text
public_lsb_raw_roundtrip_arbitrary_bytes
public_lsb_capacity_preflight
public_lsb_insufficient_capacity_structured_error_or_outcome
public_jpeg_raw_roundtrip_arbitrary_bytes
public_jpeg_supported_container_preservation
public_jpeg_unsupported_progressive_is_explicit
public_frame_roundtrip_binary_payload
public_frame_checksum_detects_corruption
public_frame_malformed_length_fails_before_large_allocation
public_lsb_framed_roundtrip
public_jpeg_framed_roundtrip
public_generic_api_does_not_emit_rights_metadata
```

Documentation tests must compile under the supported MSRV/toolchain policy.

Add an external-consumer-style integration test that imports only public symbols from `stegoeggo::stego`; do not let it reach crate-private helpers.

---

## 14. Acceptance criteria

Plan 062 is complete only when:

1. `stegoeggo::stego` or an equivalently clear public namespace exists.
2. Public raw LSB embedding/extraction accepts arbitrary bytes, explicit seed/config, and expected payload length on extraction.
3. Public raw JPEG embedding/extraction accepts arbitrary bytes and encoded JPEG input/output while keeping parser/coefficient internals private.
4. Public capacity preflight exists for both carriers and agrees with embed outcomes.
5. No generic public config contains rights policy, DMI, legal metadata, evidence profile, `ProtectionContext`, or `StegoPayload`.
6. `intensity` is not presented as a generic carrier-strength control.
7. Raw generic APIs do not search metadata or StegoEggo fallback channels for seeds.
8. Unsupported JPEG structures are distinguishable from insufficient capacity and malformed input.
9. The minimal generic frame has a documented fixed wire format with version, payload length, and CRC-based accidental-corruption detection.
10. Framed APIs can recover arbitrary payload bytes without caller-known payload length.
11. Generic framing does not duplicate payload-v3 rights/provenance semantics or introduce a second authentication/signature protocol.
12. Public examples compile and show both raw and framed use.
13. An external-consumer integration test uses only the public facade successfully.
14. Existing StegoEggo rights-protection APIs continue using the same carrier internals and remain passing.
15. Existing legacy LSB compatibility remains passing.
16. No JPEG internals are made public merely for convenience.
17. No new image formats, CI jobs, release automation, version bump, publication, or language bindings are introduced.
18. `./scripts/check.sh` passes.
19. `plans/062-status.md` records the final public API inventory, frame wire specification, semver review, and focused evidence.

---

## 15. Handoff to Plan 063

Plan 063 must evaluate the API as it actually exists after this plan. It must not redesign the public surface merely to justify a crate split.

A dedicated carrier crate is justified only if the public boundary here is already clean and moving it reduces real dependency/consumer cost without adding circular dependencies or publication complexity disproportionate to the project.