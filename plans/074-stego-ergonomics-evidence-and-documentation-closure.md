# Plan 074: Stego Ergonomics Evidence and Documentation Closure

Status: Ready for implementation

Roadmap: `plans/069-stego-api-ergonomics-and-adapter-simplification-roadmap.md`

Depends on: Plans 070-073 complete.

Audited planning baseline: `main` at `f717fd5c99adf17be2bc94d315c71d18f8c77c3d`

Authoritative implementation ledger to create before product edits: `plans/074-status.md`

---

## 1. Purpose

Plans 070-073 change source organization and generic API ergonomics while intentionally preserving carrier algorithms and application semantics. This final plan performs an independent closure audit, reconciles architecture documentation, and ensures Roadmap 069 is not marked complete merely because implementation commits exist.

Plan 074 should contain little or no substantive product-code work. If significant behavior fixes are still required, keep Roadmap 069 `PARTIAL` and create a narrow corrective plan rather than hiding the work inside documentation closure.

---

## 2. Required closure questions

The audit must answer with source/test evidence:

1. Is `src/protected/steganography` actually decomposed by responsibility, or was the monolith simply moved to `mod.rs`?
2. Does current marker construction have one owner?
3. Does root carrier dispatch delegate mechanics only to `stegoeggo-stego`?
4. Is seed/extraction search separate from payload verification classification?
5. Is legacy behavior visibly isolated without duplicating current behavior?
6. Can an independent caller recover a framed LSB payload without original length?
7. Can an independent caller recover a framed JPEG payload without original length or retained actual redundancy?
8. Are framed searches bounded and fail safely on malformed length/header data?
9. Does in-place LSB embedding avoid a full-image clone?
10. Do cloning and in-place LSB paths share one corrected mutation implementation?
11. Are corrected embed/extract bit-vector intermediates gone?
12. Is deterministic corrected-LSB output unchanged?
13. Does a fallible redundancy/config path exist for both LSB and JPEG?
14. Are JPEG parser/coefficient/F5 internals still private?
15. Does the dedicated carrier README stand alone for generic users?
16. Did any dependency, feature, release, or CI complexity creep in without authorization?

---

# Phase 0 — create final status ledger

Create/track `plans/074-status.md` before closure edits.

Record:

- starting HEAD/working tree;
- implementation commit(s) for Plans 070-073 from their status ledgers;
- current root/carrier/CLI versions;
- root -> carrier dependency declaration;
- carrier direct dependency list;
- Roadmap 069 current status.

Start rows:

```text
R01 Plan 070 status/evidence independently verified
R02 Plan 071 status/evidence independently verified
R03 Plan 072 status/evidence independently verified
R04 Plan 073 status/evidence independently verified
R05 root application module responsibility audit passes
R06 carrier/private boundary audit passes
R07 raw generic API compatibility passes
R08 framed LSB external-consumer round-trip passes
R09 framed JPEG external-consumer auto-redundancy recovery passes
R10 malformed framed input boundedness tests pass
R11 in-place LSB no-clone contract evidenced
R12 corrected LSB known-answer output preserved
R13 legacy V1/V2 and tiled compatibility pass
R14 JPEG container/progressive/unsupported regressions pass
R15 carrier docs/doctests/package structural check pass
R16 architecture docs updated to final state
R17 README/examples terminology is accurate
R18 ./scripts/check.sh passes
R19 staged pre-release structural check passes if still part of repo policy
R20 no version/publish/tag/release/CI expansion occurred
R21 Roadmap 069 final disposition is truthful
```

---

# Phase 1 — inspect implementation ledgers skeptically

Read `070-status.md` through `073-status.md` and verify each claimed closure against current source/tests.

Do not treat a predecessor `COMPLETE` header as proof.

For each plan:

- verify implementation commit exists on current history;
- verify named tests/functions still exist;
- verify no later commit regressed the claimed boundary;
- mark any stale/inaccurate evidence in `074-status.md`.

If a residual is small and documentation-only, correct it here. If it requires product behavior changes, Roadmap remains `PARTIAL` and a new corrective implementation plan is required.

---

# Phase 2 — source-boundary audit

Expected high-level source architecture:

```text
root stego application adapter
    marker preparation
    embedding dispatch
    extraction/search
    verification classification
    legacy compatibility
        |
        v
stegoeggo-stego public operation facade
    lsb raw/framed/in-place
    jpeg raw/framed
    frame
        |
        v
private carrier mechanics
    lsb_internal
    jpeg_transcoder / entropy / F5
```

Audit imports and visibility.

Required findings:

- root has no duplicated carrier permutation/F5 implementation;
- root has no direct `JpegHeader`, coefficient, entropy, or F5 type imports;
- `application_support` is used only where operation-level parent integration requires it;
- default generic facade does not re-export application-support types;
- no generalized trait/session layer was introduced.

---

# Phase 3 — end-to-end generic consumer tests

Ensure tests exercise public APIs from outside the implementing module/crate.

Required scenarios:

### 3.1 Raw LSB

```text
caller knows payload length
embed -> extract -> equal
```

### 3.2 Framed LSB

```text
caller retains only resulting image + config
extract_framed -> original payload
```

No use of original payload length during extraction.

### 3.3 In-place LSB

```text
mutable image buffer address/ownership retained
embed_in_place mutates same image
extract -> equal
```

Evidence should prove no full returned image is allocated by API contract and source path.

### 3.4 Raw JPEG

```text
embed report provides actual redundancy
raw extract using explicit length + actual redundancy
```

### 3.5 Framed JPEG

```text
caller retains resulting JPEG + config only
extract_framed probes bounded possible redundancy
returns original payload
```

Include a case where capacity downgrades redundancy.

### 3.6 Malformed framed carrier

Test bad magic, bad version, CRC corruption, implausible declared length, wrong seed, and insufficient carrier capacity. No panic or unbounded allocation.

---

# Phase 4 — compatibility/regression audit

Run focused existing regressions for:

- legacy corrected/legacy LSB extraction;
- V1/V2 application payload fixtures;
- payload-v3 known-answer behavior;
- HMAC wrong-key behavior;
- tiled LSB crop cases;
- tiled JPEG wrong-first/later-valid behavior;
- tiled JPEG one-decode instrumentation;
- JPEG APP/COM/unknown segment preservation;
- progressive JPEG seed-only fallback;
- unsupported JPEG classification;
- PNG/WebP metadata preservation paths affected only indirectly by root decomposition.

Do not regenerate golden fixtures without proving why. A refactor should not require fixture churn.

---

# Phase 5 — documentation reconciliation

Update architecture docs that describe the old monolithic or pre-framed-convenience state.

At minimum inspect/update:

```text
architecture/overview.md
architecture/pipeline.md
architecture/protected-steganography.md
architecture/jpeg-stego-f5.md (only if public/private boundary text is stale)
README.md (only user-facing statements affected)
stegoeggo-stego/README.md
examples/generic_stego.rs
```

Required documentation model:

```text
StegoEggo is the rights-reservation/provenance application.
stegoeggo-stego is the application-neutral carrier library.
Raw carrier APIs are explicit/low-level.
Framed APIs add self-describing length + CRC convenience.
In-place LSB avoids clone when caller owns RgbaImage.
CRC is not authentication.
JPEG internals are private.
```

Do not describe hidden stego as a visible watermark.

---

# Phase 6 — package and verification evidence

Run at least:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo check -p stegoeggo --no-default-features
cargo test -p stegoeggo-stego
cargo test -p stegoeggo-stego --doc
cargo test -p stegoeggo --test public_stego_api --all-features
cargo test --workspace --exclude stegoeggo-fuzz --all-features
cargo package -p stegoeggo-stego --allow-dirty
./scripts/check.sh
```

If `scripts/release-check.sh --allow-dirty --skip-check --stage=pre` remains the documented local structural pre-release check, run it as evidence. It must not publish anything.

Record exact exit status/test counts in `074-status.md`.

---

# Phase 7 — final roadmap reconciliation

Only after all rows are closed:

- set Roadmap 069 `Status: COMPLETE`;
- add a concise closure note referencing Plan 074 evidence;
- do not rewrite Roadmap 057 history;
- do not claim publication/release completion.

If any required row remains open:

```text
Status: PARTIAL — <specific residual>
```

and create a narrowly scoped follow-up plan if needed.

---

## Final acceptance criteria

Plan 074 and Roadmap 069 are COMPLETE only when:

1. Plans 070-073 evidence is independently confirmed against current source.
2. Root stego application responsibilities are genuinely separated.
3. Generic raw/framed/in-place operations are externally usable as documented.
4. Framed recovery is self-describing and bounded.
5. LSB optimization preserves deterministic carrier behavior and compatibility.
6. Public config has a fallible validation path.
7. Carrier implementation internals remain private.
8. Documentation accurately distinguishes application semantics from generic carrier mechanics.
9. Full required checks pass.
10. No version bump, publication, tag, GitHub Release, or CI expansion was introduced.

If product-code defects are discovered during closure, do not bury them in this plan; mark the roadmap partial and hand them off explicitly.