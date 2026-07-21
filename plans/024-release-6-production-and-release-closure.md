# Plan 024: Release 6 — Production and Release Closure

## Status

Blocked on completion of Plans 021–023.

## Release intent

Release 6 converts the standards-correct, policy-separated, provenance-capable codebase into a production-ready and repeatably releasable Rust library and CLI.

This release is not a feature-expansion phase. It closes packaging, compatibility, resource-bound, parser-hardening, documentation, supply-chain, and release-process gaps. The central requirement is that a published artifact can be reproduced from a tagged commit, passes the same blocking gates as main, exposes a coherent supported API, and remains robust against malformed or adversarial image and metadata inputs.

Release 6 must preserve every prior conformance and compatibility guarantee. No production-cleanup change may weaken canonical PLUS metadata, legal-field semantics, legacy reading, policy/channel separation, payload compatibility, signature/trust distinctions, or detached-manifest verification.

## Prerequisites

Before implementation begins:

- Plan 021 is closed with a versioned conformance report and clean CI evidence.
- Release 4 request/policy/channel architecture is complete.
- Release 5 payload and provenance decisions are complete, including the C2PA ADR.
- Public deprecations introduced by Releases 4 and 5 are inventoried.
- The supported MSRV, feature matrix, platforms, and release artifact set are explicitly agreed.
- A release candidate version is selected. Do not assume this release is `1.0.0`; stable-major publication requires a separate API-stability decision.

## Objectives

At completion:

1. Release validation and publication are separate, explicit, and fail closed.
2. Published crates and binaries contain exactly the intended files and documentation.
3. Public API compatibility is measured and documented.
4. All parser and verifier entrypoints have explicit resource budgets.
5. Fuzzing covers all externally reachable container, metadata, payload, manifest, merge, and repeated-update paths.
6. Feature combinations, MSRV, supported platforms, docs.rs, and package builds are continuously validated.
7. Security and supply-chain checks are blocking and reviewable.
8. CLI behavior, exit codes, machine-readable output, and secret handling are stable.
9. Documentation is organized around actual user workflows and precise evidence claims.
10. A tagged release can be produced from a clean commit with auditable provenance and rollback instructions.

## Non-goals

Release 6 must not:

- Introduce new rights policies, image formats, hidden-marker algorithms, signature schemes, or trust services.
- Remove deprecated APIs unless the selected release is an approved semver-major release.
- Implement C2PA unless Plan 023’s ADR explicitly schedules it here.
- Add a network service, hosted registry, or cloud control plane.
- Promise legal enforceability, infringement proof, or training detection.
- Pursue broad micro-optimization without measured production value.

## Workstream A: Define the supported product contract

### A1. Publish a support matrix

Document:

- Rust MSRV.
- Tested stable Rust version.
- Supported operating systems and architectures.
- Supported image formats.
- Supported payload versions for reading and writing.
- Supported manifest schema versions.
- Default and optional Cargo features.
- CLI installation methods.
- External tools required only for development/conformance, not runtime use.

### A2. Define stability tiers

Classify public surfaces:

- Stable library API.
- Deprecated compatibility API.
- Experimental feature-gated API.
- CLI stable commands/flags.
- Machine-readable schemas with independent versioning.
- Internal implementation details.

### A3. Decide release maturity

Record whether the release is:

- Another pre-1.0 release with documented semver expectations, or
- A 1.0 candidate requiring stable API commitment.

Do not infer stability solely from roadmap completion.

### A4. Define retention promises

State minimum compatibility promises for:

- Canonical metadata reading/writing.
- Legacy metadata reading.
- Payload v1/v2/v3 reading.
- Detached manifest schema reading.
- CLI JSON report schemas.

### Acceptance criteria

- Support matrix is checked into the repository.
- Every public feature is assigned a stability tier.
- Compatibility promises are testable and linked to fixtures.
- Release maturity decision is recorded in an ADR or release-status file.

## Workstream B: Separate validation from publication

### B1. Create a reusable validation workflow

Move all release-blocking validation into a reusable workflow or checked-in script invoked by both main CI and release candidates.

The contract must include:

- Formatting.
- Clippy with warnings denied.
- Unit/integration/doc tests.
- MSRV checks.
- Feature-matrix checks.
- Package verification.
- License and advisory checks.
- Security audit policy.
- Strict external conformance.
- External integration tests.
- Documentation build.
- Semver/API checks.
- Required fuzz smoke tests.

### B2. Add a non-publishing release-candidate workflow

Support `workflow_dispatch` or an equivalent mechanism that validates a commit/tag candidate and uploads all artifacts without publishing.

The candidate workflow must produce:

- Packaged `.crate` files.
- CLI binaries for supported targets where available.
- Checksums.
- Conformance report.
- Tool-version report.
- Test/feature matrix summary.
- Package-content inventory.
- SBOM or dependency inventory if adopted.

### B3. Create a separate publication workflow

Publication must:

- Require a protected tag or approved environment.
- Consume or rerun the exact validated commit.
- Verify tag/package versions.
- Verify candidate artifacts/checksums where practical.
- Publish crates in dependency order.
- Create the GitHub release and attach artifacts.
- Stop immediately on any mismatch.

### B4. Prevent mutable or partial releases

- Do not publish from a dirty checkout.
- Do not use `--allow-dirty` in publication.
- Do not use `--no-verify` without an approved, documented exception.
- Do not continue after failed license/advisory/conformance checks.
- Define recovery steps for partial workspace publication.

### B5. Add environment and permission hardening

Use least-privilege GitHub permissions. Publication credentials must be scoped to the publication job/environment and unavailable to pull-request jobs.

### Acceptance criteria

- Validation can run without publishing.
- Publication cannot run on an unvalidated or mismatched commit.
- All release gates are blocking.
- Workflow permissions are minimal and documented.
- A dry-run release candidate completes successfully before the first production release.

## Workstream C: Verify package contents and documentation

### C1. Audit Cargo include/exclude rules

The current root package includes source, examples, tests, and benches. Decide intentionally which of these belong in the published crate.

For each workspace crate:

- Inspect `cargo package --list`.
- Remove generated fixtures, large binaries, development-only plans, or sensitive files unless intentionally published.
- Include required licenses, README, changelog, security policy, and schema/spec documents.

### C2. Test packaged sources, not only the working tree

Unpack the generated `.crate` into a clean directory and run:

- `cargo check` under default features.
- `cargo test` where packaged tests are included.
- `cargo doc`.
- Example compilation.
- Feature combinations supported for consumers.

### C3. Validate docs.rs configuration

- Build documentation with the configured features and rustdoc flags.
- Eliminate broken intra-doc links and undocumented public items.
- Confirm feature-gated APIs appear correctly.
- Ensure examples do not rely on unpublished files.

### C4. Verify README examples

Use compile-tested doctests or dedicated example tests for primary workflows:

- Metadata-only rights notice.
- Policy/channel request.
- Legacy compatibility preset.
- HMAC provenance.
- Signature/detached manifest when features are enabled.
- Verification and trust policy.

### C5. Add package inventory artifacts

Upload a package file list and size summary in release-candidate validation.

### Acceptance criteria

- `cargo package --list` matches an approved inventory.
- Packaged crates build and document from a clean unpacked directory.
- No development-only large fixture corpus is accidentally published unless explicitly required.
- All primary README examples compile.
- docs.rs-equivalent build is green.

## Workstream D: Complete public API and semver cleanup

### D1. Generate a public API inventory

Use rustdoc JSON, `cargo public-api`, or another maintained tool to record the exported surface for each crate and feature set.

### D2. Run semver checks

Use `cargo-semver-checks` or an equivalent process against the previous published release.

Classify every change as:

- Additive.
- Deprecated but compatible.
- Breaking and deferred.
- Breaking and approved only if this is a major release.

### D3. Finish naming cleanup

Review:

- `ProtectionLevel` compatibility APIs.
- `EvidenceProfile` compatibility APIs.
- `DmiValue` versus `RightsPolicy` naming.
- ISCC/content identifier replacements.
- Verification summary versus structured report types.
- Ambiguous `protect`, `verify`, `legal`, or `provenance` names.

### D4. Stabilize builder and serde behavior

For public configuration/report types:

- Document serde field names and defaults.
- Add schema versions where long-lived serialized configuration is supported.
- Avoid serializing secrets.
- Test unknown-field and forward-compatibility behavior.

### D5. Define deprecation removal policy

List deprecated APIs with:

- Replacement.
- Introduced-deprecated version.
- Earliest removal version.
- Migration example.

Do not remove them in Release 6 unless semver-major removal is explicitly approved.

### Acceptance criteria

- Public API inventory is stored as an artifact or checked-in baseline.
- Semver checks are blocking for release candidates.
- Every deprecation has a replacement and removal policy.
- No secret-bearing type derives unsafe serialization/debug behavior.
- Machine-readable schemas have independent versioning.

## Workstream E: Add explicit resource budgets

### E1. Define one resource-limit configuration

Introduce a validated `ResourceLimits` or equivalent configuration covering externally reachable work:

- Maximum input bytes.
- Maximum dimensions/pixels.
- Maximum PNG chunk count and chunk length.
- Maximum JPEG segment count/length and scan complexity.
- Maximum WebP RIFF chunk count/length.
- Maximum XMP packet bytes.
- Maximum XML depth, attribute count, text length, and property count.
- Maximum metadata fields and per-field length.
- Maximum detached manifest bytes and nesting.
- Maximum payload bytes/extensions.
- Maximum tile extraction origins.
- Maximum verification candidates/seeds.
- Maximum decompressed or allocated working memory where enforceable.

### E2. Apply limits before allocation

Parsers must validate declared lengths and counts before allocating or slicing. Use checked arithmetic for every container offset and size calculation.

### E3. Distinguish configuration errors from resource exhaustion

Return structured errors such as:

- InputTooLarge.
- DimensionsExceeded.
- ContainerLimitExceeded.
- MetadataLimitExceeded.
- VerificationBudgetExceeded.
- UnsupportedComplexity.

Do not collapse these into generic decode errors.

### E4. Define safe defaults and opt-in expansion

Defaults should suit web-facing services without rejecting normal photographs. Higher limits must be explicit. Document denial-of-service implications.

### E5. Add budget accounting to results

Where useful, report observed counts/bytes/candidates so operators can tune limits.

### Acceptance criteria

- Every externally reachable parser has documented limits.
- Declared-length attacks cannot trigger unbounded allocation.
- Limit failures are deterministic and tested.
- Default limits are exercised against representative large valid images.
- No compatibility path bypasses the limits.

## Workstream F: Complete parser and merge fuzzing

### F1. Inventory fuzz targets

Required targets include:

- PNG chunk walker and metadata extraction.
- JPEG marker/header parser.
- JPEG entropy/DCT parser.
- WebP RIFF chunk parser.
- XMP extraction and XML/property parsing.
- Canonical/legacy DMI parsing.
- Metadata merge/update policy.
- Repeated protect/update cycles.
- Hidden payload v1/v2/v3 parser.
- Detached manifest parser.
- Canonical claim decoder.
- Signature/authentication records.
- CLI/config deserialization where exposed.

### F2. Seed corpora from real fixtures

Use canonical, legacy, malformed, conflict, preservation, external, payload-version, and manifest fixtures as fuzz seeds.

### F3. Add invariants

Fuzz assertions should include:

- No panic.
- No out-of-bounds access.
- No unbounded allocation under configured limits.
- Parse-serialize-parse stability where applicable.
- Merge idempotence.
- Unrelated metadata preservation.
- Reject malformed authentication without treating it as absent.

### F4. Add CI smoke and scheduled depth

- Short deterministic fuzz smoke runs in pull requests or main CI.
- Longer scheduled runs with retained crash artifacts.
- Document reproduction and corpus-minimization procedures.

### F5. Triage existing crashes

No known reproducible crash may remain unclassified at release time. Security-sensitive findings require an advisory process.

### Acceptance criteria

- All required targets exist and compile.
- CI smoke runs are blocking.
- Scheduled fuzz workflow uploads crash artifacts.
- Current seed corpus completes without crashes.
- Resource-budget invariants are enforced in fuzz harnesses.

## Workstream G: Security and supply-chain closure

### G1. Make security checks coherent

Choose and document the roles of:

- `cargo deny` licenses.
- `cargo deny` advisories.
- `cargo audit` if retained.
- Dependency source/duplicate/wildcard policies.

Avoid redundant jobs that disagree silently.

### G2. Lock and review dependencies

- Commit and verify `Cargo.lock` for workspace binaries.
- Review optional cryptographic dependencies and feature activation.
- Ban git/path dependencies in published crates unless explicitly approved.
- Document MSRV impact of dependency updates.

### G3. Generate dependency inventory or SBOM

Produce a machine-readable dependency inventory for release candidates. If adopting CycloneDX/SPDX tooling, pin and document it.

### G4. Harden release provenance

Where supported, use GitHub artifact attestations or equivalent build provenance for published binaries. This is build provenance, not content-rights provenance; keep terminology distinct.

### G5. Define vulnerability response

Update `SECURITY.md` with:

- Supported versions.
- Reporting channel.
- Response expectations.
- Embargo/advisory process.
- Handling of malformed-input crashes and cryptographic defects.

### Acceptance criteria

- License/advisory policies are blocking and documented.
- Dependency inventory is attached to release candidates.
- Cryptographic dependencies are feature-scoped and reviewed.
- Security policy names supported versions and response process.
- Release build provenance is clearly distinguished from image provenance.

## Workstream H: Platform and feature matrix

### H1. Define supported targets

At minimum evaluate:

- Linux x86_64.
- Linux aarch64.
- macOS x86_64/aarch64 as applicable.
- Windows x86_64.

Separate library support from prebuilt CLI binary availability.

### H2. Test feature combinations

Include:

- Default features.
- `async`.
- Signature/provenance features from Release 5.
- Detached manifest feature.
- All features.
- No-default-features.

Development-only `fuzz` and `test-seeds` must not accidentally alter production behavior.

### H3. Test MSRV explicitly

MSRV must compile supported default and documented feature combinations. If cryptographic optional features require a higher Rust version, either raise MSRV intentionally or document a feature-specific constraint; prefer one coherent MSRV.

### H4. Check endianness and architecture assumptions

Payload and manifest encodings must use explicit byte order. Avoid `usize`-dependent serialized fields. Add tests where feasible.

### Acceptance criteria

- Supported platform matrix is green or each exception is documented.
- Feature powerset is reduced to a practical, explicitly tested matrix.
- MSRV job covers consumer-relevant features.
- Development-only features do not ship enabled by default.

## Workstream I: CLI production contract

### I1. Stabilize command and exit-code behavior

Define exit codes for:

- Success.
- Invalid CLI/configuration.
- Input/decode failure.
- Policy validation failure.
- Verification mismatch.
- Authentication/signature failure.
- Resource-limit failure.
- Internal error.

### I2. Add stable machine-readable output

For verify, explain-plan, manifest, and conformance commands:

- Add schema versions.
- Send JSON to stdout and diagnostics to stderr.
- Avoid mixed human/JSON output.
- Preserve non-zero exit semantics independently from report generation.

### I3. Harden file handling

- Atomic output writes.
- Explicit overwrite behavior.
- Preserve or document file permissions.
- Avoid partial output on failure.
- Safely handle input/output path equality.
- Bound stdin input.

### I4. Protect secrets

- No private keys or HMAC keys in process arguments by default.
- Redact environment variable names/paths where appropriate.
- Avoid shell-history examples containing secrets.
- Test logs and panic messages.

### I5. Add completion and help validation

If shell completions/man pages exist, generate and package them deterministically. Snapshot primary help output to catch accidental breaking changes.

### Acceptance criteria

- CLI exit codes are documented and tested.
- JSON modes are schema-versioned and clean.
- Output writes are atomic.
- Secret-redaction tests pass.
- CLI package/binary smoke tests run on supported platforms.

## Workstream J: Documentation and examples restructuring

### J1. Organize by user workflow

Primary documentation order:

1. Emit a standards-based metadata-only rights notice.
2. Verify canonical and legacy metadata.
3. Configure explicit rights policy and channels.
4. Add best-effort hidden markers.
5. Add authenticated HMAC provenance.
6. Add signatures and detached manifests.
7. Deploy web/CDN TDM reservation artifacts if still deferred, with clear external guidance.
8. Integrate into services and batch pipelines.

### J2. Separate claim categories

Every relevant page must distinguish:

- Standards syntax.
- External visibility.
- Transformation survival.
- Checksum/integrity.
- Authentication.
- Signature validity.
- Trust.
- Legal effect.

### J3. Add operational guidance

Document:

- Resource limits.
- Key management.
- Metadata stripping by platforms.
- Backups and detached manifest storage.
- Monitoring warnings/errors.
- Upgrade compatibility.
- Incident response for key compromise.

### J4. Add a migration guide

Cover migration from:

- v0.2 legacy metadata.
- v0.3 `ProtectionLevel` API.
- Release 4 request/channel API.
- Old ISCC-like names.
- Legacy verification summaries.

### J5. Audit legal wording

Use accurate technical language and avoid assertions that require jurisdiction-specific legal conclusions. Include a clear non-legal-advice statement without undermining the technical purpose.

### Acceptance criteria

- Documentation leads with metadata-only rights notices.
- All public APIs used in primary examples compile.
- Migration guide covers every deprecated API family.
- Operational and security guidance is present.
- Claims are consistent across README, rustdoc, CLI help, and architecture docs.

## Workstream K: Performance and memory validation

### K1. Define representative workloads

Benchmark:

- Metadata-only same-format operations.
- Format conversion.
- Best-effort hidden markers.
- Tiled markers.
- HMAC and signature operations.
- Detached manifest generation/verification.
- Verification of clean, metadata-rich, and malformed files.

### K2. Measure allocations and peak memory

Use practical tooling to identify regressions. Resource budgets should be validated against measured behavior.

### K3. Add regression thresholds carefully

Use coarse, stable thresholds for catastrophic regressions rather than brittle microbenchmark gates. Record baseline results as artifacts.

### K4. Validate long-running service behavior

Run repeated processing/verification loops to detect:

- Memory growth.
- File descriptor leaks.
- Thread-pool issues.
- Unbounded cache/state accumulation.

### Acceptance criteria

- Metadata-only path remains materially cheaper than hidden-marker paths.
- No unbounded memory growth appears in soak tests.
- Peak memory remains within documented budgets for representative inputs.
- Benchmark baselines are recorded in the release status.

## Workstream L: Final release-candidate and publication rehearsal

### L1. Create `plans/024-status.md`

Maintain criterion-by-criterion evidence throughout the release.

### L2. Run a clean release candidate

From the exact candidate commit:

- Run the reusable validation workflow.
- Build package artifacts.
- Build supported CLI artifacts.
- Generate checksums and dependency inventory.
- Inspect package contents.
- Inspect conformance and tool reports.
- Run install/smoke tests from produced artifacts.

### L3. Rehearse publication without release

Use crates.io dry-run/package verification and a non-production GitHub release draft where safe. Confirm workspace publish order and recovery steps.

### L4. Tag and publish only after evidence approval

The status ledger must name the approved commit and artifact digests before tag publication.

### L5. Verify post-publication state

After publication:

- Install from crates.io.
- Build docs.rs or verify queued build.
- Download release binaries and verify checksums.
- Run CLI smoke tests.
- Confirm repository tag/release notes/changelog match.

### L6. Define rollback and follow-up

Document how to yank a crate version, revoke artifacts, rotate compromised keys, and publish a corrective release.

### Acceptance criteria

- Candidate workflow is green and artifacts inspected.
- Package installation from generated artifacts succeeds.
- Publication permissions and order are verified.
- Post-publication smoke tests are recorded.
- Rollback/yank procedure is documented.

## Required regression and validation matrix

### Core correctness

- Canonical metadata output and external visibility for PNG/JPEG/WebP.
- Legacy metadata and payload read compatibility.
- Legal-field semantic equivalence.
- Metadata preservation and idempotence.
- Policy/channel resolution and metadata-only byte preservation.
- Payload v1/v2/v3 and detached manifest verification.

### Malformed and adversarial input

- Truncated/oversized PNG chunks.
- Invalid JPEG segment lengths and entropy streams.
- Invalid RIFF sizes/chunk padding.
- Oversized/deep XMP.
- Duplicate/conflicting metadata.
- Oversized payload/manifest fields.
- Signature/MAC corruption.
- Repeated update/merge loops.

### Packaging

- Package list snapshot.
- Unpacked crate build/test/doc.
- Examples compile.
- License files present.
- No repository-only paths referenced.

### Compatibility

- Semver check against previous release.
- Deprecated API examples compile.
- CLI legacy flags behave as documented.
- Machine-readable schema compatibility fixtures parse.

### Platforms/features

- Supported OS/architecture matrix.
- MSRV.
- Default/no-default/all-features.
- Async and provenance feature combinations.

## Milestone sequence

### Milestone 1: Product contract and reusable validation

- Support/stability matrices.
- Reusable validation workflow.
- Release candidate workflow.

Gate: validation contract approved and green on main.

### Milestone 2: Package and API closure

- Package inventory.
- Packaged-source tests.
- Public API baseline and semver checks.
- Deprecation inventory.

Gate: clean package builds and semver report approved.

### Milestone 3: Resource budgets and parser hardening

- Resource-limit types.
- Parser enforcement.
- Structured errors.
- Adversarial regression tests.

Gate: all limits and malformed cases pass without panics or unbounded allocation.

### Milestone 4: Fuzzing and security closure

- Full fuzz target inventory.
- CI/scheduled fuzz workflows.
- Supply-chain inventory.
- Security policy update.

Gate: fuzz smoke and security jobs green.

### Milestone 5: CLI, docs, platforms, performance

- Stable CLI/JSON behavior.
- Documentation restructure.
- Feature/platform matrix.
- Benchmarks and soak tests.

Gate: supported matrix and operational docs complete.

### Milestone 6: Release rehearsal and publication

- Candidate artifacts.
- Dry-run/rehearsal.
- Evidence approval.
- Tag/publication/post-release verification.

Gate: all status-ledger criteria pass.

## Release gate

Release 6 is complete only when:

1. Product support and stability contracts are explicit.
2. Validation and publication workflows are separate and fail closed.
3. Published package contents are approved and tested from unpacked artifacts.
4. Public API and semver checks are blocking.
5. Every externally reachable parser has enforced resource budgets.
6. Fuzzing covers all required parser, merge, payload, and manifest paths.
7. Security, license, advisory, and dependency-inventory checks are complete.
8. Supported feature/platform/MSRV matrix is green.
9. CLI exit codes, JSON schemas, file handling, and secret handling are stable.
10. Documentation and migration guides reflect the final architecture accurately.
11. Performance and soak tests show no unacceptable regressions or unbounded growth.
12. A release candidate is built, inspected, installed, and verified before publication.
13. Post-publication smoke tests and rollback procedures are recorded.
14. `plans/024-status.md` contains exact commits, commands, CI runs, artifact IDs/digests, package inventories, API reports, fuzz evidence, benchmarks, and remaining risks.

## Handoff requirements

The implementing agent must update `plans/024-status.md` continuously and must not mark criteria passed solely from commit messages. Evidence must include:

- Reusable workflow and release-candidate run IDs.
- Package file lists and artifact hashes.
- Public API and semver reports.
- Resource-limit test names.
- Fuzz target list and run evidence.
- Security/dependency inventory artifacts.
- Platform/feature matrix conclusions.
- Benchmark and soak results.
- Publication rehearsal results.
- Final tag, published versions, post-publication checks, and rollback readiness.
