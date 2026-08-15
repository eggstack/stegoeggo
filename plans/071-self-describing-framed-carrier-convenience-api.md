# Plan 071: Self-Describing Framed Carrier Convenience API

Status: Ready for implementation

Roadmap: `plans/069-stego-api-ergonomics-and-adapter-simplification-roadmap.md`

Depends on: Plan 070 complete.

Audited planning baseline: `main` at `f717fd5c99adf17be2bc94d315c71d18f8c77c3d`

Authoritative implementation ledger to create before product edits: `plans/071-status.md`

---

## 1. Purpose

`stegoeggo-stego` already exposes a generic application-neutral frame:

```text
2-byte magic
1-byte version
4-byte payload length
4-byte CRC32
payload bytes
```

with `frame::encode`, `frame::decode`, and `frame::decode_prefix`.

However, callers must manually compose frame and carrier operations. The current generic example says framing allows recovery without a caller-known payload length, but still extracts using the already-known `framed.len()`.

This plan adds first-class framed convenience operations so normal generic use does not require retaining:

- original payload length for LSB; or
- original payload length **and** `EmbedReport::actual_redundancy` for JPEG.

Raw carrier APIs remain unchanged and available.

---

## 2. Required public behavior

Preferred public surface:

```rust
stegoeggo_stego::lsb::embed_framed(...)
stegoeggo_stego::lsb::extract_framed(...)

stegoeggo_stego::jpeg::embed_framed(...)
stegoeggo_stego::jpeg::extract_framed(...)
```

Equivalent names are acceptable if they are more idiomatic, but avoid a new builder/framework.

Expected ergonomic use:

```rust
let cfg = lsb::LsbConfig::new(42);
let report = lsb::embed_framed(&image, b"payload", &cfg)?;
let recovered = lsb::extract_framed(&report.output, &cfg)?;
assert_eq!(recovered, b"payload");
```

JPEG equivalent must not require the caller to retain `report.actual_redundancy`.

The convenience API is a composition layer over the existing generic frame and raw carriers; it is not a new carrier format.

---

## 3. Frozen contracts

Do not change:

- raw `lsb::embed/extract/capacity` semantics;
- raw `jpeg::embed/extract/capacity/probe_support` semantics;
- existing frame wire format or magic/version;
- frame maximum payload bound;
- corrected LSB mapping;
- JPEG F5 mapping;
- JPEG Q-table seed-hint behavior;
- application-support API;
- StegoEggo payload-v3.

Framed APIs must never discover seeds from XMP, legal metadata, fallback seeds, or StegoEggo application state. Seed/configuration remains explicit.

---

# Phase 0 — ledger and exact API inventory

Create and track `plans/071-status.md` first.

Record current signatures of:

```text
frame::{encode, decode, decode_prefix}
lsb::{capacity, embed, extract, LsbConfig}
jpeg::{capacity, embed, extract, JpegConfig, probe_support}
EmbedReport
CapacityReport
StegoError frame/capacity variants
```

Start rows:

```text
R01 LSB framed embed is application-neutral composition
R02 LSB framed extract needs no caller-known payload length
R03 JPEG framed embed is application-neutral composition
R04 JPEG framed extract needs no caller-known payload length
R05 JPEG framed extract needs no retained actual_redundancy
R06 framed extraction remains bounded by frame and carrier capacity
R07 malformed/bad-magic/bad-version/bad-CRC behavior is deterministic
R08 wrong seed fails without panic/unbounded work
R09 raw carrier APIs unchanged
R10 frame wire format unchanged
R11 no rights/application imports enter carrier crate
R12 docs/example demonstrate true independent recovery
R13 carrier tests/doctests pass
R14 workspace checks pass
```

---

# Phase 1 — LSB framed embed

Implement `lsb::embed_framed` as explicit composition:

```text
caller payload
    -> frame::encode(payload)
    -> existing lsb::embed(image, framed_bytes, config)
```

Do not duplicate frame serialization inside `lsb.rs`.

Capacity semantics must use the **framed byte length**, because those are the bytes placed in the carrier.

If returning the existing `EmbedReport<RgbaImage>`, document clearly that `payload_bytes` reflects on-carrier bytes including frame overhead. Do not silently reinterpret `EmbedReport::payload_bytes` only for framed calls.

If this is too confusing in practice, introduce one narrowly named framed report wrapper; do not create a general report hierarchy.

Tests:

- arbitrary binary payload round-trip;
- empty payload;
- exact-capacity boundary including frame overhead;
- payload that fit raw but does not fit framed reports insufficient capacity correctly;
- frame maximum bound rejection occurs before carrier mutation.

---

# Phase 2 — LSB framed extract using prefix-first recovery

Implement the extraction algorithm explicitly:

```text
1. raw-extract FRAME_HEADER_SIZE bytes with caller config
2. frame::decode_prefix(prefix)
3. validate declared total frame length against MAX_FRAME_PAYLOAD and carrier capacity
4. raw-extract exactly total frame length
5. frame::decode(full_frame)
6. return payload bytes
```

Important boundedness requirements:

- never allocate based solely on an unchecked length from carrier bytes;
- `decode_prefix` maximum-length validation must happen before the full extraction buffer is requested;
- compare required carrier capacity for the declared frame length with the image's available capacity before expensive full extraction;
- malformed prefixes return structured `StegoError`, not `None`/panic ambiguity.

Do not add seed guessing.

Tests:

- caller does not retain payload length;
- wrong seed returns a frame-related failure;
- corrupt payload fails CRC;
- declared frame larger than carrier is rejected before full extraction;
- malformed header/version is rejected;
- trailing-byte strictness remains as current `frame::decode` semantics.

---

# Phase 3 — JPEG framed embed

Implement composition:

```text
frame::encode(payload)
    -> jpeg::embed(jpeg_bytes, framed_bytes, config)
```

Preserve all existing `probe_support` and container-preservation behavior.

Do not alter auto-selected actual redundancy.

Tests:

- supported JPEG framed round-trip;
- container-preservation regression remains passing;
- insufficient capacity is reported using framed length;
- progressive/unsupported behavior matches raw embed behavior.

---

# Phase 4 — JPEG framed extraction without retained redundancy

Raw JPEG extraction currently requires the actual redundancy used by embedding. Framed convenience extraction must recover without retaining the `EmbedReport`.

Use a bounded deterministic redundancy probe over the existing valid domain only.

Preferred search order:

1. requested `config.redundancy()` first;
2. lower valid redundancies down to 1 (because current embed may reduce requested redundancy to fit capacity);
3. only probe higher values if compatibility evidence demonstrates they are needed; otherwise do not expand work beyond possible outputs of current embed semantics.

For each candidate redundancy:

```text
extract frame header bytes
    -> decode_prefix
    -> if structurally plausible and bounded, extract exact full frame with SAME redundancy
    -> frame::decode / CRC
    -> return first fully valid frame
```

A prefix that happens to resemble frame magic is not success. Only complete frame validation is success.

Do not use Q-table seed hints to infer redundancy. Do not access private coefficients. Do not search StegoEggo application metadata.

If raw current embed semantics can select a redundancy greater than the requested value, first prove that from source/tests; otherwise the bounded search should remain requested..=1.

Tests must include:

- configured redundancy embeds unchanged and is recovered;
- capacity causes auto-downgrade and framed extraction still recovers without the report;
- wrong first redundancy candidate does not mask later valid candidate;
- wrong seed fails;
- bad CRC candidate does not mask later valid candidate if more redundancy candidates remain;
- unsupported JPEG remains structured unsupported behavior;
- no more than the bounded valid redundancy candidate count is attempted.

Instrumentation may be test-only; do not expose search counters publicly.

---

# Phase 5 — error contract

Audit `StegoError` before adding variants.

Prefer reusing existing errors where semantically accurate:

```text
FrameNotFound
MalformedFrame
FrameChecksumMismatch
InsufficientCapacity
UnsupportedJpeg
InvalidConfig
MalformedInput
```

Add a new error variant only when existing variants cannot distinguish a user-actionable condition.

Document that framed extraction errors are not authentication results. CRC32 detects accidental corruption; a party that can modify the carrier can forge a valid frame.

---

# Phase 6 — public docs and example

Update `stegoeggo-stego/README.md`, public rustdoc, and `examples/generic_stego.rs` so the example genuinely demonstrates persistence-independent recovery:

```text
embed framed payload
write/retain only resulting image and seed/config
later extract framed payload without original length/report
```

The example should retain separate sections for:

- raw API: caller knows exact length and (for JPEG) actual redundancy;
- framed API: convenience recovery without those retained values.

Do not imply that the frame encrypts or authenticates payloads.

---

# Phase 7 — verification

Focused minimum:

```bash
cargo test -p stegoeggo-stego frame
cargo test -p stegoeggo-stego lsb
cargo test -p stegoeggo-stego jpeg
cargo test -p stegoeggo-stego --doc
cargo test -p stegoeggo --test public_stego_api --all-features
./scripts/check.sh
```

Add public-API integration coverage in `tests/public_stego_api.rs` or carrier-crate integration tests so framed behavior is tested outside private modules.

---

## Completion criteria

Plan 071 is COMPLETE only if:

1. LSB framed round-trip needs no caller-known payload length.
2. JPEG framed round-trip needs neither caller-known payload length nor retained actual redundancy.
3. Extraction remains bounded by existing frame/config/capacity limits.
4. Raw APIs and wire formats are unchanged.
5. No application-rights dependencies leak into `stegoeggo-stego`.
6. Documentation accurately distinguishes raw/framed semantics and CRC limitations.
7. `./scripts/check.sh` passes.
8. No version/release/CI change occurs.