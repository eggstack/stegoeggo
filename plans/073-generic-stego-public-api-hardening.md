# Plan 073: Generic Stego Public API Hardening

Status: Ready for implementation

Roadmap: `plans/069-stego-api-ergonomics-and-adapter-simplification-roadmap.md`

Depends on: Plan 072 complete.

Audited planning baseline: `main` at `f717fd5c99adf17be2bc94d315c71d18f8c77c3d`

Authoritative implementation ledger to create before product edits: `plans/073-status.md`

---

## 1. Purpose

After Plans 071/072, `stegoeggo-stego` will have a useful set of raw, framed, and in-place operations. This plan hardens that surface for independent Rust-library consumption without exposing low-level JPEG internals or introducing a generalized framework.

Primary targets:

1. provide fallible configuration validation so untrusted/user-derived redundancy values do not require a panic;
2. make raw vs framed vs in-place contracts obvious in rustdoc/README;
3. audit stable exports so parent-application support does not leak into the generic API;
4. ensure errors and capacity units are consistent enough for callers to build services/CLIs around the crate;
5. keep dependency and feature footprint small.

This is an API-polish plan, not a carrier redesign.

---

## 2. Frozen boundaries

Do not expose as stable API:

```text
JpegHeader
Coefficients
JpegTranscoder
DctStegoF5
Huffman/entropy state
raw DCT blocks
LSB permutation helpers
slot-to-pixel mapping internals
TiledJpegSearch / application-support helpers through the default facade
```

Do not add:

- a `Carrier` trait;
- dynamic dispatch;
- codec plugins;
- new media types;
- raw RGB/stride-buffer APIs in this roadmap;
- async wrappers inside `stegoeggo-stego`;
- serde requirements merely to serialize config/report structs.

The root application may continue to re-export the stable generic modules as `stegoeggo::stego`.

---

# Phase 0 — status ledger and public API inventory

Create/track `plans/073-status.md` before edits.

Generate/record a concise public inventory from carrier `lib.rs`, `lsb.rs`, `jpeg.rs`, `frame.rs`, `error.rs`, and public result types.

Start rows:

```text
R01 fallible LSB redundancy/config validation exists
R02 fallible JPEG redundancy/config validation exists
R03 compatibility handling for existing panicking builders is explicit
R04 raw/framed/in-place docs use consistent terminology
R05 capacity units documented per carrier
R06 CRC vs authentication limitation documented
R07 default public API does not expose carrier internals
R08 application-support remains hidden/narrow
R09 root re-export matches intended stable carrier surface
R10 direct dependency footprint does not grow without necessity
R11 public examples compile using only documented API
R12 malformed/untrusted config tests do not panic on fallible path
R13 semver/deprecation implications are recorded truthfully
R14 workspace checks pass
```

---

# Phase 1 — fallible configuration construction

Current builder methods such as `with_redundancy()` assert the allowed `1..=10` range.

Provide a fallible path suitable for values coming from config files, CLI parsing, network requests, or other untrusted input.

Preferred options, in order:

### Option A — fallible setter alongside compatibility builder

```rust
impl LsbConfig {
    pub fn try_with_redundancy(self, value: usize) -> Result<Self, StegoError>;
}

impl JpegConfig {
    pub fn try_with_redundancy(self, value: usize) -> Result<Self, StegoError>;
}
```

Keep existing `with_redundancy` temporarily if removing/changing it would be a breaking API change.

### Option B — fallible full constructor

```rust
LsbConfig::try_new(seed, redundancy)
JpegConfig::try_new(seed, redundancy)
```

This may coexist with `new(seed)` defaults.

Do not silently clamp invalid values. Invalid redundancy must return a structured configuration error.

Use one shared private validation helper/constant so LSB and JPEG cannot drift on the valid domain.

If the crate's published semver policy permits deprecating panicking builders, add a deprecation note pointing to the fallible path, but do not remove them or bump major/minor versions in this plan.

Tests:

- 0 rejected;
- 11 rejected;
- 1 accepted;
- 10 accepted;
- invalid fallible calls do not panic;
- valid config behavior is unchanged.

---

# Phase 2 — report and capacity terminology audit

Document exact units everywhere they appear:

- LSB `available/required`: RGB carrier slots;
- JPEG `available/required`: eligible DCT AC coefficient capacity as currently implemented;
- `payload_bytes` in raw operations: raw bytes placed in carrier;
- framed operation report behavior: explicitly state whether frame overhead is included;
- `actual_redundancy`: the redundancy actually used after capacity negotiation.

Do not rename stable fields solely for aesthetics unless the current name is materially misleading. Prefer rustdoc clarification over churn.

Add examples that check `CapacityReport::is_sufficient()` before embedding.

---

# Phase 3 — error-contract audit

Review `StegoError` against all public operation failure modes after Plans 071/072.

Required qualities:

- malformed encoded input distinguishable from unsupported JPEG structure;
- invalid config distinguishable from capacity insufficiency;
- frame-not-found/malformed/checksum failures distinguishable enough for caller policy;
- no public method panics for normal malformed image/payload/config inputs when using the fallible configuration path;
- error messages do not expose internal implementation details as a required parsing contract.

Do not introduce dozens of highly specific error variants. Keep the enum operation-level.

---

# Phase 4 — stable boundary audit

Source-audit carrier `lib.rs` and root `stego` re-export.

Required default stable modules/types should remain small and operation-oriented:

```text
lsb
jpeg
frame
error
CapacityReport
EmbedReport
in-place report/summary from Plan 072
StegoError
JpegUnsupportedReason
operation-level embed status/path types only where intentionally public
```

Check compile-fail boundary tests proving these remain inaccessible:

```text
jpeg_transcoder
JpegHeader
Coefficients
DctStegoF5
lsb_internal permutation helpers
```

`application_support` remains `#[doc(hidden)]`, feature-gated, and not re-exported by root generic `stego` facade.

Do not make a parent-crate helper stable just to silence visibility friction; fix internal visibility instead.

---

# Phase 5 — dependency/feature audit

Run a narrow dependency review of `stegoeggo-stego/Cargo.toml` after new APIs land.

The expected direct dependency set should remain approximately:

```text
image
jpeg-encoder
crc32fast
thiserror
```

No new dependency is expected for this plan.

If a proposed convenience API appears to require a new dependency, first implement it with existing/std facilities or document why it is not worth adding.

Keep `application-support` as the only parent-specific feature unless current source proves another existing feature already exists.

No feature powerset CI is required.

---

# Phase 6 — independent-consumer documentation

Rewrite/expand `stegoeggo-stego/README.md` to stand alone for a crates.io/docs.rs reader who has never heard of StegoEggo rights metadata.

Recommended structure:

```text
What this crate is
Supported carriers and limitations
Raw API
Framed convenience API
In-place LSB API
Capacity and redundancy
JPEG support probing
Error handling
Security/robustness limits
Relationship to stegoeggo application crate
```

Be precise:

- LSB is fragile under lossy re-encoding;
- JPEG F5 support is a bounded supported JPEG subset, not arbitrary JPEG;
- generic frame CRC is not authentication/encryption;
- seed is not a cryptographic secret unless caller uses it as part of a broader protocol (do not claim secrecy);
- no forensic robustness guarantee.

Update root `stegoeggo::stego` rustdoc enough to point users toward the dedicated crate for generic use.

---

# Phase 7 — public examples/tests

Ensure examples compile for:

1. raw LSB round-trip;
2. framed LSB round-trip without stored length;
3. in-place LSB embed;
4. raw JPEG with explicit actual redundancy;
5. framed JPEG without stored actual redundancy;
6. fallible config from a runtime `usize`.

Prefer one concise `examples/generic_stego.rs` plus rustdoc snippets rather than many tiny example binaries.

Public integration tests must import APIs exactly as external consumers would.

---

# Phase 8 — verification

Minimum:

```bash
cargo test -p stegoeggo-stego
cargo test -p stegoeggo-stego --doc
cargo test -p stegoeggo --test public_stego_api --all-features
cargo check -p stegoeggo-stego
cargo package -p stegoeggo-stego --allow-dirty
./scripts/check.sh
```

`cargo package` is structural evidence only. Do not publish.

Record warnings truthfully; do not add code solely to suppress harmless package-only dead-code warnings unless they reflect a real boundary problem.

---

## Completion criteria

Plan 073 is COMPLETE only when:

- fallible configuration validation exists for both public carriers;
- compatibility with existing builders is deliberate/documented;
- public units/error semantics are clear;
- default stable API remains operation-level and private internals remain private;
- independent crate documentation covers raw/framed/in-place use and limitations;
- no unnecessary dependency was added;
- package structural check and `./scripts/check.sh` pass;
- no version/release/CI change occurs.