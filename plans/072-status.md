# Plan 072 Status

Implementation ledger for [Plan 072](072-lsb-in-place-and-bitstream-allocation-optimization.md).

| ID | Requirement | Status | Evidence |
|---|---|---|---|
| R01 | Deterministic corrected-LSB known-answer behavior locked | Complete | `tests/public_stego_api.rs::public_lsb_known_answer_vector` locks the mixed-bit, boundary-value, non-power-of-two 5×3 vector; the in-place test covers redundancy 3 and alpha preservation |
| R02 | Public in-place embed added without full-image clone | Complete | `lsb::embed_in_place` returns `InPlaceEmbedReport`; source audit shows no image clone in the in-place path |
| R03 | Existing embed delegates to shared in-place core | Complete | `lsb::embed` clones once, then calls `embed_lsb_v2_in_place` |
| R04 | Normal corrected embed has no intermediate bit `Vec` | Complete | V2 embedding uses direct `payload_bit` access; `bytes_to_bits` remains only for legacy extraction-compatible code |
| R05 | Normal corrected extract has no intermediate bit `Vec` | Complete | V2 extraction allocates final `Vec<u8>` output and writes bits directly |
| R06 | Bit ordering unchanged | Complete | Known-answer vector and all raw roundtrip tests pass |
| R07 | Alpha preservation unchanged | Complete | Known-answer and in-place equivalence tests assert alpha byte identity |
| R08 | Capacity and redundancy semantics unchanged | Complete | Capacity, exact-capacity, insufficient-capacity, and redundancy tests pass |
| R09 | Legacy compatibility unchanged | Complete | Full workspace tests pass, including legacy extraction and compatibility suites |
| R10 | Tiled LSB behavior unchanged | Complete | Tiled roundtrip, crop, and application tests pass; tiled mapping was retained and only bit access changed |
| R11 | Plan 071 framed APIs pass unchanged | Complete | LSB/JPEG framed carrier tests and doctests pass |
| R12 | Allocation/performance evidence recorded | Complete | Criterion results below plus source audit |
| R13 | Carrier tests and doctests pass | Complete | Carrier unit tests: 92 passed; carrier doctests: 18 passed |
| R14 | Full workspace checks pass | Complete | `./scripts/check.sh` passed locally |

## Scope decisions

- The public in-place result is `InPlaceEmbedReport`, exposed from the LSB
  module and the carrier crate root.
- The existing cloning `lsb::embed` remains the convenience API and clones
  exactly once before calling the shared V2 mutation core.
- Tiled embedding keeps its existing image-level clone and carrier mapping;
  only its payload bit access is changed to avoid a one-byte-per-bit vector.
- The parent application adapter uses the in-place path only where it already
  owns a mutable decoded RGBA image. JPEG and copy-oriented paths remain
  unchanged.

## Measurement evidence

`cargo bench --bench bench -- lsb_clone_vs_in_place --noplot` on the local
machine reported the following median estimates:

| Image | Cloning `lsb::embed` | In-place `lsb::embed_in_place` |
|---|---:|---:|
| 1024×1024 RGBA, 256-byte payload, redundancy 2 | 517.78 µs | 167.01 µs |
| 4096×4096 RGBA, 256-byte payload, redundancy 2 | 28.189 ms | 198.07 µs |

The in-place benchmark reuses its caller-owned image, so it intentionally
excludes the clone that the cloning API must perform. The implementation audit
confirms that cloning `embed` performs one explicit `img.clone()`, while
`embed_in_place` performs none; corrected V2 embedding and extraction contain
no payload-bit vectors, only the final extracted byte buffer.
