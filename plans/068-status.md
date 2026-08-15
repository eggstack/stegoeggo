# Plan 068 Status Ledger

Status: COMPLETE — tiled-JPEG single-decode search context verified

Roadmap: `plans/057-stego-carrier-library-and-pipeline-simplification-roadmap.md`

Plan: `plans/068-tiled-jpeg-single-decode-search-context-corrective-closure.md`

## Baseline

- starting HEAD: `46fc0145547e6cdebd21e104b34994c796771412`
- working tree status: clean on `main`, aligned with `origin/main`
- workspace members: `.`, `stegoeggo-stego`, `stegoeggo-cli`, `fuzz`
- root version: `0.3.2`
- carrier version: `0.3.2`
- CLI version: `0.3.2`
- root -> carrier dependency: `stegoeggo-stego = { path = "stegoeggo-stego", version = "=0.3.2", features = ["application-support"] }`
- Roadmap 057 status at start: `COMPLETE — final tiled-JPEG and evidence residuals closed by Plan 067`
- Plan 067 status at start: `COMPLETE — tiled-JPEG integrity and final evidence closure verified`

## Runtime contract

For one root tiled-JPEG extraction or verification search over one encoded JPEG,
full coefficient decoding must occur at most once. Prefix enumeration, exact-key
V3 header extraction, exact-key V3 full extraction, and exact-key V1/V2 fallback
must all reuse one operation-local carrier-owned decoded search context.

Search coverage remains unchanged: admitted tile origins are bounded by
`max_origins`; nearby seed coordinates remain `0..=2` in both axes; and
redundancy remains `1..=10`. Candidate identity remains the exact opaque
tile-origin/grid-seed/redundancy key selected during prefix enumeration.

## Status rows

All rows begin `OPEN` and will be closed only with concrete implementation,
test, audit, or documentation evidence.

| ID | Row | Status | Evidence |
|---|---|---|---|
| R01 | one carrier-owned decoded search context per tiled-JPEG search | CLOSED | `TiledJpegSearch` owns private header/coefficient state; both root tiled entry paths construct one context per call |
| R02 | prefix enumeration uses the retained decoded context | CLOSED | `TiledJpegSearch::prefix_candidates` extracts all bounded prefixes from `self.coefficients` |
| R03 | exact-candidate header extraction uses the retained decoded context | CLOSED | Root evaluator calls `search.extract_candidate` for the V3 header length |
| R04 | exact-candidate full/legacy extraction uses the retained decoded context | CLOSED | The same context serves V3 full extraction and both legacy lengths |
| R05 | no per-candidate coefficient re-decode in normal tiled search | CLOSED | Source audit found tiled-search decoding only in `TiledJpegSearch::new`; candidate methods contain no decode call |
| R06 | candidate identity semantics from Plan 067 preserved | CLOSED | `TiledJpegCandidateKey` remains tile origin + grid seed + redundancy and is reused through evaluation |
| R07 | wrong-first/later-valid regression preserved | CLOSED | Carrier and root wrong-first/later-valid regressions pass; carrier test also asserts one search decode |
| R08 | V3 authentication/failure classification preserved | CLOSED | Shared `evaluate_tiled_candidates` remains unchanged in continuation/classification behavior; full workspace tests pass |
| R09 | legacy V1/V2 candidate fallback preserved | CLOSED | Root still probes `[ECC_PAYLOAD_BITS_V2, ECC_PAYLOAD_BITS]`; dedicated carrier decode-count test covers both lengths |
| R10 | max_origins still bounds tile origins, not candidate variants | CLOSED | Prefix loop retains origin counter semantics; existing max-origin carrier/root tests pass |
| R11 | nearby seed range remains 0..=2 and redundancy remains 1..=10 | CLOSED | Context prefix loop retains both bounded ranges; identity and redundancy tests pass |
| R12 | carrier codec/coefficient internals remain private | CLOSED | Context fields are private; `jpeg_transcoder`, coefficient, and F5 types remain crate-private; boundary doctests pass |
| R13 | default public carrier API unchanged | CLOSED | No default-feature facade changes; public carrier API and doctests pass |
| R14 | root uses only operation-level application-support type(s) | CLOSED | Root references only `TiledJpegSearch` and `TiledJpegCandidateKey`, never JPEG parser/coefficient/F5 types |
| R15 | decode-count/instrumentation regression proves bounded single decode per search | CLOSED | Multiple-candidate, wrong-first, broad no-match, and legacy fallback tests all assert decode count `== 1` |
| R16 | focused tiled-JPEG carrier tests pass | CLOSED | `cargo test -p stegoeggo-stego tiled_jpeg --features application-support`: 12 passed |
| R17 | focused root tiled-JPEG tests pass | CLOSED | `cargo test -p stegoeggo tiled_jpeg --all-features`: 2 passed |
| R18 | carrier boundary/doctests pass | CLOSED | Full carrier suite: 106 passed; `cargo test -p stegoeggo-stego --doc`: 14 passed |
| R19 | `./scripts/check.sh` passes | CLOSED | Format, strict clippy, minimal-feature check, all workspace tests, and doctests passed; root 543 unit tests and carrier 92 unit tests passed |
| R20 | staged pre-release structural check passes | CLOSED | `./scripts/release-check.sh --allow-dirty --skip-check --stage=pre` passed |
| R21 | Plan 067 residual note reconciled truthfully | CLOSED | Plan 067 retains its correctness closure and now records the later repeated-decode residual as Plan 068 scope |
| R22 | Roadmap 057 final closure is evidence-consistent | CLOSED | Roadmap is COMPLETE only after all rows and required checks closed |

## Evidence log

Implementation and verification evidence:

- Phase 0 ledger was force-tracked before product-source edits with
  `git add -f plans/068-status.md` and `git ls-files --error-unmatch
  plans/068-status.md`.
- The carrier support boundary now exposes only an opaque operation-local
  `TiledJpegSearch`; the default `stegoeggo-stego` API was not expanded.
- The focused carrier suite passed with 12 tiled-JPEG tests, including
  single-decode instrumentation for multiple candidates, wrong-first/later-
  valid search, broad no-match search, both legacy lengths, and incompatible
  geometry-key rejection. The root tiled-JPEG suite passed with 2 tests.
- `./scripts/check.sh` passed. The all-feature workspace run reported 543 root
  unit tests, 92 carrier unit tests, all non-ignored integration suites, and no
  failures. External-tool suites remained ignored as configured.
- `./scripts/release-check.sh --allow-dirty --skip-check --stage=pre` passed,
  including version lockstep, carrier packaging, and root/CLI structural
  package validation. Standalone carrier packaging emitted only non-fatal
  dead-code warnings for parent-adapter-only helpers.

## Final source audit

1. Full tiled-search JPEG coefficient decoding occurs only in
   `TiledJpegSearch::new`.
2. `prefix_candidates()` cannot trigger another full decode.
3. `extract_candidate()` cannot trigger another full decode.
4. V3 header extraction cannot trigger another full decode.
5. V3 full extraction cannot trigger another full decode.
6. Legacy V2/V1 fallback cannot trigger another full decode.
7. Each root tiled extraction/verification entry call creates one search
   context; no context is created inside the shared evaluator or candidate
   extraction method.
8. Candidate identity still contains tile origin, nearby grid-seed coordinates,
   and redundancy.
9. JPEG parser, coefficient, Huffman, entropy, DCT block, and F5 types remain
   private to `stegoeggo-stego`.
10. The default carrier API is unchanged.
11. Search bounds are unchanged: `max_origins`, nearby coordinates `0..=2`,
    and redundancy `1..=10`.
12. No package version, release, or CI behavior changed.

## Final commit/evidence record

- implementation commit: the final Plan 068 closure commit on `main`
- final HEAD and remote CI result: recorded after commit and push in the
  delivery log; no publication, tag, release, or CI expansion was performed
