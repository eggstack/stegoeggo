# Plan 073 Status

Status: COMPLETE — implementation verified retrospectively; the required
pre-edit ledger chronology was missed and is documented by Plan 075.

Implementation ledger for [Plan 073](073-generic-stego-public-api-hardening.md).

## Chronology reconciliation

- Implementation commit: `8fd01530443ee73d77377ca8a8018a59e04d5b32`
- Predecessor: `9125e0f`
- Later independent closure audit: `09eba2a` / [Plan 074 status](074-status.md)
- Chronology defect: this status file was not present before the Plan-073
  source edits. It was reconstructed as evidence under Plan 075 and must not
  be read as if it satisfied the original pre-edit ledger requirement.

The rows below are source-backed verification of the implementation that
landed in `8fd0153`; they are not fabricated contemporaneous records.

| ID | Requirement | Status | Evidence |
|---|---|---|---|
| R01 | Fallible LSB redundancy/config validation exists | Complete | `LsbConfig::try_with_redundancy` + `LsbConfig::try_new` added in `lsb_internal.rs`, re-exported via `lsb` |
| R02 | Fallible JPEG redundancy/config validation exists | Complete | `JpegConfig::try_with_redundancy` + `JpegConfig::try_new` added in `jpeg.rs` |
| R03 | Compatibility handling for existing panicking builders is explicit | Complete | `with_redundancy` retained; rustdoc points readers to fallible path; no behavior change |
| R04 | Raw/framed/in-place docs use consistent terminology | Complete | Module-level docs in `lsb.rs`, `jpeg.rs`, `frame.rs` use consistent terms; summary section in carrier README |
| R05 | Capacity units documented per carrier | Complete | `CapacityReport`, `EmbedReport`, `InPlaceEmbedReport` rustdoc + module-level docs spell out RGB slots vs AC coefficients |
| R06 | CRC vs authentication limitation documented | Complete | `frame.rs` module-level docs + carrier README "Security and robustness limits" section |
| R07 | Default public API does not expose carrier internals | Complete | `compile_fail` doctests in `lib.rs` guard `jpeg_transcoder` + `lsb_internal`; carrier `application_support` is `#[doc(hidden)]` and feature-gated |
| R08 | Application-support remains hidden/narrow | Complete | `pub mod application_support` gated and `#[doc(hidden)]`; not re-exported by root `stegoeggo::stego` |
| R09 | Root re-export matches intended stable carrier surface | Complete | root `stegoeggo::stego` re-exports only operation-level modules: `error`, `frame`, `jpeg`, `lsb`, `types`, plus `CapacityReport`/`EmbedReport`/`InPlaceEmbedReport`/etc. |
| R10 | Direct dependency footprint does not grow without necessity | Complete | carrier `Cargo.toml` unchanged (image, jpeg-encoder, crc32fast, thiserror) |
| R11 | Public examples compile using only documented API | Complete | `examples/generic_stego.rs` uses only public stego modules; `cargo run --example generic_stego` round-trips all four paths |
| R12 | Malformed/untrusted config tests do not panic on fallible path | Complete | 5 new unit tests per config; `std::panic::catch_unwind` proves no panic on `0/11/MAX/100/1_000_000`; 4 new public_stego_api tests |
| R13 | Semver/deprecation implications are recorded truthfully | Complete | 073 adds no removals; panicking `with_redundancy` retained; no major/minor bump |
| R14 | Workspace checks pass | Complete | `./scripts/check.sh` passes: 1639 tests + 25 ignored, 38 suites; fmt/clippy clean |

## Scope decisions

- Add `try_with_redundancy` alongside `with_redundancy` (Plan 073 Option A). Keep
  caller-side `new()` panicking-free (defaults are always valid).
- Add `try_new(seed, redundancy)` constructors as the primary fallible entry.
  `new(seed)` remains as the trusted-default constructor.
- Expose a small `validate_redundancy()` helper in `constants.rs` so the LSB and
  JPEG builders cannot drift on the valid domain.
- Keep the public API surface minimal: fallible *setter* + fallible *constructor*
  on the existing config types. No new public modules.
- Documentation updates stay inside the carrier crate, root `stegoeggo::stego`,
  `stegoeggo-stego/README.md`, and `architecture/protected-steganography.md`.
  No new public architecture documents.
- The `R10` constraint forbids adding dependencies. The new fallible path uses
  only the existing `StegoError::InvalidConfig` variant.

## Implementation order

1. Phase 1 — fallible configuration construction.
2. Phase 2 — capacity/unit terminology audit + framed-overhead clarification.
3. Phase 3 — error contract audit (no new variants; verify existing coverage).
4. Phase 4 — stable boundary audit (compile-fail doctests already present;
   verify they hold against new code).
5. Phase 5 — dependency/feature audit (no changes expected).
6. Phase 6 — independent-consumer documentation.
7. Phase 7 — examples tests.
8. Phase 8 — verification.

## Verification evidence

- `cargo test -p stegoeggo-stego --all-features` — 123 passed (90 lib + 33 doc)
- `cargo test -p stegoeggo --test public_stego_api --all-features` — 43 passed
- `cargo test --workspace --exclude stegoeggo-fuzz --all-features` — 1639 passed, 25 ignored, 38 suites
- `cargo fmt --all -- --check` — clean
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` — clean
- `cargo check -p stegoeggo --no-default-features` — clean
- `cargo package -p stegoeggo-stego --allow-dirty` — packages successfully

## Public API additions

- `stegoeggo_stego::lsb::LsbConfig::try_new(seed, redundancy) -> Result<Self, StegoError>`
- `stegoeggo_stego::lsb::LsbConfig::try_with_redundancy(self, redundancy) -> Result<Self, StegoError>`
- `stegoeggo_stego::jpeg::JpegConfig::try_new(seed, redundancy) -> Result<Self, StegoError>`
- `stegoeggo_stego::jpeg::JpegConfig::try_with_redundancy(self, redundancy) -> Result<Self, StegoError>`

All four are re-exported through the root `stegoeggo::stego` namespace.

## Completion criteria

All 14 rows are Complete. `./scripts/check.sh` passes locally. The
fallible configuration path is documented in module-level rustdoc, the
carrier README, the root README, and the architecture docs. No version
bumps, no new dependencies, no release/CI changes.
