# Plan 097: Carrier v1 Packaging, Documentation, and Evidence Closure

Status: READY FOR IMPLEMENTATION

Parent roadmap: `plans/090-generic-carrier-v1-semantics-correctness-and-reuse-roadmap.md`

Depends on: Plans 091-096.

Audited baseline: `27bdd3d429d663948021de43d3e6f818fa613319`.

## 1. Objective

Close Roadmap 090 with external-consumer evidence and a truthful, maintainable package/stability story for `stegoeggo-stego`. This plan does not publish a release automatically. It proves that the resulting carrier surface is ready for an explicit release/version decision and that documentation, semver promises, examples, MSRV/platform claims, and repository enforcement match reality.

## 2. External consumer contract

Create or update compile/run fixtures that consume the package as a third party would, with no access to private workspace modules.

Required direct `stegoeggo-stego` consumer coverage:

- validated LSB raw/framed/in-place/tiled operations;
- packed and strided pixel-buffer APIs from Plan 095;
- JPEG support probing;
- strict and explicitly best-effort JPEG operations from Plan 091;
- framed/tiled JPEG extraction;
- opaque prepared JPEG repeated operations from Plan 094;
- seed-hint explicit operation and its insufficient-hint failure;
- structured error handling without exhaustive matching on future-proof enums;
- default features only; `application-support` must not be enabled.

Required root-facade consumer coverage, if `stegoeggo::stego` is retained:

- the documented convenience subset compiles and behaves identically to the direct package;
- no extra carrier API exists only through the root facade;
- docs explicitly prefer the direct package for generic-only use.

Keep consumer fixtures small. They are compatibility evidence, not another example application.

## 3. SemVer and public API audit

Use Cargo SemVer compatibility guidance as the baseline. Before finalizing the roadmap, classify every public change since the published/current 0.4 carrier surface as:

- additive compatible;
- correctness fix with no valid-success contract regression;
- deprecation/transition;
- breaking and therefore reserved for an explicit new compatibility boundary.

Run `cargo-semver-checks` against the appropriate published/reference version when practical. It should be tooling used for evidence rather than a production dependency. If the tool cannot model an intentional pre-1.0/breaking transition cleanly, record the exact reported breakages and reconcile each against the approved migration inventory instead of suppressing them wholesale.

Audit public structs/enums for future-proofing:

- prefer private fields for new structs;
- use `#[non_exhaustive]` where callers should tolerate future variants;
- ensure new validated scalar types carry common traits;
- avoid exposing implementation collection types (`HashMap` coefficient maps, etc.);
- avoid public type aliases that leak private implementation choices.

## 4. Stability/deprecation documentation

Reconcile at minimum:

- `stegoeggo-stego/README.md`;
- carrier crate rustdoc (`stegoeggo-stego/src/lib.rs` plus module docs);
- root `README.md`;
- `docs/rust-api.md`;
- `STABILITY.md`;
- `DEPRECATIONS.md`;
- `SUPPORT.md`;
- `CHANGELOG.md`;
- architecture carrier/steganography/pipeline docs;
- `.skills/stegoeggo-conventions/SKILL.md` and architecture-review guidance where paths/contracts changed.

Required documentation statements:

1. `stegoeggo-stego` is the canonical package for arbitrary-payload carrier use.
2. The parent `stegoeggo` package owns rights metadata, policy, provenance, application fallback, and verification semantics.
3. CRC32 framing is corruption detection, not authentication.
4. The seed selects deterministic carrier positions and is not a cryptographic secret.
5. LSB survives lossless preservation of carrier channel bytes, not lossy transcoding.
6. JPEG DCT support is the exact currently supported coding subset; progressive/multiscan/restart limitations are explicit.
7. The JPEG algorithm is an F5-style/no-zero-coefficient StegoEggo variant, with no claim of conventional-F5 interoperability.
8. JPEG payload capacity uses eligible AC coefficients (`|coef| >= 2` under the current mapping), not all non-zero AC coefficients.
9. Raw tiled extraction is not self-authenticating; framed tiled recovery is recommended when candidate validation is needed.
10. Prepared JPEG exposes reusable operations but no codec/coefficient internals.
11. Generic pixel-buffer layout/stride/alpha/padding behavior is exact.
12. The carrier-mapping version and compatibility policy from Plan 093 are explicit.

## 5. Examples

Move the generic example emphasis away from `stegoeggo::stego` toward direct `stegoeggo_stego` imports. Keep at least one concise root-facade example only if the convenience re-export remains an intentional supported path.

Examples should demonstrate realistic error handling rather than `unwrap` where the point is API usage. Include at least:

- direct framed LSB on a caller-owned buffer;
- strict JPEG embed after support/capacity check;
- prepared JPEG repeated read operations;
- best-effort JPEG only when explicitly selected;
- authentication layering guidance as prose, not a home-grown cryptographic example.

## 6. Package and dependency disposition

### 6.1 Release cadence

The carrier is a separate crates.io package but currently versioned in workspace lockstep with the parent, and the root depends on `=0.4.0` plus hidden `application-support`.

Record an explicit decision:

- **lockstep retained** if atomic parent/carrier compatibility and manual release simplicity outweigh unrelated version bumps; or
- **independent carrier versioning** if external generic consumers now justify carrier releases that do not track rights/CLI changes.

If decoupling is selected, define exact release ordering and compatible root dependency ranges before changing manifests. Do not loosen the root dependency specifier until compatibility between the public and hidden support surfaces is understood.

No crates.io publish is part of this plan unless the user separately authorizes a release.

### 6.2 Dependency audit

Re-run `cargo tree -p stegoeggo-stego --edges normal`. The generic carrier must remain free of rights/provenance/HMAC/signature/metadata dependencies.

Evaluate whether `image` can or should become optional after Plan 095. Do not remove/default-disable it merely to reduce dependency count: current `RgbaImage` public compatibility and JPEG support must remain ergonomic. Record the decision with measured benefit/cost.

No dependency should be added for an abstraction that can be implemented safely and clearly with std/current dependencies.

## 7. MSRV/platform evidence

Re-run the existing assurance expectations for the carrier:

- Rust 1.87 compile and carrier tests;
- stable Linux x86_64 full gate;
- scheduled/native evidence for Linux aarch64, macOS aarch64, and Windows x86_64 as configured by Roadmap 081;
- direct consumer fixture on at least primary stable Linux and MSRV where practical.

New pixel-view arithmetic must have platform-width tests sufficient to guard 32-bit `usize` overflow even if a 32-bit runner is not part of supported CI.

## 8. CI/repository governance truthfulness

At the audited baseline GitHub reports `main` as unprotected with required-status-check enforcement disabled, while `SUPPORT.md` describes `Check` as the sole required check.

Resolve the discrepancy in one of two truthful ways:

- enable branch/ruleset enforcement for the documented required check; or
- change repository documentation to describe CI as the standard push/PR gate but not an enforced GitHub required check.

Do not claim enforcement based only on workflow existence or a green run. Record the final hosting configuration/evidence in `plans/097-status.md`.

## 9. Final verification matrix

Before marking complete, record exact commands/results for:

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo check -p stegoeggo-stego --locked
cargo test -p stegoeggo-stego --all-features
cargo test --workspace --exclude stegoeggo-fuzz --all-features
cargo package -p stegoeggo-stego --allow-dirty --list
./scripts/check.sh
```

Also run focused public known-answer, JPEG preservation, tiled recovery, prepared-decode-count, strided-buffer, parent verification-convergence, and legacy-compatibility suites.

If `cargo-semver-checks` is used, record tool version, baseline version/ref, and every intentional exception.

## 10. Acceptance criteria

- Direct generic consumers use `stegoeggo-stego` without parent/application dependencies or hidden features.
- Public docs and examples describe the final API exactly and prefer direct carrier usage for generic tasks.
- All breaking changes are accounted for at an explicit compatibility boundary; no accidental semver break is hidden in a cleanup commit.
- Carrier/result/config types are future-proofed consistently with the final v1 contract.
- Package contents contain all required docs/examples and no unintended workspace-only files.
- Dependency audit confirms no rights/provenance leakage into the carrier crate.
- Release cadence is explicitly retained or decoupled with a documented rationale and release ordering.
- MSRV/platform claims have current evidence.
- GitHub enforcement/documentation are truthful and consistent.
- Roadmap 090 status ledger cites every child-plan completion and the final integrated `./scripts/check.sh` result.

## 11. Non-goals

No automatic crates.io release, no release automation expansion, no new stego algorithm, no progressive JPEG implementation, no C2PA work, no benchmark marketing claims beyond recorded evidence, and no branch-protection change unrelated to making existing support claims truthful.
