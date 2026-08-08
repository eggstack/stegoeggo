# Plan 056 Status Ledger

Plan baseline SHA: `81c934d02dd43578482e01a15ea645a62ec0209b`

Disposition: **OPEN — final XMP reference/serialization residuals remain**

Implementation head: **UNSET**

Plan 056 owns only the final XMP semantic-preservation defects identified after the Plan 054/055 completion claim. JPEG structural correctness and animated-WebP ANMF/container correctness remain closed unless a focused regression demonstrates otherwise.

No version bump, tag, crates.io publication, GitHub release, release automation, dependency expansion, or CI expansion is authorized.

---

## Workstream state

```text
Predefined XML reference handling: OPEN
Numeric character-reference handling: OPEN
Unknown/custom entity rejection policy: OPEN
Attribute semantic decode-before-escape: OPEN
Owned-depth end-event precedence: OPEN
Owned-depth comment suppression: OPEN
Owned-depth processing-instruction suppression: OPEN
Reference handling inside owned subtrees: OPEN
Three-round reference/idempotence proof: OPEN
Public WebP rewrite reference preservation: OPEN
Focused verification: OPEN
Workspace verification: OPEN
Final status reconciliation: OPEN
Publication hold: RETAINED
```

---

## Defect ledger

| item | audited behavior at `81c934d` | exact closure contract | implementation SHA | focused evidence | disposition |
|---|---|---|---|---|---|
| Predefined XML references | `Event::GeneralRef` rejected unconditionally | accept XML predefined references and preserve semantic value | — | required Plan 056 reference fixtures | OPEN |
| Numeric character references | `Event::GeneralRef` rejected unconditionally | accept valid decimal/hex character references; reject invalid values | — | required Plan 056 numeric-reference fixtures | OPEN |
| Custom named entities | no bounded distinction because all references fail | predefined only; unresolved/custom entities and DTD expansion remain unsupported | — | unknown-entity/DOCTYPE fixtures | OPEN |
| Merge Start attributes | raw `Attribute::value` is escaped again | decode/normalize semantic value, then escape exactly once | — | ampersand/numeric attribute fixtures | OPEN |
| Merge Empty attributes | raw `Attribute::value` is escaped again | same semantic decode-before-escape rule as Start | — | empty-element attribute fixture | OPEN |
| Owned nested RDF close | RDF `Description` end checked before `owned_depth` | owned-depth End handling takes precedence | — | nested RDF owned-subtree fixture | OPEN |
| Owned comments | comment emitted whenever output buffer exists | comments inside owned subtree suppressed | — | owned-comment fixture | OPEN |
| Owned processing instructions | PI emitted whenever output buffer exists | PIs inside owned subtree suppressed | — | owned-PI fixture | OPEN |
| Owned general references | reference path currently errors globally | inside owned subtree reference content cannot leak to preserved output | — | owned-reference fixture | OPEN |
| Public rewrite stability | prior three-round tests do not exercise XML references | namespace-aware semantic preservation across three public rewrite rounds | — | Plan 056 conformance fixtures | OPEN |

---

## Retained closed work

The following is not reopened by this plan without a focused failing fixture:

- checked JPEG structural analysis and exact entropy spans;
- JPEG malformed marker/segment rejection;
- shared canonical Huffman handling;
- restart-bearing JPEG unsupported classification;
- strict RIFF extent/chunk padding/final cursor validation;
- VP8X structure and feature validation;
- ANMF header field parsing, canvas bounds, flags, nested payload validation, and alpha derivation;
- animated WebP metadata rewrite preserving ANMF payloads;
- RDF-qualified preserved descriptions;
- namespace-URI plus local-name ownership matching;
- structural event-based RDF merge;
- byte-identical preserved-description deduplication;
- one-job CI and manual release policy.

---

## Required verification before completion

Record exact results and exact implementation SHA for:

```bash
cargo fmt --all -- --check
cargo test -p stegoeggo xmp --all-features
cargo test -p stegoeggo webp --all-features
cargo test -p stegoeggo --test conformance_container_tests --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo check -p stegoeggo --no-default-features
cargo test --workspace --exclude stegoeggo-fuzz --all-features
./scripts/check.sh
```

Exact-head GitHub Actions evidence must be recorded as PASS/FAIL only if independently observable for the exact implementation SHA. Otherwise record `UNAVAILABLE`.

---

## Closure rule

Plan 056 may be marked COMPLETE only after every OPEN row above has focused source/test evidence and the required verification set is recorded against the actual implementation head.

Roadmap 045 and inherited Plans 051-055 must remain PARTIAL/REOPENED for final XMP closure until Plan 056 is complete. Their already-closed JPEG and animated-WebP sub-work remains accepted.
