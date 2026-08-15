# Plan 072: LSB In-Place and Bitstream Allocation Optimization

Status: Ready for implementation

Roadmap: `plans/069-stego-api-ergonomics-and-adapter-simplification-roadmap.md`

Depends on: Plan 071 complete.

Audited planning baseline: `main` at `f717fd5c99adf17be2bc94d315c71d18f8c77c3d`

Authoritative implementation ledger to create before product edits: `plans/072-status.md`

---

## 1. Purpose

The corrected LSB carrier is already algorithmically sound, but its convenience implementation still pays two avoidable allocation costs:

1. public `lsb::embed(&RgbaImage, ...)` returns an owned image and therefore clones the full image even when a caller already owns a mutable buffer;
2. corrected LSB embedding/extraction converts payload bytes to `Vec<u8>` bits and later converts bit vectors back to bytes, using one byte of heap storage per logical bit.

This plan removes those costs without changing the corrected carrier mapping, deterministic output, capacity calculation, legacy extraction compatibility, or tiled behavior.

Performance is secondary to exact behavioral equivalence.

---

## 2. Required end state

Public API retains the convenient cloning operation and adds a mutation-oriented path, conceptually:

```rust
pub fn embed(
    image: &RgbaImage,
    payload: &[u8],
    config: &LsbConfig,
) -> Result<EmbedReport<RgbaImage>, StegoError>;

pub fn embed_in_place(
    image: &mut RgbaImage,
    payload: &[u8],
    config: &LsbConfig,
) -> Result<InPlaceEmbedReport, StegoError>;
```

Exact report type may differ, but in-place callers must receive at least:

- embedded/skipped status;
- payload bytes;
- required capacity;
- available capacity;
- actual redundancy.

The cloning `embed()` should become:

```text
clone image once
    -> call shared in-place core
    -> package cloned image + summary into existing EmbedReport
```

There must be one corrected LSB mutation algorithm, not parallel clone and in-place implementations.

---

## 3. Frozen carrier behavior

Do not change:

- corrected V2 slot permutation;
- seed use;
- RGB-only carrier channels;
- alpha preservation;
- spread factor;
- redundancy semantics;
- capacity formulas;
- bit ordering (least-significant bit first as currently encoded);
- deterministic direction choice for ±1 channel mutation;
- tiled seed derivation;
- legacy mapping or legacy extraction;
- public raw/framed semantics from Plan 071.

For a fixed input image, payload, seed, and redundancy, the cloning `embed()` output after Plan 072 must be byte-for-byte identical in pixel data to the pre-Plan-072 corrected implementation.

---

# Phase 0 — ledger and known-answer lock

Create/track `plans/072-status.md` first.

Before rewriting the hot path, add or identify deterministic known-answer vectors for corrected non-tiled LSB behavior covering:

- payload with mixed zero/one bits;
- pixel values containing 0 and 255 channel boundaries;
- non-power-of-two dimensions;
- redundancy 1 and >1;
- exact-capacity or near-capacity case;
- alpha values that are not all 255 to prove alpha is untouched.

Record baseline output digests or explicit modified pixel vectors in tests before optimization.

Start rows:

```text
R01 deterministic corrected-LSB known-answer behavior locked
R02 public in-place embed added without full-image clone
R03 existing embed delegates to shared in-place core
R04 no normal corrected embed intermediate bit Vec
R05 no normal corrected extract intermediate bit Vec
R06 bit ordering unchanged
R07 alpha preservation unchanged
R08 capacity/redundancy semantics unchanged
R09 legacy compatibility unchanged
R10 tiled LSB behavior unchanged
R11 framed APIs from Plan 071 pass unchanged
R12 allocation/performance evidence recorded
R13 carrier tests/doctests pass
R14 full workspace checks pass
```

---

# Phase 1 — define the in-place result contract

Do not force `EmbedReport<()>` on users merely because it avoids a new type unless rustdoc/examples show that this is genuinely clear.

Preferred small type:

```rust
pub struct InPlaceEmbedReport {
    pub embedded: bool,
    pub payload_bytes: usize,
    pub required_capacity: usize,
    pub available_capacity: usize,
    pub actual_redundancy: usize,
}
```

Naming may be `EmbedStats`, `EmbedSummary`, or similar if an existing public result type already exactly represents these fields.

Before adding a new type, audit `EmbedOutcomeSummary` and existing report types. Reuse an existing type only if doing so does not lose `actual_redundancy` or create misleading path/application semantics.

Avoid a generic report trait.

Acceptance:

- in-place caller gets all information needed to know whether bytes were embedded and at what redundancy;
- no output-image field is present because the caller already owns it;
- invalid configuration behavior follows current public contract until Plan 073 hardens it.

---

# Phase 2 — extract one shared corrected LSB mutation core

Refactor corrected non-tiled V2 embedding so both public forms call one private mutation function.

Preferred shape:

```rust
fn embed_v2_in_place_core(
    image: &mut RgbaImage,
    payload: &[u8],
    seed: u64,
    redundancy: usize,
) -> CoreEmbedStats
```

The function should:

1. calculate exact existing capacity;
2. return skipped-capacity without modifying image if insufficient;
3. iterate payload bits in the existing order;
4. map logical replicas through the existing permutation;
5. mutate the selected RGB channel using the existing bounded ±1 rule;
6. never touch alpha;
7. return summary data.

Public `embed()` clones before calling this core. Public `embed_in_place()` calls it directly.

Important atomicity requirement:

- capacity/config validation must occur before the first mutation;
- a normal deterministic capacity failure leaves the caller's in-place image unchanged.

If any later operation can fail after mutation, either prove it cannot fail or stage enough state to preserve this contract. Do not return an error after partially mutating the caller image without documenting and explicitly approving that semantic change.

---

# Phase 3 — remove embedding bit-vector allocation

Current mechanics use `bytes_to_bits(payload)` for the corrected path.

Replace with direct bit access while preserving ordering:

```rust
bit_index -> payload[bit_index / 8] -> (byte >> (bit_index % 8)) & 1
```

Use checked arithmetic where payload length multiplication can overflow, consistent with current capacity hardening.

Do not delete generic/private byte-bit helpers yet if legacy/tiled/tests still use them. Delete only dead helpers after source audit.

Tests must prove exact known-answer carrier output remains unchanged.

---

# Phase 4 — remove extraction bit-vector allocation

Corrected extraction should allocate final bytes directly:

```text
output = vec![0u8; payload_len]
for logical bit:
    majority vote existing replicas
    if one: output[byte_index] |= 1 << bit_offset
```

Do not change majority threshold or replica selection.

When raw API supplies payload length in bytes, calculate expected bits using checked multiplication.

For prefix/range helpers needed by framed APIs, avoid extracting substantially more bytes than requested merely for implementation convenience.

Acceptance:

- corrected extraction output is identical for all existing tests;
- frame prefix extraction remains efficient and bounded;
- no full `Vec<u8>` of one element per bit exists in normal corrected extraction.

---

# Phase 5 — tiled-path audit

Roadmap 057 already removed full temporary tile-image allocation for tiled **embedding**. Do not rewrite tiled math unnecessarily.

Audit whether the new shared bit accessor/writer can be reused by tiled embedding/extraction without changing behavior. Reuse only if it materially deletes duplicate bit-vector logic.

Do not combine this plan with a new tiled search algorithm.

Any existing tiled extraction crop allocation not owned by the current optimization target should be measured before changing it. Avoid scope expansion.

---

# Phase 6 — application adapter adoption

Audit root `StegoEggo` usage of LSB carrier operations after Plan 070.

Where root already owns a mutable `RgbaImage` and currently calls a cloning generic helper only to replace the original with the returned clone, switch to the in-place operation.

Do not force in-place use where ownership/cow semantics make the cloning API clearer or where changing ownership would complicate error rollback.

Record each adoption site in the status ledger and explain whether one full-image clone was removed.

---

# Phase 7 — measured evidence

Record at least one representative local measurement for:

- 1024x1024 RGBA image, small payload;
- 4096x4096 RGBA image, small payload;
- payload large enough to exercise a meaningful number of slots.

Evidence can be Criterion timing plus a reasoned allocation audit. The acceptance requirement is not a fixed percentage speedup.

Required evidence:

1. `embed_in_place` performs no full-image clone by source/allocator inspection.
2. corrected embed/extract no longer allocate O(payload_bits) bytes solely for bit vectors.
3. existing cloning API still performs exactly the intentional image clone required by its ownership contract, not additional full-image clones.

Do not add benchmark thresholds to CI.

---

# Phase 8 — tests and docs

Add public example/rustdoc for in-place usage.

Focused verification:

```bash
cargo test -p stegoeggo-stego lsb
cargo test -p stegoeggo-stego frame
cargo test -p stegoeggo-stego --doc
cargo test -p stegoeggo --all-features stego
cargo test -p stegoeggo --all-features tiled
./scripts/check.sh
```

---

## Completion criteria

Plan 072 is COMPLETE only when:

- in-place public LSB embedding exists;
- cloning and in-place forms share one corrected mutation core;
- deterministic corrected carrier output remains unchanged;
- normal corrected embed/extract no longer materialize payload-bit vectors;
- legacy/tiled/framed compatibility remains passing;
- allocation evidence is recorded;
- `./scripts/check.sh` passes;
- no version/release/CI change occurs.