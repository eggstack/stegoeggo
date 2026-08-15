# Plan 071 Status: Self-Describing Framed Carrier Convenience API

## Baseline

- Starting HEAD: `ffdab7a` (`refactor: decompose application stego adapter`)
- Working tree: clean on `main`, tracking `origin/main`
- Workspace versions: root `stegoeggo` 0.3.2, carrier `stegoeggo-stego` 0.3.2, CLI 0.3.2
- Plan baseline: `plans/071-self-describing-framed-carrier-convenience-api.md`

## API inventory

- `frame::{encode, decode, decode_prefix}` encode and validate the existing
  11-byte-header frame format with `MAX_FRAME_PAYLOAD` bounded at 16 MiB.
- `lsb::{capacity, embed, extract, LsbConfig}` expose raw corrected-V2 pixel
  carrier operations; raw extraction requires the caller-known byte length.
- `jpeg::{capacity, embed, extract, JpegConfig, probe_support}` expose raw
  supported-JPEG DCT operations; raw extraction requires byte length and the
  embedding redundancy actually used.
- `EmbedReport<T>` contains output, embedded status, payload byte count,
  capacity values, and `actual_redundancy`.
- `CapacityReport` contains required and available carrier units and exposes
  `is_sufficient()`.
- `StegoError` already contains `FrameNotFound`, `MalformedFrame`,
  `FrameChecksumMismatch`, `InsufficientCapacity`, `UnsupportedJpeg`,
  `InvalidConfig`, and `MalformedInput`; no new variant is expected.

## Status rows

| ID | Status | Evidence |
|---|---|---|
| R01 | CLOSED | `stegoeggo-stego/src/lsb.rs::embed_framed` encodes with `frame::encode` and delegates to raw LSB embedding |
| R02 | CLOSED | `stegoeggo-stego/src/lsb.rs::extract_framed` extracts and validates the fixed prefix before the full frame |
| R03 | CLOSED | `stegoeggo-stego/src/jpeg.rs::embed_framed` encodes with `frame::encode` and delegates to raw JPEG embedding |
| R04 | CLOSED | `stegoeggo-stego/src/jpeg.rs::extract_framed` recovers length from `frame::decode_prefix` |
| R05 | CLOSED | JPEG framed extraction tries configured redundancy down through 1 without `EmbedReport` state |
| R06 | CLOSED | Both framed extractors validate frame bounds and carrier capacity before full extraction |
| R07 | CLOSED | Frame decoder errors and CRC validation are reused; malformed/oversized prefix and capacity tests pass |
| R08 | CLOSED | Public LSB/JPEG wrong-seed tests fail deterministically without unbounded probing |
| R09 | CLOSED | Raw carrier functions and their existing tests remain unchanged |
| R10 | CLOSED | No frame constants, serialization, or decode semantics changed |
| R11 | CLOSED | Carrier framed functions import only the generic frame module; no rights/application imports were added |
| R12 | CLOSED | Carrier README, root README, and `examples/generic_stego.rs` distinguish raw and framed recovery |
| R13 | CLOSED | 36 public API tests, 92 carrier tests, and 17 carrier doctests pass |
| R14 | CLOSED | `./scripts/check.sh` passes formatting, clippy, minimal-feature check, workspace tests, and doctests |

## Change log

- Added public LSB and JPEG framed convenience operations as composition over
  the existing generic frame and raw carrier APIs.
- Added bounded prefix-first extraction, capacity preflight, and candidate
  validation coverage for malformed, oversized, wrong-seed, and downgraded
  redundancy cases.
- Updated carrier/root documentation, architecture descriptions, the generic
  example, and the StegoEggo conventions skill.

## Verification log

- `cargo test --test public_stego_api --all-features` — PASS (36 passed)
- `cargo test -p stegoeggo-stego --all-features` — PASS (106 passed across focused run; 92 in full workspace run)
- `cargo test --example generic_stego --all-features` — PASS (compilation/test target)
- `cargo fmt --all -- --check` — PASS
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` — PASS
- `cargo check -p stegoeggo --no-default-features` — PASS
- `./scripts/check.sh` — PASS (543 root unit tests, all workspace integration suites, 92 carrier unit tests, 24 root doctests, 17 carrier doctests)
