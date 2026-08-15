# Plan 067 Status Ledger

Status: COMPLETE — tiled-JPEG integrity and final evidence closure verified

Roadmap: `plans/057-stego-carrier-library-and-pipeline-simplification-roadmap.md`

Plan: `plans/067-tiled-jpeg-candidate-integrity-and-final-evidence-closure.md`

## Baseline

- starting HEAD: `29fa62f (plans: add tiled JPEG and final evidence corrective pass)`
- working tree status: clean before this ledger
- workspace members: `.`, `stegoeggo-stego`, `stegoeggo-cli`, `fuzz`
- root version: `0.3.2`
- carrier version: `0.3.2`
- CLI version: `0.3.2`
- root -> carrier dependency: `stegoeggo-stego = { path = "stegoeggo-stego", version = "=0.3.2", features = ["application-support"] }`
- Roadmap 057 status at start: `COMPLETE — final residuals closed by Plan 066`
- Plan 066 status at start: `COMPLETE — final closure and public-boundary correction verified`

## Compatibility contract

Tiled-JPEG extraction is bounded by admitted tile origins, the existing nearby
grid-seed neighborhood (`0..=2` in each coordinate), and redundancy values
`1..=10`. A candidate identity contains the tile origin, grid-seed coordinates,
and redundancy. Prefix, header, full-payload, and legacy extraction lengths must
all use the same identity. Invalid, `NotV3`, malformed, unsupported-version,
authentication-failed, and integrity-failed candidates are skipped so later
bounded candidates can succeed. Verification records useful failure
classification only after candidate exhaustion. `max_origins` limits tile
origins, not the candidate variants within an admitted origin.

## Status rows

All rows begin `OPEN` and will be closed with concrete implementation, test,
audit, or documentation evidence.

| ID | Row | Status | Evidence |
|---|---|---|---|
| R01 | tiled JPEG candidate identity preserved across prefix/header/full extraction | CLOSED | `TiledJpegCandidateKey`; root `evaluate_tiled_candidates`; carrier identity round-trip test |
| R02 | wrong early NotV3 candidate does not terminate later V3 search | CLOSED | carrier and root wrong-first/later-valid regressions |
| R03 | malformed/unsupported/auth-failed candidate does not mask later valid candidate | CLOSED | shared evaluator continues candidates and preserves first failure only after exhaustion; wrong-MAC regression |
| R04 | legacy tiled-JPEG candidate probing remains bounded and compatible | CLOSED | exact-key legacy lengths remain `[ECC_PAYLOAD_BITS_V2, ECC_PAYLOAD_BITS]`; full workspace tests |
| R05 | max_origins semantics remain bounded at tile-origin level | CLOSED | carrier max-origin regression and existing root max-origin tests |
| R06 | tiled JPEG Embedded outcome survives post-encode extraction | CLOSED | carrier encodes once, decodes output, and verifies recorded tile/payload |
| R07 | failed tiled JPEG post-encode verification is not reported Embedded | CLOSED | private post-encode outcome helper and failure regression |
| R08 | carrier internals remain inaccessible from default public API | CLOSED | private modules, compile-fail boundary, carrier doctests |
| R09 | root application uses only operation-level carrier support | CLOSED | root references only `application_support` operations and opaque key |
| R10 | production request_from_legacy is directly tested in crate-local tests | CLOSED | ten `request_from_legacy_*` tests in `src/lib.rs` |
| R11 | duplicated integration-test legacy request builder removed or reduced to public-behavior-only use | CLOSED | `plan065_compat_helpers` removed; integration suite uses public APIs |
| R12 | Plan 063 public API inventory matches current source | CLOSED | stale `MIN_TILE_SIZE`/`tile_seed()` default-API entries removed and correction banner added |
| R13 | Plan 065 implementation commits/evidence are reconciled | CLOSED | `70845eb` and `1c530a7` recorded with later-plan ownership note |
| R14 | Plan 066 ledger chronology is corrected truthfully | CLOSED | chronology correction and residual disposition added to `plans/066-status.md` |
| R15 | Plan 064 final supersession note points to Plan 067 | CLOSED | final supersession note added without rewriting historical evidence |
| R16 | Roadmap 057 is PARTIAL while Plan 067 is open | CLOSED | roadmap reopened before implementation; final status changed only after checks |
| R17 | focused tiled-JPEG regression matrix passes | CLOSED | carrier `tiled_jpeg` and root `tiled_jpeg` tests pass |
| R18 | focused legacy-adapter matrix passes | CLOSED | direct adapter tests and `plan065_legacy_compat` pass |
| R19 | carrier boundary/doctest checks pass | CLOSED | carrier unit tests and `cargo test -p stegoeggo-stego --doc` pass |
| R20 | ./scripts/check.sh passes | CLOSED | fmt, strict clippy, minimal check, workspace tests, and doctests pass |
| R21 | staged pre-release structural check passes | CLOSED | `release-check.sh --allow-dirty --skip-check --stage=pre` passes |
| R22 | final Roadmap 057 evidence is internally consistent | CLOSED | source audit, documentation reconciliation, and all required checks pass |

## Evidence log

Implementation and verification evidence:

- The ledger was force-tracked before product-source edits: `git ls-files
  --error-unmatch plans/067-status.md` succeeded.
- Targeted checks passed: carrier tiled-JPEG tests, root tiled-JPEG tests,
  direct `request_from_legacy` tests, `plan065_legacy_compat`, carrier
  boundary/doctests, and strict clippy.
- `./scripts/check.sh` passed: formatting, strict workspace clippy,
  `cargo check -p stegoeggo --no-default-features`, and all workspace tests
  excluding fuzz. The final run reported 543 root unit tests, 88 carrier unit
  tests, all integration suites, and zero failures (external-tool suites stayed
  ignored as configured).
- `./scripts/release-check.sh --allow-dirty --skip-check --stage=pre` passed,
  including carrier package verification and root/CLI structural package checks.
- Source audit found no old flattened tiled-JPEG operation, no duplicated
  Plan-specific request builder, no default-public carrier internals, and no
  root import of JPEG parser/coefficient/F5 types.
