# Plan 087 Status

Status: IMPLEMENTED

Baseline: `07bd05304a6942a843a76c28013a5bc0f08179cc`
Head before work: `6660839 plan 086: record CI run evidence in status ledger`
Dependency: Plan 082 (IMPLEMENTED, `88fe30d`)

## Required evidence

- [x] one canonical CLI `ProtectionRequest` builder/path
- [x] modern/legacy precedence table implemented and tested
- [x] contradictory combinations fail consistently
- [x] single/batch/dry-run paths share request construction
- [x] CLI source decomposed into cohesive private modules
- [x] production dependency/root-feature call-site audit recorded
- [x] unused/redundant features/dependencies removed or justified
- [x] default and `signatures` builds/package checks pass
- [x] v1 removal/migration inventory documented
- [x] `./scripts/check.sh` passes

## Canonical builder

One path: `request::build_protection_request_with_explicit_options()` in
`stegoeggo-cli/src/request.rs`, called once in `main.rs::run()` with clap
`ValueSource` explicitness for `--level`/`--profile`. The resulting
`ProtectionRequest` is reused by single-file, sequential batch, parallel batch
(`rayon`), dry-run (`resolve_request`), and JSON (`process_request_bytes_with_report`)
paths. No duplication between modes.

## Precedence contract

Implemented in `request.rs`, documented in `docs/cli-usage.md` and
`architecture/cli.md`:

1. Explicit modern wins when directly specifying a field.
2. Legacy translates only when the equivalent modern field was not explicitly supplied.
3. Contradictory explicit combinations fail with `EXIT_CONFIG` (exit 2).
4. Defaults apply once, after explicitness is known.

New tightening vs baseline (bug fixes, regression-tested):
- Explicit `--hidden-marker`/`--authentication` with explicit `--level`/`--profile`
  now fails (was silently ignoring legacy). Unit: `test_hidden_marker_and_explicit_level_conflict`,
  `test_authentication_and_explicit_profile_conflict`. Integration:
  `test_hidden_marker_with_level_explicit_is_config_error`.
- `--metadata false` with a metadata-injecting preset/channels now fails (was
  silently ignored in new-style path). Unit: `test_metadata_false_with_preset_is_config_error`.
  Integration: `test_metadata_false_with_preset_is_config_error`.
- `legal-claims` now forces `rights_metadata=true` in new-style path (was ignored).

Retained: `--preset` + `--level`/`--profile` fails; `--preset` +
`--hidden-marker`/`--authentication` fails; `--dmi`/`--rights-policy`/shorthand
contradictions fail; HMAC-without-key and HMAC-with-disabled-marker fail;
omitted `--dmi` == `--dmi auto`; explicit `--dmi unspecified` distinct from default.

## Module layout

`stegoeggo-cli/src/` (was single `main.rs` 2646 lines, now 2828 lines across 8 files):

| File | Lines | Responsibility |
|---|---|---|
| `args.rs` | 394 | clap structures/value enums only |
| `request.rs` | 920 | canonical request construction/validation (+36 unit tests) |
| `protect.rs` | 302 | single/batch orchestration, output paths, atomic writes (+3 unit tests) |
| `verify.rs` | 149 | image verification and rendering |
| `output.rs` | 106 | human/JSON types and exit-code mapping |
| `keys.rs` | 77 | key input parsing (+2 unit tests) |
| `manifest.rs` | 428 | feature-gated signing/detached-manifest (`signatures`; +2 unit tests) |
| `main.rs` | 452 | thin orchestration, no generic command framework |

Single binary and command-line contract preserved.

## Dependency audit

Before: `cargo tree -p stegoeggo-cli --edges normal` = 155 lines.
After: 119 lines (-36).

| Dependency | Call sites | Verdict |
|---|---|---|
| root `iscc` | none (only `Error::Iscc` exit-code arm) | REMOVED (was pulling `iscc-lib`/`blake3`/`unicode-general-category`/`xxhash`) |
| root `conformance` | none (`stegoeggo-conformance` belongs to root package) | REMOVED (was pulling `toml`/`winnow`/`unicode-normalization`) |
| root `parallel` | none (CLI never calls `process_request_bytes_parallel`; root batch fails fast, CLI batch needs per-file error tolerance) | REMOVED; direct `rayon` RETAINED with justification |
| direct `image` | one verbose-only `load_from_memory` dimension log | REMOVED from production (tests retain via dev-deps); verbose block deleted |
| direct `rayon` | batch `par_iter` with `jobs>1` | RETAINED (error-tolerant batch unsuitable via root parallel) |
| direct `hex` | key parsing + manifest display | RETAINED |
| direct `serde`/`serde_json` | `--json` structs + `verify-manifest --json` | RETAINED |
| direct `tempfile` | atomic writes (`NamedTempFile`) + tests (`tempdir`) | RETAINED |
| direct `clap` | arg parsing | RETAINED |

CLI `stegoeggo` dep is now default-features only. `signatures` feature still enables
`stegoeggo/signatures` + `stegoeggo/detached-manifest`.

Builds verified: `cargo build -p stegoeggo-cli` and
`cargo build -p stegoeggo-cli --features signatures` both succeed.
`cargo tree` before/after captured in this ledger (see diff below).

Before/after tree diff (normal edges):

```
26,47d25 (direct image block removed; image now only transitive via stegoeggo)
83,106c62,82 (iscc-lib + blake3 + unicode-general-category + xxhash removed)
110d85 (duplicate rayon marker resolved)
129,139d103 (toml + winnow + unicode-normalization removed)
```

## Tests

- Unit (bins): 42 passed (`cargo test -p stegoeggo-cli --bins`), including 4 new
  precedence tests.
- Integration (`tests/cli.rs`): 65 passed (was 57; +8 new):
  `test_batch_parallel_jobs_produce_same_outputs`,
  `test_verify_json_schema`,
  `test_explicit_hidden_marker_best_effort_with_policy`,
  `test_hidden_marker_with_level_explicit_is_config_error`,
  `test_metadata_false_with_preset_is_config_error`,
  `test_key_from_env_var`, `test_key_from_stdin`,
  `test_authenticated_preset_with_key_reports_hmac`.
- Detached (`tests/cli_detached_verification.rs`, `--all-features`): 16 passed.
- Full `./scripts/check.sh`: pass (fmt, clippy `-D warnings`, no-default-features
  check, workspace `--all-features` tests including all above).

Tiled CLI: not exposed (no `--tile-size` flag; tiling is library `ProtectionRequest`
/ `TileConfig` only). Covered by library tiled tests, not CLI. Documented as
out-of-scope in `docs/cli-usage.md` v1 inventory context.

## Docs

- `docs/cli-usage.md`: precedence table + v1 removal inventory.
- `DEPRECATIONS.md`: CLI v1 removal inventory (deprecated syntax / compat behavior /
  stable syntax).
- `architecture/cli.md`: module table, audited dependency table with removals/
  retentions, full precedence table, tightened conflict rules; pruned stale
  `--known-seeds` flag and `lib.rs`/`types.rs` module references.
- `STABILITY.md`: CLI request-contract note.
- `SUPPORT.md`: CLI default-features note.
- `CHANGELOG.md`: Unreleased Changed entries for Plan 087.
- `AGENTS.md`: workspace module list, CLI default-features correction (was claiming
  `iscc`/`conformance`/`parallel`), tightened precedence gotchas, binary location retained.
- `README.md`: no change needed (examples already policy-first; feature table is
  library-scoped).

## Notes

No 0.x flag/API removed. All changes are additive tightening (new config errors
for previously silent weakening) plus decomposition and dependency trim.
