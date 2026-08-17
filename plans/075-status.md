# Plan 075 Status

Implementation ledger for [Plan 075](075-jpeg-framed-extraction-and-evidence-corrective-closure.md).

## Baseline

- Starting HEAD: `5d051b3` (`plans: add stego corrective closure pass`)
- Working tree: clean on `main`, tracking `origin/main`
- Workspace versions: root `stegoeggo` 0.3.2, carrier `stegoeggo-stego` 0.3.2, CLI 0.3.2
- Root carrier dependency: `stegoeggo-stego = { path = "stegoeggo-stego", version = "=0.3.2", features = ["application-support"] }`
- Roadmap 069 header: `Status: COMPLETE` before this corrective pass
- Present status ledgers: `070-status.md`, `071-status.md`, `072-status.md`, `073-status.md`, `074-status.md`; `073-status.md` was present only as an ignored/untracked working-tree file at baseline
- `plans/073-status.md` is absent from the audited Git baseline even though a copy is present in this checkout; its chronology and source evidence are reconciled by Plan 075 rather than assumed correct
- GitHub CI for `09eba2a`: no run was returned by `gh run list --commit 09eba2a`; availability could not be established
- Implementation commit: `8a6bd6f247824b54f5722bd9f315d8c41d22b03b`

## Final status rows

All rows were opened before product-source edits and closed after implementation and local verification.

| ID | Status | Evidence |
|---|---|---|
| R01 | CLOSED | Roadmap 069 was `PARTIAL` during implementation and is restored to `COMPLETE` only after this ledger closure |
| R02 | CLOSED | Plan 073 is now force-tracked with explicit retrospective chronology language; no historical timestamp was fabricated |
| R03 | CLOSED | Plan 073 rows are source-backed against `8fd0153`, including fallible configs, shared validation, docs, doctests, public boundary, dependencies, tests, and no release/CI change |
| R04 | CLOSED | Private `DecodedJpegCarrier` retains coefficients; the test-only thread-local decode counter records one decode for redundancy 1, 3, auto-downgrade, and wrong-seed redundancy 10 cases |
| R05 | CLOSED | `extract_framed` still iterates `(1..=config.redundancy()).rev()` |
| R06 | CLOSED | Prefix extraction, full extraction, capacity checks, and CRC validation use the same retained private coefficients |
| R07 | CLOSED | `FramedFailure` ranks full-frame errors over prefix errors over capacity-only results |
| R08 | CLOSED | The first recorded full-frame error is retained and cannot be overwritten by later prefix-only noise; helper tests cover the precedence |
| R09 | CLOSED | `checked_payload_bits` and `checked_required_capacity` guard public JPEG arithmetic in capacity, embed, extract, and framed paths |
| R10 | CLOSED | Raw `jpeg::extract` calls shared redundancy validation before private F5 construction; 0, 11, and `usize::MAX` return `InvalidConfig` |
| R11 | CLOSED | Public raw JPEG roundtrips pass for redundancies 1, 3, and 10; existing supported/unsupported behavior remains passing |
| R12 | CLOSED | Public and carrier tests recover framed JPEG payloads after automatic redundancy downgrade |
| R13 | CLOSED | Wrong-seed framed extraction remains bounded and non-panicking; malformed/overflow input tests pass |
| R14 | CLOSED | No JPEG parser, coefficient, session, or F5 type was made public; compile-fail boundary doctests pass |
| R15 | CLOSED | No public session or carrier framework API was added |
| R16 | CLOSED | `lsb_clone_vs_in_place` uses Criterion `iter_batched` with a fresh pristine source for each in-place iteration |
| R17 | CLOSED | The preparation clone is in the Criterion batch setup, outside the timed in-place closure; manual benchmark completed for 1024 and 4096 sizes |
| R18 | CLOSED | Root rustdoc, README, carrier README, and architecture wording now describe the carrier package/API boundary without claiming an independent release cadence |
| R19 | CLOSED | Carrier tests: 126 passed; carrier doctests: 21 passed; public API tests: 45 passed; focused JPEG and tiled-JPEG filters passed |
| R20 | CLOSED | `./scripts/check.sh` passed, including fmt, strict clippy, minimal-feature check, full workspace tests, and doctests |
| R21 | CLOSED | No dependency, version, publication, tag, release, required-CI, or workflow expansion was made |
| R22 | CLOSED | Roadmap 069 now references Plan 075 as the final corrective pass and is restored to `COMPLETE` after this evidence update |

## Change log

- Created before product-source edits, as required by Plan 075 Phase 0.
- Reopened Roadmap 069 as `PARTIAL` while corrective work is active.
- Implementation landed in `8a6bd6f`; this closure reconciliation records the exact implementation commit and restores the roadmap only after all rows closed.

## Verification log

- `cargo fmt --all -- --check` — PASS
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` — PASS
- `cargo check -p stegoeggo --no-default-features` — PASS
- `cargo test -p stegoeggo-stego --all-features` — PASS (126)
- `cargo test -p stegoeggo-stego --doc` — PASS (21)
- `cargo test -p stegoeggo --test public_stego_api --all-features` — PASS (45)
- `cargo test -p stegoeggo --all-features jpeg` — PASS (139 passed, 4 ignored)
- `cargo test -p stegoeggo --all-features tiled_jpeg` — PASS (2)
- `cargo test --workspace --exclude stegoeggo-fuzz --all-features` — PASS
- `./scripts/check.sh` — PASS
- `cargo bench --bench bench lsb_clone_vs_in_place` — PASS; corrected fresh-state measurements completed for 1024 and 4096 images
- `./scripts/release-check.sh --allow-dirty --skip-check --stage=pre` — PASS; lockstep and structural package checks passed
