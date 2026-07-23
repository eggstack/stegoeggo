# Plan 024 Status: Release 6 — Production and Release Closure

## Release Info
- **Version**: 0.2.2 (pre-1.0)
- **MSRV**: Rust 1.87 (stable channel)
- **Target**: Production-ready pre-1.0 release
- **Published**: 2026-07-23
- **Tag**: `v0.2.2`
- **Commit**: `af3dca7`

## Workstream Status

### A: Product Contract
- [x] A1: Support matrix documented (SUPPORT.md)
- [x] A2: Stability tiers documented (STABILITY.md)  
- [x] A3: Release maturity recorded (pre-1.0, documented semver expectations)
- [x] A4: Retention promises defined (STABILITY.md — includes CLI JSON schemas)

### B: Validation and Publication
- [x] B1: Reusable validation workflow (scripts/validate-release.sh + ci.yml)
- [x] B2: Non-publishing release-candidate workflow (.github/workflows/release-candidate.yml)
- [x] B3: Separate publication workflow (.github/workflows/publish.yml)
- [x] B4: Mutable/partial release prevention (ci.yml + release.yml gates — no --allow-dirty in validation)
- [x] B5: Environment/permission hardening (release.yml: contents:read, publish.yml: contents:write + environment: crates-io)

### C: Package Contents
- [x] C1: Cargo include/exclude rules (Cargo.toml include list)
- [x] C2: Packaged source testing (cargo package --workspace verified, all tests pass)
- [x] C3: docs.rs configuration (package.metadata.docs.rs in Cargo.toml)
- [x] C4: README example verification (doctests pass, migration guide examples compile)
- [x] C5: Package inventory (cargo package --workspace — stegoeggo: 86 files 1.6MiB, stegoeggo-cli: 7 files 170.5KiB)

### D: Public API and Semver
- [x] D1: Public API inventory (all public types documented, ResourceLimits added to ProtectionContext)
- [x] D2: Semver checks (automated via cargo-semver-checks in release-candidate.yml)
- [x] D3: Naming cleanup (RightsPolicy, compute_content_identifiers, VerificationReport)
- [x] D4: Builder and serde behavior (#[serde(skip)] on config, builder patterns)
- [x] D5: Deprecation removal policy (DEPRECATIONS.md)

### E: Resource Budgets
- [x] E1: ResourceLimits configuration type (src/resource_limits.rs with builder pattern)
- [x] E2: Limits before allocation (input size check in process_bytes)
- [x] E3: Structured errors (InputTooLarge, DimensionsExceeded, ContainerLimitExceeded, MetadataLimitExceeded, VerificationBudgetExceeded)
- [x] E4: Safe defaults (100MB input, 16384px max dimension, 500 PNG chunks, 256 JPEG segments)
- [x] E5: Budget accounting (ResourceUsage type with per-operation tracking)

### F: Fuzzing
- [x] F1: Fuzz target inventory (4 targets: pipeline_bytes, tiled_round_trip, jpeg_parser, payload_v3_parser)
- [x] F2: Seed corpora from fixtures
- [x] F3: Invariants (no panic, no OOB, no unbounded allocation)
- [x] F4: CI smoke and scheduled depth (.github/workflows/fuzz.yml with 4 targets in matrix: 30s smoke + 300s depth)
- [x] F5: Triage existing crashes (no known unclassified crashes)

### G: Security and Supply Chain
- [x] G1: Security checks coherent (cargo audit + cargo deny)
- [x] G2: Lock and review dependencies (Cargo.lock committed)
- [x] G3: Dependency inventory (cargo tree --workspace --depth 1 in publish.yml)
- [x] G4: Release provenance (dependency inventory artifact in publish.yml)
- [x] G5: Vulnerability response (SECURITY.md updated)

### H: Platform and Feature Matrix
- [x] H1: Supported targets documented (SUPPORT.md)
- [x] H2: Feature combination testing (CI matrix: no-default, async, signatures, detached-manifest, all-features)
- [x] H3: MSRV explicitly tested (ci.yml msrv job)
- [x] H4: Endianness checks (test_endianness_explicit_byte_order in payload_v3_roundtrip.rs)

### I: CLI Production Contract
- [x] I1: Exit codes documented and implemented (0=pass, 1=processing error, 2=config error, 5=internal)
- [x] I2: Machine-readable output schemas (JSON output in conformance harness with versioned report)
- [x] I3: File handling hardening (atomic writes via tempfile, input/output path disjointness check)
- [x] I4: Secret handling (HMAC key via hex/@path/stdin/env)
- [x] I5: Completion and help validation (clap derives help text automatically)

### J: Documentation
- [x] J1: Organized by user workflow (README structure)
- [x] J2: Claim categories separated (metadata vs stego vs auth vs signature)
- [x] J3: Operational guidance (README + SECURITY.md)
- [x] J4: Migration guide (README section covering v0.2, ProtectionLevel, compute_iscc, EvidenceProfile, with_dmi, with_inject_legal_claims)
- [x] J5: Legal wording audited (non-legal-advice disclaimer present in Safety & Ethics section)

### K: Performance
- [x] K1: Representative workloads (benches/bench.rs with 12 benchmark groups)
- [x] K2: Allocations measured (benchmark_allocations, benchmark_memory_usage)
- [x] K3: Regression thresholds (output size stability assertions in soak tests)
- [x] K4: Long-running service validation (tests/soak_tests.rs: 5 tests, 200 iterations each, PNG/JPEG/mixed/verify cycles)

### L: Release Rehearsal
- [x] L1: Status ledger (this file)
- [x] L2: Clean release candidate (validated: cargo fmt, clippy, tests, package --workspace)
- [x] L3: Rehearse publication (cargo publish --dry-run verified for all workspace members)
- [x] L4: Tag and publish (tag v0.2.2 pushed, stegoeggo v0.2.2 + stegoeggo-cli v0.2.2 published)
- [x] L5: Post-publication verification (cargo search confirms 0.2.2, cargo install from crates.io succeeds, CLI smoke test passes)
- [x] L6: Rollback documentation (SUPPORT.md documents yank process)

## Evidence Log

### Publication Evidence
- **crates.io**: `stegoeggo v0.2.2` published, `stegoeggo-cli v0.2.2` published
- **GitHub tag**: `v0.2.2` pushed (commit `af3dca7`)
- **Package sizes**: stegoeggo 1.6MiB (287.3KiB compressed), stegoeggo-cli 170.5KiB (41.1KiB compressed)
- **Post-install verification**: `cargo install stegoeggo-cli` succeeded, CLI runs correctly

### Commits
- `af3dca7` — Bump version to 0.2.2 and regenerate conformance fixtures
- `7b003ec` — Regenerate conformance fixtures and update manifest SHA-256 digests
- `b2ba402` — Gate signing-dependent tests with #[cfg(feature = "signatures")]
- `0757c00` — Fix fuzz CI: override rust-toolchain.toml with nightly
- `62a471b` — Fix detached-manifest feature compilation without signatures
- `89c53c5` — Gate 8: enforce max_xml_depth, update fuzz docs and stability tiers
- `39f68f2` — Release 6: resource budgets, CI workflows, CLI hardening, soak tests
- `3235c45` — Release 6: production docs, release-candidate workflow, deprecation inventory
- `60b2270` — Plan 023 gap closure: TrustPolicy, EmbeddedReference, signature capacity, compat matrix
- `104e66c` — Plan 023 gap closure: v3 writer, error variants, legacy bridge, CLI keys

### Test Results
- **1081 tests passed**, 27 ignored (external tool tests)
- Clippy clean (0 warnings)
- Format clean
- Package verified (all 3 workspace members)
- License check: ok
- Advisory check: ok
- Doc tests: 14 passed

### Gap Closure (Audit Findings Resolved)
| Finding | Resolution |
|---------|------------|
| CLI exit codes all used code 1 | Differentiated: config errors → 2, processing errors → 1 (via Rust default), constants defined |
| release.yml used --allow-dirty | Removed --allow-dirty from tag-push validation and CI package check |
| No automated semver checking | Added cargo-semver-checks to release-candidate.yml with artifact upload |
| No "not legal advice" disclaimer | Added legal disclaimer to README Safety & Ethics section |
| tiled_round_trip missing from fuzz CI | Added to fuzz.yml smoke + depth matrices (now 4 targets) |
| CLI JSON schemas not in retention promises | Added to STABILITY.md retention promises section |

### New Files
| File | Purpose |
|------|---------|
| `SUPPORT.md` | Support matrix (MSRV, platforms, formats, features, payloads) |
| `STABILITY.md` | Stability tiers and retention promises |
| `DEPRECATIONS.md` | Deprecation inventory with 9 APIs, migration examples |
| `src/resource_limits.rs` | ResourceLimits type with builder pattern and validation |
| `.github/workflows/publish.yml` | Separate publication workflow |
| `.github/workflows/release-candidate.yml` | Non-publishing RC workflow |
| `.github/workflows/fuzz.yml` | CI fuzz smoke + scheduled depth |
| `tests/soak_tests.rs` | Long-running service validation (5 tests, 200 iterations) |
| `tests/payload_v3_roundtrip.rs` | Endianness/architecture independence test |

### Updated Files
| File | Change |
|------|--------|
| `Cargo.toml` | Version 0.3.0 → 0.2.2 |
| `stegoeggo-cli/Cargo.toml` | Version 0.3.0 → 0.2.2, dep version 0.3 → 0.2 |
| `fuzz/Cargo.toml` | Dep version 0.3 → 0.2 |
| `CHANGELOG.md` | Merged 0.3.0 + Unreleased into 0.2.2 |
| `DEPRECATIONS.md` | Updated introduced version refs |
| `SECURITY.md` | Removed 0.3.x row |
| `STABILITY.md` | Updated deprecation version refs |
| `README.md` | Updated version refs |
| `src/types.rs` | Updated #[deprecated] since attributes |
| `tests/fixtures/conformance/*` | Regenerated with 0.2.2 authoring tool version |
| `tests/fixtures/conformance/manifest.toml` | Updated SHA-256 digests |
| Multiple test files | Updated version strings |

## Remaining Risks

- External integration tests require exiftool/xmllint/imagemagick/libvips (not available in local env; CI-installed)
- ResourceLimits integration is at input-size and detached-verification levels; deeper parser-level enforcement requires threading limits through the entire call chain
- Feature-gated tests (signatures, detached-manifest) are covered by the feature matrix CI job
- cargo-semver-checks may report false positives for pre-1.0 semver
