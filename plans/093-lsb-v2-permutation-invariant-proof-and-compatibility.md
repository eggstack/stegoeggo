# Plan 093: LSB V2 Permutation Invariant Proof and Compatibility

Status: READY FOR IMPLEMENTATION

Parent roadmap: `plans/090-generic-carrier-v1-semantics-correctness-and-reuse-roadmap.md`

Audited baseline: `27bdd3d429d663948021de43d3e6f818fa613319`.

## 1. Objective

Resolve the discrepancy between the public/parent claim that the corrected LSB V2 carrier uses a true collision-free bijection and the actual bounded cycle-walking implementation, whose 256-step escape falls back to a mapping explicitly documented as not proven bijective.

The goal is not to replace the permutation for elegance. The goal is to establish a defensible invariant before freezing the carrier API while preserving all already-produced V2 carrier mappings and known-answer vectors.

## 2. Required mathematical invariant

For a fixed `seed` and accepted `slot_count > 0`, the carrier mapping used by an embedding operation must be total and injective over every logical index it may consume. If the intended claim remains “true bijection over `0..slot_count`,” then prove:

```text
P_seed,n : {0, ..., n-1} -> {0, ..., n-1}

for all accepted n and seeds:
  total(P)
  and i != j => P(i) != P(j)
```

A finite total injective map from a set to itself is bijective. Distribution quality is a separate property; chi-squared tests do not establish injectivity.

The underlying affine permutation over the next-power-of-two domain with odd multiplier is bijective. True cycle-walking of a permutation onto a subset is also a permutation when allowed to continue until re-entry. The audit concern is specifically the fixed 256-step escape to `splitmix64(x) % slot_count`, which is not justified by that argument.

NIST SP 800-38G may be cited as conceptual background for finite-domain permutation/cycle-walking terminology only. Do not claim that this carrier implements NIST FPE or inherits cryptographic security from that standard.

## 3. Compatibility constraint

Existing public tests contain LSB known-answer vectors, and protected images depend on the exact V2 slot mapping. Therefore:

- do not change `stego_permutation_v2` output for any previously reachable `(index, slot_count, seed)` pair without an explicit carrier-version migration;
- do not silently make a new permutation the default while calling it V2;
- preserve extraction of all existing V2 carriers;
- if a new mapping is required, version it explicitly and keep V2 readable.

## 4. Investigation phases

### Phase 0 — isolate and instrument the permutation

Move the permutation and directly related tests into a cohesive private module if useful (`lsb_internal/permutation.rs` or equivalent) without changing visibility or bytes.

Add test-only instrumentation that records cycle-walk depth before successful re-entry and whether the 256-step fallback is reached. Do not expose this instrumentation publicly.

Correct stale diagnostics that still refer to a 64-step cutoff.

### Phase 1 — define the accepted domain exactly

Record the maximum `slot_count` reachable from every public LSB API, including packed `RgbaImage`, tiled sub-images, and the generic buffer API planned by Plan 095. Account for:

- `u32` width/height;
- `width * height * 3` checked arithmetic;
- address-space/`usize` limits on 32-bit and 64-bit targets;
- resource limits imposed before allocation or iteration;
- logical indices actually consumed by `required_capacity`.

Do not prove a larger abstract domain than the API can represent unless that makes the proof simpler.

### Phase 2 — exhaustive/property evidence

Without adding a production dependency, build deterministic tests over broad small/medium domains that assert, per seed and slot count:

- every output is in range;
- every input in the tested domain produces exactly one output;
- no two logical indices collide;
- full-domain output cardinality equals `slot_count`;
- embed/extract round-trips at exact capacity;
- maximum observed cycle-walk depth is recorded.

Use exhaustive enumeration where tractable and deterministic sampled/property-style sweeps for larger domains. The status ledger must record the tested ranges and seeds, not just “property tests pass.”

### Phase 3 — analytic disposition of the cutoff

Choose exactly one evidence-backed disposition.

#### Disposition A — cutoff unreachable in the accepted domain

If it can be proven that true cycle-walking always re-enters before 256 steps for every accepted `slot_count`, seed, and logical index, document the proof/derivation and add a regression test or exhaustive bound check sufficient to guard implementation drift.

Then remove or make unreachable the non-bijective fallback without changing any reachable carrier output. Preserve boundedness with an analytically justified maximum rather than an arbitrary escape.

#### Disposition B — cutoff reachable but fallback itself is provably collision-free

If the fallback can be proven injective over exactly the subset of states that reach it, document that proof and add direct regression coverage. Do not infer this from distribution tests.

#### Disposition C — current V2 cannot support the claimed invariant

If neither A nor B is defensible, stop claiming V2 is a true bijection. Preserve V2 unchanged for compatibility and introduce a separately versioned mapping only after designing how callers select/read it.

A new mapping must not make generic raw extraction ambiguous. At minimum the configuration must carry an explicit carrier mapping version; framed data may additionally encode the mapping version in a future frame version only through a separately documented wire-format decision. The StegoEggo parent must continue probing/reading V2 for existing images.

Do not implement a new algorithm in this plan until the compatibility design is recorded in `plans/093-status.md`.

### Phase 4 — align capacity claims with the proven property

`lsb_required_capacity_v2` assumes each logical replica consumes an independent slot. After the disposition, make documentation and invariants exact:

- if V2 is proven injective, state why exact capacity implies no inter-replica collision;
- if not, stop calling the capacity model exact for V2 and ensure any new mapping restores an exact collision-free model.

Audit `src/protected/steganography/embed.rs`, `architecture/protected-steganography.md`, carrier README/rustdoc, and comments for the phrase “true bijection,” “distinct slots,” and equivalent claims.

## 5. Required regression tests

Preserve all current known-answer vectors. Add tests for:

- per-seed full-domain injectivity for a documented exhaustive small-domain range;
- non-power-of-two sizes adjacent to powers of two, where cycle-walking is most relevant;
- pathological small domains (`2`, `3`, `5`, etc.);
- representative large dimensions and exact-capacity embed/extract;
- zero and `u64::MAX` seeds;
- cycle-walk depth instrumentation, including an explicit assertion about whether the fallback is reachable under tested domains;
- 32-bit arithmetic behavior via checked-unit tests even if CI target execution is unavailable;
- `RgbaImage` and, after Plan 095, generic pixel-view equivalence.

If a new mapping version is introduced, add cross-version fixtures proving V2 extraction remains byte-compatible and the mapping version cannot be confused silently.

## 6. Acceptance criteria

- The repository no longer simultaneously claims a true V2 bijection while containing an unproven collision-producing escape path.
- The exact accepted permutation domain is documented.
- The 256-step fallback has an evidence-backed disposition A/B/C recorded in the status ledger.
- Existing V2 known-answer bytes and extraction compatibility are preserved.
- Exact-capacity/no-collision claims are backed by proof plus targeted regression evidence, not only statistical tests.
- No cryptographic-security/FPE claim is added.
- No production dependency is added solely for permutation testing.
- `cargo test -p stegoeggo-stego`, known-answer/public API suites, parent semantic-correctness suites, Clippy, and `./scripts/check.sh` pass.

## 7. Non-goals

No attempt to make the seed cryptographically secret, no AES/FF1 adoption, no steganalysis-resistance claim, no frame-format change unless Disposition C requires a separately approved versioning decision, and no change to legacy V1 carrier extraction.
