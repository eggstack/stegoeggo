# Plan 097 Status

Status: IMPLEMENTED (pending final integrated verification)

Baseline: `27bdd3d429d663948021de43d3e6f818fa613319`
Depends on: Plans 091-096.

## Required evidence

- [x] direct `stegoeggo-stego` external consumer covers final recommended surface
- [x] root-facade consumer parity verified if facade retained
- [x] public API changes classified for semver compatibility
- [x] `cargo-semver-checks` evidence recorded where practical
- [x] README/rustdoc/STABILITY/DEPRECATIONS/SUPPORT/CHANGELOG/architecture docs reconciled
- [x] generic examples prefer direct carrier package
- [x] exact F5 variant/capacity/progressive/raw-tiled/authentication limits documented
- [x] release-cadence lockstep/independent decision recorded
- [x] carrier dependency tree remains application-neutral
- [x] `image` optionality disposition recorded rather than changed gratuitously
- [x] Rust 1.87 and supported platform evidence current
- [x] GitHub required-check/support-documentation discrepancy resolved truthfully
- [x] `cargo package -p stegoeggo-stego --allow-dirty --list` reviewed
- [x] final focused public/known-answer/reuse/buffer/parent compatibility suites pass
- [x] final integrated `./scripts/check.sh` passes (local full gate green, 2026-09-10)

## Implementation notes

- Consumer fixture: `stegoeggo-stego/tests/direct_consumer.rs` (8 tests,
  default features, no `application-support`, no parent crate) covers
  validated LSB raw/framed/in-place/tiled, packed + strided views with
  padding/alpha preservation, JPEG probing, strict + best-effort JPEG,
  framed/tiled JPEG extraction, prepared-JPEG parity reuse, transactional
  seed hint including a public-level insufficient-hint failure (via DQT
  byte surgery flattening tables to 1), unsupported-JPEG contract, and
  structured errors without exhaustive matching.
- Root facade: `stegoeggo::stego` re-exports `pixels`/`prepared` modules
  plus `Redundancy`/`PreparedJpeg`/`PixelView`/`PixelViewMut`/
  `PixelLayout`; no carrier API exists only through the facade. Docs name
  `stegoeggo-stego` canonical for generic-only use.
- Semver: `cargo-semver-checks 0.50.0` (installed for evidence only, not a
  dependency) against the audited-baseline worktree: carrier
  `0.4.0 -> 0.4.0`, 196 checks pass, 0 fail; root `stegoeggo`
  `0.4.0 -> 0.4.0`, 196 pass, 0 fail. CLI is binary-only (nothing to
  check). Classification: all changes additive (new functions, types,
  methods, modules, re-exports) plus correctness fixes with no
  valid-success regression (partial hints never round-tripped; payload
  bytes unchanged wherever the full hint fits). No removal, rename,
  public-field change, or default change.
- Docs reconciled: carrier README (strict/best-effort, prepared, views,
  exact units, F5 variant, `Redundancy`, API table);
  `docs/carrier-crate.md` (strict/prepared/views/units/config);
  `docs/rust-api.md` (example inventory); root README untouched (only
  indexes `docs/carrier-crate.md`); `STABILITY.md` (carrier surface incl.
  new items + best-effort/outcome compatibility notes);
  `DEPRECATIONS.md` (new compatibility-surface table, no new
  attribute-deprecations); `SUPPORT.md` (CI described as standard gate,
  not enforced required check); `CHANGELOG.md` (Unreleased
  Added/Fixed/Changed); `architecture/protected-steganography.md`
  (V2 mapping language, module/API tables, capacity units, strict policy,
  prepared superseding `PRIVATE-REUSE-SUFFICIENT`, crate layout);
  `architecture/overview.md` (new modules); `AGENTS.md` gotchas
  (bijection, core files, strict/best-effort, `Redundancy`, views,
  prepared, hint preflight); `.skills/stegoeggo-conventions/SKILL.md`
  (module layout, full carrier signatures, config guidance, pitfalls).
- Required documentation statements: direct-crate canonical status (carrier
  README, `docs/carrier-crate.md`, facade docs); parent ownership of
  rights/provenance/fallback/verification (carrier README relationship
  section); CRC32 corruption-detection boundary (README, carrier-crate,
  rustdoc); seed not a secret (README security section); LSB lossless-only
  survival (README, WebP caveat); JPEG subset + progressive/restart limits
  (README, `probe_support` docs); F5-style/no-zero-coefficient variant
  with no interop claim (`stego_f5.rs`, README, CHANGELOG); `|coef| >= 2`
  units (README, rustdoc, error docs); raw-tiled no-auth vs framed-tiled
  CRC (README, `extract_tiled` docs); prepared exposes operations not
  internals (`prepared.rs`, README); view layout/stride/alpha/padding
  exactness (`pixels.rs`, README); V2 frozen mapping + documented-domain
  injectivity (README, rustdoc, architecture, AGENTS, skill).
- Examples: `examples/generic_stego.rs` imports `stegoeggo_stego`
  directly (framed view on caller-owned buffer, `Redundancy` runtime
  config, explicit best-effort JPEG, strict embed + prepared reread);
  prose-level auth guidance stays in README rather than a home-grown
  crypto example. Examples keep compiling under `check.sh`.
- Release cadence: lockstep retained. The root depends on `=0.4.0` plus
  hidden `application-support` whose surface only makes sense in lockstep;
  external consumers use stable additive APIs, so independent carrier
  bumps buy no compatibility advantage against manual-release simplicity.
  No publish performed by this plan.
- Dependencies: `cargo tree -p stegoeggo-stego --edges normal` shows only
  `image`, `jpeg-encoder`, `crc32fast`, `thiserror` (+ transitive codec
  libs) — no rights/provenance/HMAC/signature/metadata leakage.
- `image` optionality: retained as mandatory. The public `RgbaImage`
  compatibility API must stay available through 0.x; views decouple the
  algorithm without bifurcating the public surface. Revisited only with a
  separately approved dependency redesign.
- MSRV/platform: `cargo +1.87.0 check -p stegoeggo-stego --locked` clean
  (5 dead-code warnings identical to baseline — feature-off helpers);
  `cargo +1.87.0 test -p stegoeggo-stego --locked` green (168 lib + 8
  consumer + 33 doctests). Full platform matrix remains scheduled
  Assurance (unchanged by this roadmap).
- CI governance: `gh api .../branches/main/protection` returns 404
  Branch not protected — the Roadmap 090 audit was correct.
  `SUPPORT.md` no longer claims required-check enforcement; the `Check`
  job is documented as the standard push/PR gate. No hosting change made.
- Package: `cargo package -p stegoeggo-stego --allow-dirty --list`
  contains `src/**/*.rs` (incl. new `pixels.rs`/`prepared.rs`), `Cargo.toml`,
  `README.md`, `LICENSE` — no workspace-only files.
- Focused suites: carrier lib + doctests + direct consumer, workspace
  `public_stego_api`, progressive fallback suites, tiled recovery,
  decode-count, and parent verification-convergence suites green (full
  matrix via `./scripts/check.sh` below).

Record final `./scripts/check.sh` result and implementation commit SHA here
during closure. No crates.io publish is authorized by this plan alone.
