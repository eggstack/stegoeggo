# Plan 087: CLI Contract and Dependency Consolidation

Status: READY FOR IMPLEMENTATION

Parent roadmap: `plans/081-pre-v1-consolidation-maintainability-and-portability-roadmap.md`

Audited baseline: `07bd05304a6942a843a76c28013a5bc0f08179cc`.

Dependency: Plan 082 should land first so the CLI targets the final canonical request API shape.

## 1. Objective

Reduce CLI maintenance and dependency surface by making all protection invocations construct one canonical `ProtectionRequest`, decomposing the monolithic CLI implementation, tightening legacy/current flag precedence, and enabling only root features required by shipped commands.

The CLI may preserve legacy flags through the 0.x compatibility window, but they must be translation syntax rather than an independent behavior model.

## 2. Canonical request construction

Create one internal request-building path that owns:

- input/output format selection;
- `RightsPolicy` resolution;
- `ProtectionPreset`/`ProtectionChannels` selection;
- hidden-marker mode;
- authentication mode and key requirements;
- legal metadata fields;
- processing options such as JPEG quality, progressive mode, redundancy, max dimensions, timestamps/content hash if exposed;
- contradiction and precedence validation;
- dry-run plan rendering inputs.

Legacy flags such as `--level`, `--profile`, `--dmi`, metadata toggles, and current flags such as `--rights-policy`, `--preset`, `--hidden-marker`, `--authentication` must converge in this builder.

Use clap `ValueSource`/explicitness tracking where necessary so defaulted legacy flags do not accidentally override explicit modern flags.

Do not duplicate request construction between single-file, batch, dry-run, and JSON paths.

## 3. Precedence contract

Document and test a deterministic precedence table. General rule:

1. explicit modern request-model flags win when they directly specify a field;
2. legacy flags translate only when the equivalent modern field was not explicitly supplied;
3. contradictory explicit combinations fail with `EXIT_CONFIG` rather than silently weakening protection;
4. defaults should be applied once, after explicitness is known.

Preserve existing intentional compatibility behavior unless it conflicts with the canonical contract; any changed precedence must be documented as a bug fix and regression-tested.

## 4. Physical CLI decomposition

Split `stegoeggo-cli/src/main.rs` into private modules while preserving the single binary and command-line contract. Suggested responsibilities:

- `args.rs` — clap structures/value enums only;
- `request.rs` — canonical request construction/validation;
- `protect.rs` — single and batch processing orchestration;
- `verify.rs` — image verification and rendering;
- `output.rs` — human/JSON formatting and exit-code mapping;
- `keys.rs` — key input parsing and secret handling;
- `manifest.rs` — feature-gated signing/detached-manifest subcommands.

Exact names may differ. Avoid a generic command framework.

## 5. Dependency/feature audit

Audit `stegoeggo-cli/Cargo.toml` direct dependencies and the root feature set currently enabled by the CLI.

For each of `iscc`, `conformance`, `parallel`, direct `rayon`, `image`, `serde`, `serde_json`, `tempfile`, `hex`, and any other normal dependency:

- identify the production call sites;
- classify as required by ordinary protect/verify, required only by a subcommand, test-only, or redundant through root APIs;
- remove or feature-gate anything not required by the shipped default CLI.

In particular, verify whether the root `conformance` feature is needed by the normal `stegoeggo` binary. The separate `stegoeggo-conformance` binary belongs to the root package and should not force conformance dependencies into `cargo install stegoeggo-cli` unless CLI code actually uses them.

Avoid replacing a small direct dependency with a larger root public API solely for dependency-count aesthetics; use measured compile/tree evidence.

## 6. Tests

Add/retain CLI integration tests for:

- default invocation;
- explicit modern preset/policy/channel combinations;
- legacy equivalent combinations;
- modern-vs-legacy precedence;
- contradictory flags;
- metadata-only, seed-only, best-effort, tiled, authenticated behavior;
- batch processing and job count;
- dry-run resolved plan;
- JSON output schemas;
- exit codes;
- key input modes;
- signature subcommands under feature flag.

Add a dependency/feature evidence command to the status ledger, e.g. `cargo tree -p stegoeggo-cli --edges normal` before/after, and package/build checks with default and `signatures` features.

## 7. Documentation and v1 handoff

Update `docs/cli-usage.md`, README examples, `DEPRECATIONS.md`, `STABILITY.md`, `SUPPORT.md`, and `CHANGELOG.md`.

Add a concise v1 removal inventory identifying which legacy CLI flags/public types are candidates for removal at v1.0.0. Do not remove them in this plan.

The v1 inventory should distinguish:

- deprecated syntax with exact modern replacement;
- compatibility behavior that must remain for reading old protected images;
- stable current syntax that will carry forward.

## 8. Acceptance criteria

- all protect modes build exactly one canonical `ProtectionRequest` path;
- precedence/contradiction rules are table-tested;
- CLI source is decomposed into cohesive private modules;
- default installed CLI no longer enables unused root features or redundant production dependencies;
- `cargo install --path stegoeggo-cli` equivalent build works with default features and with `signatures` where supported;
- no 0.x promised flag/API is removed;
- docs contain a clear v1 migration/removal inventory;
- `./scripts/check.sh` passes;
- `plans/087-status.md` records before/after dependency tree and CLI contract evidence.

## 9. Non-goals

No new subcommands, binary release installers, shell completion project, config-file format, daemon mode, network service, or v1 semver-major removal itself.
