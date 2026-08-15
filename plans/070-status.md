# Plan 070 Status: Application Stego Adapter Decomposition

## Baseline

- Starting HEAD: `48bffce` (`plans: add stego API ergonomics roadmap`)
- Working tree: clean on `main`, tracking `origin/main`
- Workspace versions: root `stegoeggo` 0.3.2, carrier `stegoeggo-stego` 0.3.2, CLI 0.3.2
- Root carrier dependency: `stegoeggo-stego = 0.3.2` with the `application-support` feature
- Baseline source: `src/protected/steganography.rs`, 4,888 lines / 192,664 bytes

## Source inventory

`SteganographyProtector` has two implementation blocks: the main implementation beginning at line 197 and the `Default`/`Protector` implementations beginning at lines 3270/3276. The main block combines:

- facade construction and public verification/extraction entry points;
- current V3 marker generation and legacy context adapters;
- LSB and JPEG embedding dispatch, including tiled paths and seed fallback writes;
- LSB/JPEG/tiled extraction search and seed discovery;
- V3 prefix/header probing, payload parsing, CRC/HMAC classification, and legacy V1/V2 decoding;
- `StegoPayload` accessors and unit tests.

Defined application types and shared values include `CandidateOutcome`, `V3PrefixResult`, `PayloadMalformedReason`, `ValidatedV3Header`, `V3ProbeResult`, `ExtractionTrace`, `SteganographyProtector`, `StegoPayload`, the V1/V2/V3 payload-size constants, supported-version data, fallback seeds, and the V3 prefix size.

The module imports `stegoeggo_stego::application_support` as `carrier_support`, plus the generic JPEG facade through `crate::stego::jpeg`. It uses only operation-level carrier APIs; no root import reaches JPEG parser, coefficient, Huffman, or F5 implementation types.

Direct callers found in the current tree:

- `src/lib.rs`: canonical plan execution, compatibility pipeline execution, seed fallback, capacity estimation, and public re-export use;
- `src/detached/verify.rs`: detached-manifest embedded-payload verification;
- `src/protected/notice_verification.rs`: notice verification integration;
- `tests/`, `stegoeggo-cli/tests/`, and module unit tests: public API, legacy, payload, LSB, JPEG, and tiled behavior coverage.

## Status rows

| ID | Status | Evidence |
|---|---|---|
| R01 | CLOSED | `src/protected/steganography.rs` moved to `src/protected/steganography/mod.rs`; minimal-feature check passes |
| R02 | CLOSED | Current marker construction is owned by `steganography/marker.rs` |
| R03 | CLOSED | Carrier selection and embedding dispatch are owned by `steganography/embed.rs` |
| R04 | CLOSED | Seed discovery and extraction/search orchestration are owned by `steganography/extract.rs` |
| R05 | CLOSED | Payload parsing, integrity/authentication, and classification are owned by `steganography/verify.rs` |
| R06 | CLOSED | V1/V2 decoding and ECC compatibility adapters are isolated in `steganography/legacy.rs` |
| R07 | CLOSED | Root adapter files contain no carrier algorithm implementation; operations delegate to carrier support APIs |
| R08 | CLOSED | Root imports only `application_support` and generic JPEG facade operations; no JPEG parser/coefficient/F5 internals are imported |
| R09 | CLOSED | Plan-driven methods remain in the canonical embedding/marker path and compile unchanged |
| R10 | CLOSED | Tiled candidate evaluation remains in one extraction module and retains the carrier-owned search/context calls |
| R11 | CLOSED | Focused stego, tiled-JPEG, legacy, payload, and public carrier API tests pass |
| R12 | CLOSED | Focused root stego tests pass: 78, 2, 34, and 123 tests respectively |
| R13 | CLOSED | `./scripts/check.sh` passes: fmt, strict clippy, minimal-feature check, workspace all-feature tests, and doctests |
| R14 | CLOSED | README, AGENTS.md, and architecture references now describe the decomposed adapter; Plan 074 remains separate |

## Change log

## Verification log

- `cargo check -p stegoeggo --no-default-features` — PASS
- `cargo test -p stegoeggo --all-features stego` — PASS (78 passed, 1,335 filtered)
- `cargo test -p stegoeggo --all-features tiled_jpeg` — PASS (2 passed, 1,411 filtered)
- `cargo test -p stegoeggo --all-features legacy` — PASS (34 passed, 1 ignored, 1,378 filtered)
- `cargo test -p stegoeggo --all-features payload` — PASS (123 passed, 1,290 filtered)
- `cargo test --test public_stego_api --all-features` — PASS (29 passed)

Post-decomposition source sizes: `mod.rs` 1,835 lines, `marker.rs` 221, `embed.rs` 401, `extract.rs` 1,494, `verify.rs` 893, and `legacy.rs` 97. The total is 4,941 lines including module headers and formatting; the facade is no longer the implementation monolith.
