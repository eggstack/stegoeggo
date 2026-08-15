# Plan 067: Tiled-JPEG Candidate Integrity and Final Evidence Closure

Status: Ready for implementation

Roadmap: `plans/057-stego-carrier-library-and-pipeline-simplification-roadmap.md`

Corrective predecessor: `plans/066-post-065-final-closure-and-public-boundary-correction.md`

Audited implementation baseline: `main` at `eebb519098b58236558e1fcfc86750fea5fe900d`

Historical pre-boundary reference for tiled-JPEG behavior: `e694209e10b46cc8f13727b220467399371c9c97`

Authoritative implementation ledger to create as the first implementation change: `plans/067-status.md`

---

## 1. Purpose

Plan 066 successfully corrected the major residual architecture issues from Roadmap 057:

- legacy `Light` now resolves to `HiddenMarkerMode::SeedOnly` even when a legacy context carries a non-zero `tile_size`;
- `ProtectionPipeline` is a stateless compatibility adapter into the canonical request/plan executor;
- the normal `stegoeggo-stego` API no longer exposes JPEG parser/coefficient/F5 internals or low-level LSB algorithm primitives;
- the parent `stegoeggo` crate consumes a feature-gated operation-level `application-support` layer rather than codec structs;
- legal-claims documentation and the pixel-domain LSB API were reconciled;
- release checking now models the manual carrier -> root -> CLI publication dependency order.

A post-Plan-066 audit found one important correctness regression introduced while moving tiled-JPEG mechanics behind the carrier boundary, plus a small set of evidence/test-design inaccuracies.

This plan closes only those residuals:

1. preserve tiled-JPEG candidate identity across prefix -> header -> full-payload extraction so a later valid grid/seed/redundancy candidate cannot be confused with an earlier invalid candidate;
2. ensure a wrong early `NotV3`, malformed, unsupported-version, or authentication candidate does not prematurely terminate the search before later candidates are tried;
3. restore a truthful tiled-JPEG post-encode success check so `EmbedOutcome::Embedded` means the encoded output actually yields the payload from the tile/candidate that was written;
4. replace the duplicated integration-test implementation of `request_from_legacy()` with tests that exercise the real private production adapter;
5. correct the remaining Plan 063/065/066/064/Roadmap 057 evidence inaccuracies and only then make the final Roadmap 057 COMPLETE claim.

This is intended to be the final corrective pass for Roadmap 057. Do not use it as a vehicle for broader cleanup.

---

## 2. Required end state

At completion, tiled-JPEG extraction must have this semantic shape:

```text
encoded JPEG
    |
    v
carrier enumerates bounded candidate identities
    |
    +-- candidate A = exact tile/grid-seed/redundancy identity
    |       |
    |       +-- extract prefix A
    |       +-- if V3: extract header A -> extract full payload A
    |       +-- if legacy: extract legacy lengths using A
    |       +-- invalid A => continue
    |
    +-- candidate B = exact tile/grid-seed/redundancy identity
    |       |
    |       +-- prefix/header/full extraction all remain B
    |       +-- valid B => success
    |
    ... bounded by max origins / supported redundancy
```

It must **not** behave like:

```text
all prefixes flattened into Vec<Vec<u8>>
    |
    +-- classify prefix from candidate B
    |
    +-- independently regenerate all header candidates
    |
    +-- validate first header candidate A
    |
    +-- return early on unrelated candidate A
```

The carrier must continue hiding:

```text
JpegHeader
Coefficients
JpegTranscoder
DctStegoF5
Huffman/parser state
raw block-set implementation details
```

The parent crate may receive a small opaque/simple candidate identity from the feature-gated `application-support` API, but it must not receive codec internals.

---

## 3. Retained work — do not reopen

Retain the following unless a focused regression test proves a specific defect:

- corrected V2 LSB slot/permutation model;
- legacy LSB extraction compatibility;
- bounded LSB channel mutation at `0` and `255`;
- public pixel-domain `EmbedReport<RgbaImage>` LSB boundary;
- normal one-pass non-tiled JPEG embedding from Plan 060;
- direct-region tiled LSB implementation;
- `HiddenMarkerMode::SeedOnly` and the current Light mapping;
- canonical `ProtectionRequest -> ResolvedProtectionPlan -> execution` architecture;
- stateless `ProtectionPipeline`;
- `stegoeggo-stego` workspace crate split;
- small default carrier API (`lsb`, `jpeg`, `frame` plus small reports/errors);
- private `jpeg_transcoder` / `lsb_internal` modules;
- feature-gated `application-support` concept for the parent crate;
- exact root -> carrier version wiring;
- staged local release-check concept;
- manual release cadence and existing CI simplification.

Do not reopen metadata vocabulary, PLUS/DMI semantics, payload-v3 format, WebP/XMP container work, provenance/signatures, F5 mathematics, or release automation.

---

## 4. Explicit non-goals

Plan 067 must not:

- add another crate;
- add a payload version;
- change the F5 algorithm;
- change LSB spread/redundancy math;
- add a generic carrier trait hierarchy;
- expose JPEG parser/coefficient types again;
- add a new default-public carrier module;
- redesign the generic frame API;
- redesign resource accounting;
- modify the unrelated conformance/XMP/WebP/payload-flag work from commit `e694209` unless a direct Plan 067 compile/test dependency requires a mechanical adjustment;
- expand CI or add release automation;
- bump package versions;
- publish a crate;
- create a tag or GitHub Release.

If implementation discovers an unrelated defect, record it separately. Do not fold it into this closure pass.

---

# Phase 0 — create a truthful Plan 067 ledger and reopen Roadmap 057

## 0.1 `plans/067-status.md` must be the first implementation artifact

Before any product-source edits, create `plans/067-status.md` and force-track it because `plans/` is ignored.

Required local commands:

```bash
git add -f plans/067-status.md
git ls-files --error-unmatch plans/067-status.md
git status --short
```

Record the actual:

```text
starting HEAD
working-tree status
root/carrier/CLI versions
root -> carrier dependency declaration
Roadmap 057 status
Plan 066 status
```

Do not repeat Plan 066's evidence mistake. If source edits occur before the ledger is tracked, state that fact honestly; do not claim the ledger predated those edits.

## 0.2 Initial status rows

Start all rows as `OPEN`:

```text
R01 tiled JPEG candidate identity preserved across prefix/header/full extraction
R02 wrong early NotV3 candidate does not terminate later V3 search
R03 malformed/unsupported/auth-failed candidate does not mask later valid candidate
R04 legacy tiled-JPEG candidate probing remains bounded and compatible
R05 max_origins semantics remain bounded at tile-origin level
R06 tiled JPEG Embedded outcome survives post-encode extraction
R07 failed tiled JPEG post-encode verification is not reported Embedded
R08 carrier internals remain inaccessible from default public API
R09 root application uses only operation-level carrier support
R10 production request_from_legacy is directly tested in crate-local tests
R11 duplicated integration-test legacy request builder removed or reduced to public-behavior-only use
R12 Plan 063 public API inventory matches current source
R13 Plan 065 implementation commits/evidence are reconciled
R14 Plan 066 ledger chronology is corrected truthfully
R15 Plan 064 final supersession note points to Plan 067
R16 Roadmap 057 is PARTIAL while Plan 067 is open
R17 focused tiled-JPEG regression matrix passes
R18 focused legacy-adapter matrix passes
R19 carrier boundary/doctest checks pass
R20 ./scripts/check.sh passes
R21 staged pre-release structural check passes
R22 final Roadmap 057 evidence is internally consistent
```

## 0.3 Reopen Roadmap 057 during implementation

Change its status to:

```text
Status: PARTIAL — final tiled-JPEG/evidence residuals tracked by Plan 067
```

Do not erase the historical Plan 064/065/066 completion attempts. Add concise supersession/correction notes instead.

### Phase 0 acceptance criteria

- `plans/067-status.md` is tracked before product edits;
- baseline facts are actual facts, not reconstructed claims;
- all rows begin OPEN;
- Roadmap 057 does not claim COMPLETE while Plan 067 is open.

---

# Phase 1 — lock the tiled-JPEG compatibility contract before changing the support API

## 1.1 Use the pre-Plan-066 implementation as behavioral evidence

Inspect `src/protected/steganography.rs` at `e694209e10b46cc8f13727b220467399371c9c97`.

The important historical property is not the specific placement of JPEG internals in the root crate. The important property is **candidate continuity**:

```text
for tile origin
  for nearby grid seed coordinate
    for redundancy
      extract prefix using this exact candidate
      if V3:
        extract header using this exact candidate
        extract full payload using this exact candidate
      if legacy:
        extract legacy payload using this exact candidate
      if invalid:
        continue to next candidate
```

The current Plan-066 support API loses this property by returning flattened `Vec<Vec<u8>>` results from separate calls for prefix/header/full lengths.

## 1.2 Required search semantics

For the ordinary extraction API (`Option<Vec<u8>>`):

- invalid candidate -> continue;
- `NotV3` candidate -> try legacy lengths for **that candidate**, then continue if legacy decoding fails;
- malformed V3 candidate -> continue;
- unsupported version candidate -> continue;
- failed integrity/authentication candidate -> continue;
- valid candidate -> return success;
- end of bounded search with no valid candidate -> `None`.

For the verification tri-state path:

- valid candidate wins immediately;
- invalid/malformed/unsupported/auth-failed candidates may update `last_outcome`;
- scanning continues while bounded candidates remain;
- only after candidate exhaustion does the path return the best/first recorded failure outcome;
- `NotFound` means no candidate produced a stronger classification.

Do not reintroduce unbounded search.

## 1.3 Preserve origin-bound semantics

`max_origins` limits tile origins, not the number of redundancy or nearby-seed variants within an already admitted origin.

Retain the existing bounded neighborhood and redundancy ranges unless the current source proves they differ from the pre-066 contract:

```text
nearby grid offset: 0..=2 in x/y
redundancy: 1..=10
```

Do not increase these ranges in Plan 067.

### Phase 1 acceptance criteria

- the compatibility contract is written into `067-status.md` before implementation;
- no implementation decision depends on flattened candidate ordering;
- search remains bounded by existing origin and redundancy limits.

---

# Phase 2 — replace flattened tiled-JPEG bytes with candidate-identity-preserving support

This is the central corrective change.

## 2.1 Do not expose codec internals

Keep:

```rust
pub(crate) mod jpeg_transcoder;
pub(crate) mod lsb_internal;
```

Do not restore `__internal_jpeg_facade` or `__internal_lsb_facade`.

The root crate must not import:

```text
JpegHeader
Coefficients
JpegTranscoder
DctStegoF5
Huffman decoder/encoder structs
raw JPEG block vectors
```

## 2.2 Preferred support shape: simple opaque candidate key

Replace or supersede the current flattened operation:

```rust
jpeg_extract_tiled_candidates(..., payload_bits) -> Option<Vec<Vec<u8>>>
```

with an identity-preserving operation-level API under the existing feature-gated `application_support` module.

Preferred shape:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TiledJpegCandidateKey {
    // private fields
    tile_x: u32,
    tile_y: u32,
    seed_x: u32,
    seed_y: u32,
    redundancy: u8,
}

pub fn jpeg_tiled_prefix_candidates(
    jpeg_bytes: &[u8],
    master_seed: u64,
    tile_size: u32,
    max_origins: u32,
    prefix_bits: usize,
) -> Option<Vec<(TiledJpegCandidateKey, Vec<u8>)>>;

pub fn jpeg_tiled_extract_candidate(
    jpeg_bytes: &[u8],
    master_seed: u64,
    tile_size: u32,
    candidate: TiledJpegCandidateKey,
    payload_bits: usize,
) -> Option<Vec<u8>>;
```

Exact names may vary, but the semantics must not.

The key is intentionally simple/opaque:

- it contains only primitive tile/grid/redundancy identity;
- fields should remain private unless a test/debug reason requires read-only access;
- it exposes no JPEG parser, coefficient, Huffman, F5, or block-set object;
- callers may pass it back to the carrier to request a different extraction length for the **same** candidate.

If a smaller implementation can preserve identity with a different equally narrow operation-level shape, that is acceptable. Do not build a generalized iterator/session abstraction unless clearly simpler than the key approach.

## 2.3 Candidate enumeration rules

`jpeg_tiled_prefix_candidates` should:

1. decode the JPEG once for that enumeration call;
2. walk tile origins in the established order;
3. increment `origins_tried` once per admitted tile origin;
4. for each admitted origin, enumerate the existing nearby grid-seed coordinates;
5. for each seed coordinate, enumerate redundancy 1..=10;
6. extract exactly `prefix_bits` for that candidate;
7. return `(candidate_key, bytes)` for candidates with enough extracted bits;
8. dedup only if doing so cannot destroy candidate identity. Prefer **no byte-value deduplication** at this layer.

Do not collapse different keys merely because their current prefix bytes are equal. Equal prefixes can later produce different headers/full payloads.

## 2.4 Exact-candidate extraction rules

`jpeg_tiled_extract_candidate` should:

1. validate `tile_size` and candidate coordinates defensively;
2. decode the JPEG;
3. rebuild the exact block set for `candidate.tile_x/tile_y`;
4. derive the exact local seed represented by `candidate.seed_x/seed_y` and `master_seed`;
5. use the exact redundancy in the candidate key;
6. extract `payload_bits` only from that identity;
7. return bytes or `None`.

Do not silently fall back to another candidate inside this function.

## 2.5 Keep operation-level support narrow

While touching `application_support.rs`, do not expand its primitive helper surface.

If `bits_to_bytes`, raw `corrected_lsb_*`, crop helpers, etc. are currently needed by the root, leave them unless removing them is trivial and directly enabled by this patch. Do not turn Plan 067 into another API-boundary redesign.

The blocking requirement is that the **default** carrier API remains clean and tiled-JPEG identity is preserved.

### Phase 2 acceptance criteria

- prefix/header/full extraction can be tied to one exact candidate;
- different candidate identities are never deduplicated solely because prefix bytes match;
- root cannot name JPEG parser/coefficient/F5 internals;
- default carrier API surface does not expand;
- support remains feature-gated under `application-support`.

---

# Phase 3 — rewrite root tiled-JPEG evaluation around exact candidate identity

Update `SteganographyProtector::extract_f5_tiled_candidates()` and `verify_extract_f5_tiled()` to consume the identity-preserving support API.

## 3.1 Ordinary extraction pseudocode

Required semantic structure:

```rust
let candidates = carrier_support::jpeg_tiled_prefix_candidates(...)?;

for (key, prefix) in candidates {
    match Self::classify_v3_prefix(&prefix, Some(&self.limits)) {
        V3PrefixResult::Detected { header_length, total_length } => {
            let Some(header) = carrier_support::jpeg_tiled_extract_candidate(
                jpeg_bytes,
                master_seed,
                tile_size,
                key,
                header_length * 8,
            ) else {
                continue;
            };

            if Self::validate_v3_header(&header, Some(&self.limits)).is_err() {
                continue;
            }

            let Some(full) = carrier_support::jpeg_tiled_extract_candidate(
                jpeg_bytes,
                master_seed,
                tile_size,
                key,
                total_length * 8,
            ) else {
                continue;
            };

            if Self::verify_payload_integrity(&full, mac_key) {
                return Some(Self::truncate_to_actual_payload(&full));
            }

            continue;
        }

        V3PrefixResult::NotV3 => {
            for bits in [ECC_PAYLOAD_BITS_V2, ECC_PAYLOAD_BITS] {
                let Some(payload) = carrier_support::jpeg_tiled_extract_candidate(
                    jpeg_bytes,
                    master_seed,
                    tile_size,
                    key,
                    bits,
                ) else {
                    continue;
                };

                if Self::try_ecc_decode(&payload).is_some() {
                    return Some(payload);
                }
            }

            // Critical: do NOT return None here.
            continue;
        }

        V3PrefixResult::Malformed(_)
        | V3PrefixResult::UnsupportedVersion(_)
        | V3PrefixResult::ResourceLimitExceeded => {
            // Critical: one bad candidate must not mask later candidates.
            continue;
        }
    }
}

None
```

## 3.2 Verification-path pseudocode

Keep candidate identity and retain failure classification:

```rust
let mut last_outcome = None;

for (key, prefix) in candidates {
    match classify(prefix) {
        valid-v3-prefix => {
            // exact key for header and full payload
            // valid payload => return Valid
            // failed auth/integrity => record classify_auth_failure if no stronger result yet
        }
        NotV3 => {
            // exact key for legacy lengths
            // valid legacy => return Valid
            // invalid => record classification if useful, then continue
        }
        Malformed => record MalformedV3 if appropriate; continue,
        UnsupportedVersion(v) => record UnsupportedVersion(v) if appropriate; continue,
        ResourceLimitExceeded => record bounded malformed/resource outcome; continue,
    }
}

last_outcome.unwrap_or(CandidateOutcome::NotFound)
```

Do not return a failure outcome while untried bounded candidates remain.

## 3.3 Avoid repeated semantic drift between the two root functions

The ordinary and verification paths can share a small private candidate-evaluation helper if doing so clearly reduces duplicated prefix/header/full orchestration.

Do not introduce a public abstraction or trait merely to share this code.

### Phase 3 acceptance criteria

- exact candidate key is reused for V3 prefix, header, and full payload;
- `NotV3` never terminates the whole bounded search by itself;
- malformed/unsupported/auth-failed candidate never masks a later valid candidate;
- legacy candidate probing remains available;
- verification path retains meaningful failure classification after exhaustion.

---

# Phase 4 — restore truthful tiled-JPEG embed success semantics

The Plan-066 move into `application_support::jpeg_embed_tiled()` dropped the post-encode payload verification that the root implementation still had at `e694209`.

Restore this check **inside the carrier**, where JPEG internals are already private and available.

## 4.1 Track the tile actually used

During tiled embedding, track the first successful embedded tile identity rather than using a generic `embedded_any` boolean only.

Preferred local state:

```rust
let mut first_embedded: Option<(u32, u32, u64)> = None;
```

When an `embed_f5_in_blocks()` call succeeds:

```rust
first_embedded.get_or_insert((tx, ty, local_seed));
```

Continue embedding other tiles as the existing algorithm does; do not change the redundancy strategy.

## 4.2 Encode once, then verify the encoded result

After the normal tiled modifications:

1. encode the JPEG once;
2. if no tile embedded, return the established skipped-capacity outcome;
3. decode coefficients from the **encoded output**;
4. rebuild the block set for the recorded successful tile using the decoded header/coefficient state;
5. extract exactly the payload bits from that tile using redundancy 1 and the recorded local seed;
6. convert to bytes and compare to the original payload;
7. only report `EmbedOutcome::Embedded` if the payload matches;
8. otherwise report the established non-success outcome (normally `SkippedCapacity`) rather than claiming Embedded.

Do not add repeated retry/re-encode loops.

The successful path should therefore remain:

```text
one coefficient decode
one modification pass
one encode
one verification decode
one extraction from the recorded tile
```

This exception is specifically for tiled JPEG because its established contract before Plan 066 included post-encode verification. Do not restore round-trip verification to the normal non-tiled JPEG path.

## 4.3 Preserve seed hint behavior

Retain quantization-table seed embedding behavior. Do not change progressive/unsupported fallback semantics in this plan.

### Phase 4 acceptance criteria

- tiled JPEG cannot report `Embedded` solely because in-memory coefficient modification returned `Ok`;
- verification is performed against the actual encoded output;
- verification uses the same tile/local seed that was recorded as successfully embedded;
- there is no multi-attempt encode loop;
- non-tiled JPEG code remains untouched except mechanical API compatibility if required.

---

# Phase 5 — add regression tests that specifically exercise wrong-first/later-valid candidates

Existing uncropped tiled-JPEG tests are insufficient because candidate zero is normally valid. The new regression must prove search continuation and identity preservation.

## 5.1 Carrier-level controlled candidate fixture

Inside `stegoeggo-stego` tests, use private codec/F5 access to construct a deterministic fixture where:

- the physical tile/block set is known;
- the payload is embedded with a nearby grid-derived seed that is **not the first seed candidate** the extractor will try;
- e.g. tile `(0,0)` blocks use `tile_seed(master_seed, 1, 0)` while candidate enumeration first tries the `(0,0)` seed identity;
- the first prefix candidate is therefore wrong/non-V3 or otherwise invalid;
- a later candidate is valid.

This is a test-only fixture. Do not expose a fixture-builder API publicly.

Required tests:

```text
tiled_jpeg_candidate_key_roundtrips_same_identity

tiled_jpeg_wrong_first_candidate_does_not_mask_later_v3

tiled_jpeg_equal_prefix_bytes_do_not_collapse_distinct_candidate_keys

tiled_jpeg_candidate_extraction_uses_requested_redundancy

tiled_jpeg_max_origins_still_bounds_origin_scan
```

The equal-prefix test may use a small internal enumeration helper if constructing naturally equal prefixes is inconvenient. The point is to prove the support layer does not dedup by bytes and lose identity.

## 5.2 Root extraction regression

Add at least one focused root-unit regression using the production tiled-JPEG evaluation logic where the first candidate is invalid and a later candidate is valid.

Preferred approach:

- put the test in `src/protected/steganography.rs` so it can use crate-private helpers;
- reuse a deterministic carrier test fixture mechanism only if it can remain non-public, or factor the root candidate-evaluation loop into a small private function whose candidate fetch operations can be controlled in a unit test;
- avoid adding a production-public test seam.

Required assertions:

```text
later valid V3 candidate is recovered
wrong first NotV3 candidate does not return None early
verification path returns Valid for later candidate
wrong MAC still does not verify
```

## 5.3 Embed verification tests

Required tests:

```text
tiled_jpeg_embed_reports_embedded_only_after_encoded_roundtrip

tiled_jpeg_embed_roundtrip_uses_recorded_successful_tile

tiled_jpeg_failed_roundtrip_is_not_reported_embedded
```

For the failure test, use a test-only internal seam/helper if necessary to corrupt or replace the encoded coefficient state between encode and verification. Do not add a public fault-injection hook.

## 5.4 Keep existing tests

Do not delete the existing:

```text
embed_f5_tiled_round_trip_no_crop
embed_f5_tiled_with_mac_key
embed_f5_tiled_max_origins_limits_scan
```

Update them only as required by the support API.

### Phase 5 acceptance criteria

- at least one deterministic test fails on the Plan-066 flattened-candidate implementation and passes after the fix;
- tests prove later-valid behavior rather than only the first `(0,0)` candidate;
- post-encode success semantics are directly tested;
- no public test hook is introduced.

---

# Phase 6 — test the real `request_from_legacy()` instead of a duplicated test implementation

Plan 066 correctly removed the production `#[doc(hidden)] _plan065_internal_request_from_legacy` hook. However, `tests/plan065_legacy_compat.rs` replaced it with `plan065_compat_helpers::build_request()`, which duplicates the production translation logic.

That weakens the evidence: a mapping regression in production can be mirrored or missed by the test helper.

## 6.1 Move translation-specific assertions into crate-local unit tests

Because `request_from_legacy()` is private to `src/lib.rs`, add/retain tests in `src/lib.rs`'s `#[cfg(test)]` module that call the **actual function directly**.

Required direct-adapter tests:

```text
request_from_legacy_light_default_policy_is_unspecified
request_from_legacy_light_with_tile_size_is_seed_only
request_from_legacy_standard_with_tile_size_is_tiled
request_from_legacy_explicit_dmi_overrides_default
request_from_legacy_explicit_redundancy_preserved
request_from_legacy_content_hash_preserved
request_from_legacy_timestamp_override_preserved
request_from_legacy_legal_claims_none_auto_includes_supplied_metadata
request_from_legacy_legal_claims_false_excludes_claims
request_from_legacy_resource_limits_preserved
```

Assertions should inspect the actual returned `ProtectionRequest` and/or the `ResolvedProtectionPlan` derived from it.

## 6.2 Keep integration tests focused on public behavior

`tests/plan065_legacy_compat.rs` may remain for end-to-end public compatibility, but remove `plan065_compat_helpers::build_request()` as the source of translation assertions.

Good integration tests call public APIs such as:

```text
process_image_bytes(..., ProtectionLevel::Light, ctx)
process_image_bytes_with_warnings(...)
ProtectionPipeline::process_bytes(...)
verify_image_bytes_detailed(...)
```

and assert emitted behavior.

Do not recreate another public or hidden production test hook.

## 6.3 Avoid testing test code

No test should claim to prove production `request_from_legacy()` semantics when it actually calls a separately implemented request builder.

### Phase 6 acceptance criteria

- direct mapping tests invoke the actual private production adapter;
- duplicated `plan065_compat_helpers::build_request()` is removed, or retained only for a purpose that does not claim production equivalence;
- integration tests remain public-API behavior tests;
- no new public test seam exists.

---

# Phase 7 — reconcile planning records truthfully

This phase is small but mandatory because Roadmap 057 has had multiple premature COMPLETE claims.

## 7.1 Correct Plan 066 chronology

`plans/066-status.md` currently says:

```text
This ledger was created before Plan 066 product-source edits.
```

but the tracked file was added in closure commit `eebb519...`, after product commit `e694209...`.

Replace that claim with a truthful correction note, for example:

```text
Chronology correction added by Plan 067:
The tracked 066-status ledger was committed in the Plan 066 closure commit after
`e694209`; therefore it cannot serve as proof that a tracked ledger existed
before those product edits. Its implementation/test evidence remains historical
closure evidence, but the pre-edit chronology claim is withdrawn.
```

Do not fabricate an untracked/local timestamp.

Because Plan 067 found a tiled-JPEG regression, change Plan 066's disposition to something factually accurate such as:

```text
CLOSED WITH POST-CLOSURE RESIDUAL — superseded by Plan 067
```

Retain its historical rows.

## 7.2 Reconcile Plan 063 public API inventory

`plans/063-status.md` still lists items such as `MIN_TILE_SIZE` / `tile_seed()` in the default LSB public inventory even though current `stegoeggo-stego::lsb` exposes only the final allowlist.

Update the current-disposition banner/inventory to match actual current source.

Do not rewrite the historical split decision.

## 7.3 Fill Plan 065 implementation commit evidence

Replace the stale:

```text
## Implementation commits
To be filled in at closure.
```

with actual implementation references, at minimum:

```text
70845eba... — Plan 065 phases 0-5
1c530a7f... — Plan 065 phases 6-10
```

and note that Plan 066/067 own later residual corrections rather than pretending those commits were part of Plan 065.

## 7.4 Update Plan 064 supersession note

Add a short final note that Plan 067 owns the tiled-JPEG/evidence correction discovered after Plan 066.

Do not rewrite its original evidence table.

## 7.5 Roadmap 057 final disposition

Only after all product/test checks pass, change Roadmap 057 to:

```text
Status: COMPLETE — final tiled-JPEG and evidence residuals closed by Plan 067
```

### Phase 7 acceptance criteria

- no ledger claims a tracked pre-edit status file that did not exist in commit history;
- Plan 063 current API inventory matches source;
- Plan 065 contains actual implementation commit references;
- Plan 066 is preserved as historical work but does not claim perfect final closure after a known regression;
- Roadmap 057 remains PARTIAL until final verification succeeds.

---

# Phase 8 — focused validation

Run the smallest useful checks first, then the repository's normal check.

## 8.1 Carrier tiled-JPEG tests

Run the relevant carrier tests, for example:

```bash
cargo test -p stegoeggo-stego --features application-support tiled_jpeg
cargo test -p stegoeggo-stego --features application-support jpeg_
```

Use the actual test filters that match the implemented names.

## 8.2 Root steganography tests

Run focused root tests:

```bash
cargo test -p stegoeggo --all-features protected::steganography
cargo test -p stegoeggo --all-features request_from_legacy
cargo test -p stegoeggo --test plan065_legacy_compat
```

If filter syntax differs, record the actual commands/results in `067-status.md`.

## 8.3 Boundary/doctest verification

Ensure the carrier compile-fail boundary still passes:

```bash
cargo test -p stegoeggo-stego --doc
```

The following must remain unnameable through the default API:

```text
stegoeggo_stego::jpeg_transcoder::JpegTranscoder
stegoeggo_stego::__internal_jpeg_facade::...
stegoeggo_stego::__internal_lsb_facade::...
```

The optional `application-support` API may expose only its documented operation-level/simple candidate types.

## 8.4 Full repository check

Run:

```bash
./scripts/check.sh
```

Record actual output/summary, not anticipated counts.

## 8.5 Pre-release structural check

No publication is requested. Run only the bounded local pre-stage check:

```bash
./scripts/release-check.sh --allow-dirty --skip-check --stage=pre
```

This should fully package-verify `stegoeggo-stego` and structurally verify the unpublished dependent packages under the established staged model.

### Phase 8 acceptance criteria

- focused wrong-first/later-valid tiled-JPEG tests pass;
- direct production legacy-adapter tests pass;
- carrier doctests/API-boundary tests pass;
- `./scripts/check.sh` passes;
- staged pre-release structural check passes;
- no crate is published and no version/tag/release changes occur.

---

# Phase 9 — final source audit and closure

Before changing statuses to COMPLETE, perform a source audit.

Required searches should establish:

```text
no __internal_jpeg_facade in product source
no __internal_lsb_facade in product source
no root import of JpegHeader / Coefficients / JpegTranscoder / DctStegoF5
no public Plan-specific compatibility test hook
no duplicated plan065 compatibility request builder claiming production equivalence
no flattened tiled-JPEG prefix/header/full candidate crossing in root extraction
```

Inspect the actual support API and verify that:

- candidate identity includes all state required to reproduce exact extraction;
- identity is preserved for V3 and legacy lengths;
- malformed/wrong candidates continue rather than terminate search;
- max origin bounds remain enforced;
- tiled embed success is checked on encoded output.

Update `plans/067-status.md` row-by-row with concrete evidence.

Only then:

1. set `plans/067-status.md` to COMPLETE;
2. set Roadmap 057 to COMPLETE with Plan 067 as the final closure owner;
3. ensure Plan 064/065/066 notes point forward truthfully;
4. commit all tracked planning files, remembering `git add -f plans/...` where required.

---

## 10. Hard stop / non-closure conditions

Do **not** mark Plan 067 or Roadmap 057 COMPLETE if any of the following is true:

- prefix/header/full extraction can come from different tiled-JPEG candidate identities;
- a wrong early `NotV3` candidate returns `None` before later candidates are tried;
- malformed/unsupported/auth-failed candidate masks a later valid candidate;
- candidate enumeration deduplicates distinct identities solely by equal bytes;
- `max_origins` is no longer bounded;
- tiled JPEG reports `Embedded` without verifying the encoded output;
- the parent crate again imports JPEG parser/coefficient/F5 implementation types;
- default carrier API exposes codec internals;
- legacy adapter tests primarily exercise a duplicated test implementation instead of production `request_from_legacy()`;
- Plan 066 still falsely claims its tracked ledger predated the implementation commit;
- Plan 063's current API inventory is materially stale;
- Plan 065 still has an unfilled implementation-commit section;
- `./scripts/check.sh` fails;
- the staged pre-release structural check fails.

Partial closure is preferable to another inaccurate COMPLETE claim.

---

## 11. Expected implementation footprint

The corrective implementation should normally remain concentrated in:

```text
stegoeggo-stego/src/application_support.rs
stegoeggo-stego/src/jpeg.rs                 # only if support plumbing requires it
stegoeggo-stego/src/jpeg_transcoder/*        # tests/private helper only if required
src/protected/steganography.rs
src/lib.rs                                   # direct private legacy-adapter tests

tests/plan065_legacy_compat.rs               # remove duplicated request builder / keep public behavior tests

plans/057-stego-carrier-library-and-pipeline-simplification-roadmap.md
plans/063-status.md
plans/064-status.md
plans/065-status.md
plans/066-status.md
plans/067-status.md
```

Possible documentation touch only if implementation changes an already documented support detail:

```text
architecture/protected-steganography.md
stegoeggo-stego/README.md
```

Changes outside this footprint require a direct explanation in `067-status.md`.

Files from unrelated `e694209` cleanup (`conformance`, XMP/WebP, payload flags, etc.) should not be modified merely because they were part of the recent commit history.

---

## 12. Suggested implementation commit structure

Keep the handoff easy to review. Prefer a small number of coherent commits, e.g.:

```text
1. plan 067: create status ledger and reopen roadmap
2. fix tiled JPEG candidate identity and encoded-output verification
3. test production legacy adapter and add targeted tiled-JPEG regressions
4. reconcile closure ledgers and close roadmap after full verification
```

Do not combine unrelated cleanup into these commits.

---

## 13. Final acceptance criteria

Plan 067 is complete only when all of the following are true:

1. tiled-JPEG prefix/header/full extraction is candidate-identity-preserving;
2. later valid candidates survive wrong early `NotV3`, malformed, unsupported, or authentication-failed candidates;
3. legacy tiled-JPEG probing remains functional and bounded;
4. distinct candidate identities are not lost by byte-value deduplication;
5. `max_origins` retains its established bounded semantics;
6. tiled-JPEG `Embedded` means the actual encoded JPEG yields the payload from a recorded successful tile;
7. failed post-encode extraction is not reported as successful embedding;
8. default carrier API still hides all JPEG codec/coefficient/F5 and low-level LSB internals;
9. root application uses only the feature-gated operation-level support boundary for compatibility mechanics;
10. production `request_from_legacy()` is directly exercised by crate-local tests;
11. the duplicated integration-test request translator is removed as compatibility evidence;
12. Plan 063 current API inventory matches current source;
13. Plan 065 contains actual implementation commit references;
14. Plan 066 chronology is corrected and its post-closure residual is acknowledged;
15. Plan 064 points to the final Plan 067 closure without rewriting history;
16. Roadmap 057 remains PARTIAL until all checks pass, then becomes COMPLETE once;
17. focused tiled-JPEG regression tests pass;
18. focused legacy compatibility tests pass;
19. carrier boundary/doctest checks pass;
20. `./scripts/check.sh` passes;
21. `./scripts/release-check.sh --allow-dirty --skip-check --stage=pre` passes;
22. no version bump, publication, tag, GitHub Release, CI expansion, payload change, or unrelated architecture work occurs.

When these criteria are satisfied, Roadmap 057 can be treated as genuinely closed rather than conditionally closed pending another carrier-boundary correction.
