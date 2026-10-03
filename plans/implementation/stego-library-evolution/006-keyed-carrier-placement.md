# Stego Library Evolution Milestone 006 — Keyed Carrier Placement

Status: blocked  
Repository baseline: `d5425d3c73e634fd2276d44507fa94728a18784f`  
Source roadmap: `plans/subsystems/stego-library-evolution-roadmap.md#7-milestones`  
Long-term requirements: `plans/000-long-term-specification.md#2-canonical-api-invariants`  
Applicable ADRs: `plans/adrs/ADR-0007-keyed-carrier-selection.md`  
Primary class: capability

Blockers: ADR-0007 must be accepted; M008 compatibility corrective must be closed.

## 1. Objective

Implement the accepted keyed-placement contract as an opt-in generic carrier scheme
while preserving every legacy seed-based byte sequence and avoiding false claims about
payload encryption/authentication.

## 2. Why this milestone is blocked

The public scheme is a durable compatibility protocol. ADR-0007 is deliberately
proposed, not accepted. M002/M003 are closed, but the post-M003 compatibility audit
opened M008 to restore the pre-0.4.2 one-shot input/error contract before more carrier
surface is frozen. No implementation may freeze algorithm identifiers, key derivation,
dependency choices, or placement configuration before both blockers close.

## 3. Current implementation evidence

Existing configs use `u64 seed`; root default seeds are CSPRNG-generated but seed
size/placement is not secret-key security. LSB and JPEG use distinct historical
selection algorithms and must remain byte-stable.

## 4. Invariants that must not regress

All current known-answer vectors remain identical. Keyed and legacy placement are
explicitly distinct. Keys never enter Debug/errors/reports. Generic framing CRC32
remains only corruption detection.

## 5. Scope

In after unblock: candidate primitive benchmark; 32-byte key type; versioned placement
scheme; domain separation; LSB/JPEG/tiled/views/prepared parity; known-answer vectors.

Out: payload encryption, password KDF, rights HMAC replacement, automatic key
discovery, changing StegoEggo parent defaults.

## 6. Required production changes

Implement only the accepted ADR. Expected work includes:

- secret key wrapper with redacted `Debug`;
- deterministic domain-separated generator/permutation primitive;
- additive config constructors/options for LSB/JPEG/tiled operations;
- shared implementation across `RgbaImage` and borrowed pixel views;
- prepared JPEG extraction/embedding parity;
- frozen scheme identifier and independent known-answer vectors;
- optional dependency feature if justified so non-keyed users pay no unnecessary cost.

## 7. Ordered work packages

WP0 unblock record: accepted ADR + exact chosen primitive/dependencies.  
WP1 placement core and KATs.  
WP2 LSB/views/tiled integration.  
WP3 JPEG/prepared integration.  
WP4 security ergonomics, docs, direct-consumer tests.  
WP5 semver/MSRV/dependency qualification.

## 8. Failure, cancellation, restart, contention semantics

Wrong key returns normal extraction/frame-integrity failure without key-dependent secret
diagnostics. No partial output on strict embed failure.

## 9. Compatibility and migration

No migration of existing carriers. Legacy seed APIs remain available and byte-stable.
Keyed placement is opt-in.

## 10. Required tests

Legacy KAT invariance; keyed LSB/JPEG KATs; domain-separation vectors; wrong-key
negative tests; views/prepared/tiled parity; redacted Debug; no key bytes in formatted
errors/reports; capacity/failure atomicity.

## 11. Required verification commands

```bash
cargo test -p stegoeggo-stego --all-features
cargo test --workspace --all-features --test known_answer_vectors
cargo test --workspace --all-features --test public_stego_api
./scripts/check.sh
cargo semver-checks check-release -p stegoeggo-stego
```

Also record MSRV and dependency audit for the exact selected primitive.

## 12. Documentation updates

ADR status/decision evidence, carrier security model, README/rustdoc,
`docs/carrier-crate.md`, `STABILITY.md`, architecture, CHANGELOG.

## 13. Acceptance criteria

A caller can choose a 256-bit-keyed placement scheme with frozen portable vectors;
legacy vectors are unchanged; API/docs make clear this is position secrecy only.

## 14. Stop conditions

Stop if ADR remains proposed, M008 remains open, selected primitive cannot meet
MSRV/portability/no-unsafe requirements, or implementation requires changing existing
default placement.

## 15. Closure evidence required

Accepted ADR reference, primitive/dependency evaluation, KAT corpus, legacy parity,
redaction tests, semver/MSRV/check evidence.

## 16. Handoff notes

Do not choose a crypto primitive merely because it is already transitive elsewhere.
Carrier dependencies are an independent public maintenance commitment.
