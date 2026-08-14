# Plan 066 Status Ledger

Status: COMPLETE — final closure and public-boundary correction verified

Roadmap: `plans/057-stego-carrier-library-and-pipeline-simplification-roadmap.md`

This ledger was created before Plan 066 product-source edits. It records the
actual repository baseline and the focused closure evidence for this plan.

## Baseline

- starting HEAD: `e694209e10b46cc8f13727b220467399371c9c97`
- working tree status: clean
- workspace members: `.`, `stegoeggo-stego`, `stegoeggo-cli`, `fuzz`
- root version: `0.3.2`
- carrier version: `0.3.2`
- CLI version: `0.3.2`
- root -> carrier dependency: `stegoeggo-stego = { path = "stegoeggo-stego", version = "=0.3.2" }`
- Roadmap 057 status at start: `COMPLETE — closure evidence consolidated by Plan 065`
- Plan 065 status at start: `OPEN — corrective closure`

## Status rows

Every row begins `OPEN` and is closed only with concrete implementation,
test, audit, or documentation evidence recorded below.

| ID | Row | Status | Evidence |
|---|---|---|---|
| R01 | Light + tile_size remains SeedOnly | CLOSED | `request_from_legacy()` and legacy compatibility matrix |
| R02 | Standard + tile_size remains Tiled | CLOSED | canonical request path and tiled integration tests |
| R03 | default carrier API cannot name JPEG codec/coefficient internals | CLOSED | private transcoder modules, explicit facade, compile-fail doctest |
| R04 | default carrier API cannot name low-level LSB internals | CLOSED | private LSB internals, explicit facade, compile-fail doctest |
| R05 | root application no longer consumes internal carrier structs/functions directly | CLOSED | root steganography adapter uses operation-level support API |
| R06 | root fuzz support remains functional without default internal exposure | CLOSED | bounded public JPEG inspection path and workspace tests |
| R07 | Plan-specific public compatibility test hook removed | CLOSED | production hook removed; public-builder compatibility tests pass |
| R08 | ProtectionPipeline stale state removed | CLOSED | zero-field stateless adapter and pipeline tests |
| R09 | legal-claim rustdoc matches established legacy runtime semantics | CLOSED | corrected `None`/`Some(false)`/`Some(true)` docs and tests |
| R10 | LSB pixel-output rustdoc/examples match RgbaImage API | CLOSED | carrier API docs and generic example use `RgbaImage` output |
| R11 | carrier README/API claims match actual visibility | CLOSED | README, crate docs, and public-surface audit |
| R12 | release check models carrier -> root -> CLI publication ordering truthfully | CLOSED | staged release-check flow and updated release procedure |
| R13 | carrier package full dry-run succeeds before publication | CLOSED | `release-check --stage=pre --allow-dirty --skip-check` |
| R14 | root/CLI pre-publication structural package checks succeed without false failure | CLOSED | root/CLI package content-list checks in pre stage |
| R15 | retrospective 058/059/060/061/063 ledgers are clearly labeled and current-disposition-correct | CLOSED | retrospective banners and forced tracking |
| R16 | Plan 065 status is reconciled truthfully | CLOSED | Plan 065 closure banner, final rows, and Plan 066 authority note |
| R17 | Plan 064 correction note points to final Plan 066 closure | CLOSED | final supersession note |
| R18 | Roadmap 057 is PARTIAL while Plan 066 is open | CLOSED | Roadmap now `COMPLETE — final residuals closed by Plan 066` |
| R19 | focused test/API matrix passes | CLOSED | focused compatibility, carrier, robustness, and doctest suites |
| R20 | ./scripts/check.sh passes | CLOSED | fmt, clippy, minimal check, all workspace tests, and doctests pass |
| R21 | final Roadmap 057 closure evidence is internally consistent | CLOSED | current source, package checks, tests, and ledgers agree |

## Evidence log

## Final verification evidence

- `./scripts/check.sh` passed: formatting, strict clippy, minimal-feature check,
  all workspace tests excluding fuzz, and doctests.
- `cargo test --workspace --exclude stegoeggo-fuzz --all-features` passed,
  including 533 root unit tests, 116 integration tests, 20 Plan 065 legacy
  compatibility tests, 80 carrier unit tests, and carrier compile-fail doctests.
- `./scripts/release-check.sh --allow-dirty --skip-check --stage=pre` passed.
  The pre-publication root and CLI checks validate package contents because the
  exact unpublished carrier dependency is not yet available on crates.io.
- Targeted source audit found no production Plan 065 compatibility hook, no
  removed carrier facade names in product code, no root references to private
  carrier codec types, and no stale LSB PNG-output or legal-claim documentation.
- Roadmap 057 is now complete; Plans 058–065 retain historical evidence and
  Plan 066 is the current final closure ledger.
