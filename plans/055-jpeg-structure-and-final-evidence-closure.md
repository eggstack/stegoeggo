# Plan 055: JPEG Structure and Final Evidence Closure

Status: Ready for implementation after or alongside Plan 054 product work

Audited baseline: `main` at `e683c87785d7e4ec60b17fa8ec961d983e4b6fac`

Plan 055 owns the remaining JPEG structural-analysis defects and the final evidence/planning reconciliation for the Roadmap 045 closure line.

Authoritative implementation ledger: `plans/055-status.md`

Dependency rule:

- JPEG implementation phases may be executed independently of Plan 054.
- Final planning closure in Phase 3 must not occur until both Plan 054 and Plan 055 product criteria are complete.

---

## 1. Purpose

The Plan 053 JPEG implementation correctly landed several major fixes:

- exact entropy-only decoding for the supported single-scan path;
- actual decoded-block accounting;
- unread entropy byte and final pad-bit rejection;
- DHT class validation;
- one shared canonical Huffman entry builder;
- decoder lookup derived from those canonical entries;
- restart-bearing scan rejection in `probe_dct_support_full()`;
- corrected normal SOS offset fields.

The remaining JPEG defect is concentrated in structural analysis. `JpegHeader::analyze_structure()` remains a best-effort function that returns a partially populated `JpegStructure` after malformed marker or segment boundaries. It also does not preserve the first `0xFF` offset of a repeated marker-fill run, so marker fill bytes can remain inside the reported entropy slice.

The evidence ledgers also currently claim completion while Plan 053's own CI row remains open/pending and the Plan 053 plan header still says ready for implementation.

This plan closes those bounded gaps and performs the final truthful reconciliation only after Plan 054 is also complete.

---

## 2. Scope

Primary product files:

```text
src/jpeg_transcoder/header.rs
src/jpeg_transcoder/mod.rs
```

Touch `src/jpeg_transcoder/entropy.rs` only if a focused regression demonstrates that the already-shared canonical lookup or finalization contracts require correction.

Tests may be added under existing JPEG test modules and:

```text
tests/jpeg_container_preservation.rs
tests/conformance_container_tests.rs
```

Planning/evidence files in final phase:

```text
plans/045-status.md
plans/051-status.md
plans/052-status.md
plans/053-status.md
plans/054-status.md
plans/055-status.md
plans/053-xmp-animated-webp-and-jpeg-exactness-closure.md
```

Do not change:

- release machinery;
- version numbers;
- crates.io configuration;
- CI job count or matrix;
- public JPEG API;
- JPEG progressive/restart support policy;
- coefficient algorithm behavior unless required by a focused failing fixture.

---

# Phase 0 — establish truthful OPEN state

Before product edits:

1. Read `plans/055-status.md`.
2. Record actual starting SHA.
3. Confirm the JPEG residual rows remain OPEN.
4. Do not mark historical plans complete based on a broad test pass.

Expected residual rows:

```text
checked structural analysis: OPEN
truncated marker-run handling: OPEN
segment length < 2 handling: OPEN
truncated segment length handling: OPEN
segment extent overflow handling: OPEN
malformed SOS handling: OPEN
first-fill-byte entropy end: OPEN
multiple-FF-before-00 handling: OPEN
checked probe mapping: OPEN
checked decode structure reuse: OPEN
focused malformed fixtures: OPEN
final current-head CI evidence: OPEN
cross-plan ledger reconciliation: OPEN
```

---

# Phase 1 — replace best-effort JPEG structure scanning with checked analysis

Primary file: `src/jpeg_transcoder/header.rs`

## 1.1 Introduce one checked analyzer

Preferred API:

```rust
pub(crate) fn analyze_structure_checked(data: &[u8]) -> Result<JpegStructure>;
```

If the existing non-Result `analyze_structure()` has internal callers that require it, either:

- convert all repository callers to the checked API and remove the old helper; or
- retain a thin non-production compatibility wrapper only if necessary, clearly documenting that supported DCT decisions and decoding use the checked API.

No supported-path correctness decision may rely on a best-effort partial structure.

## 1.2 Define exact marker categories

Use explicit marker handling rather than a generic "all markers have lengths" assumption.

Standalone/no-length markers:

```text
SOI  FF D8
EOI  FF D9
RST0..RST7 FF D0..D7
TEM  FF 01
```

Markers with a two-byte big-endian segment length include normal APP/DQT/DHT/SOF/SOS/etc. The declared segment length includes the two length bytes and must be at least 2.

Inside entropy, `FF 00` is byte stuffing and not a marker.

Repeated `FF` bytes before a real marker are marker fill and belong outside the entropy slice.

## 1.3 Add one checked segment-end helper

Use a helper equivalent to:

```rust
fn checked_segment_end(data: &[u8], marker_start: usize) -> Result<usize>;
```

Contract:

1. require marker bytes and two length bytes to exist;
2. decode `seg_len` as big-endian `u16`;
3. reject `seg_len < 2`;
4. compute `marker_start + 2 + seg_len` with checked arithmetic;
5. reject if the end exceeds input length;
6. return the first byte after the complete marker segment.

No `break` on malformed segment length. Return an error.

## 1.4 Add one checked marker-run parser

Use a helper or equivalent logic that preserves the first `0xFF` offset.

Conceptual result:

```rust
struct MarkerRun {
    run_start: usize,
    marker_code_offset: usize,
    marker: u8,
}
```

Given `data[run_start] == 0xFF`:

1. Save `run_start` immediately.
2. Advance over one or more `0xFF` bytes.
3. If input ends before a non-`0xFF` byte, return malformed/truncated marker-run error.
4. Let the first non-`0xFF` byte be `marker`.
5. Do not move `run_start` to the last fill byte.

For entropy termination, `entropy_end` and `terminating_marker_offset` must both use `run_start`.

## 1.5 Handle stuffing and repeated fill exactly

While inside entropy:

### Case A: ordinary data byte

```text
not 0xFF -> entropy continues
```

### Case B: exactly `FF 00`

```text
one FF followed by 00 -> stuffed data; both bytes remain inside entropy
```

Advance past both bytes and continue scan.

### Case C: repeated FF before real marker

Example:

```text
... entropy ... FF FF D9
```

Required:

```text
run_start = offset of first FF
marker = D9
entropy_end = run_start
terminating_marker_offset = run_start
```

Neither fill `FF` belongs to the entropy slice.

### Case D: multiple FF before 00

Example:

```text
FF FF 00
```

Plan 053's explicit contract requires this to be malformed rather than ordinary byte stuffing.

Reject it.

### Case E: dangling FF run at EOF

Reject.

## 1.6 Handle restart markers consistently

Inside entropy, RST0..RST7 markers:

- set `has_restart_markers = true`;
- do not terminate the scan merely because a restart marker was encountered;
- advance past the complete marker run;
- continue structural scanning;
- `probe_dct_support_full()` must still classify the JPEG as unsupported for DCT embedding.

The checked analyzer exists to classify the structure accurately even though the current transcoder does not support restart-bearing entropy.

Do not teach `BitReader` to decode restart intervals in this plan.

## 1.7 Handle SOS exactly

When SOS is encountered outside entropy:

- record `sos_marker_offset = marker_run.run_start`;
- validate the SOS marker segment with checked length handling;
- set `sos_header_end` to the first byte after the complete SOS segment;
- set `entropy_start = sos_header_end`;
- enter entropy mode at exactly that byte.

For supported baseline JPEG:

```text
sos_marker_offset < sos_header_end == entropy_start < entropy_end
```

## 1.8 Handle scan termination

On EOI while in scan:

- close current `JpegScanSpan` using the first `FF` of the EOI marker run;
- set EOI offset consistently;
- leave `has_trailing_segments_after_scan = false`.

On a non-EOI marker terminating entropy:

- close the scan at the first `FF` of that marker run;
- classify post-scan structural content according to existing policy;
- if another SOS appears, count another scan;
- any unsupported multi-scan/progressive shape remains unsupported, not newly implemented.

If the file ends while still in scan without a terminating marker, return malformed structure.

## 1.9 Reject malformed boundaries instead of returning partial state

Required checked errors:

```text
input shorter than SOI minimum
truncated trailing FF
truncated FF fill run
length-bearing marker missing length bytes
segment length smaller than two
segment extends beyond input
checked arithmetic overflow
malformed SOS segment extent
entropy scan reaches EOF without terminating marker
invalid marker sequence where current bounded grammar cannot determine structure
```

Do not return a partial `JpegStructure` for these cases.

## Phase 1 required tests

Add exact tests:

```text
checked_structure_rejects_truncated_ff_run
checked_structure_rejects_segment_missing_length_bytes
checked_structure_rejects_segment_length_zero
checked_structure_rejects_segment_length_one
checked_structure_rejects_segment_extending_past_input
checked_structure_rejects_truncated_sos
checked_structure_rejects_entropy_without_terminator
sos_marker_offset_points_to_first_sos_ff
entropy_start_equals_sos_header_end_checked
entropy_end_excludes_single_marker_prefix
entropy_end_excludes_all_repeated_marker_fill_bytes
terminating_marker_offset_points_to_first_fill_ff
stuffed_ff00_remains_inside_entropy
multiple_ff_before_00_is_rejected
restart_marker_inside_scan_is_recorded
restart_marker_does_not_end_scan_structure
```

## Phase 1 acceptance criteria

- supported-path structural analysis returns `Result`;
- malformed marker/segment boundaries return errors, not partial state;
- repeated marker fill uses the first `FF` offset;
- `FF 00` remains stuffed entropy;
- `FF FF 00` is rejected per the plan contract;
- restart markers are recorded without falsely ending the scan;
- SOS and entropy offsets have exact documented meanings;
- all required tests pass.

Suggested commit:

```text
jpeg: make scan structure analysis checked and fill-exact
```

---

# Phase 2 — route probing and decoding through the checked structure

Primary files:

```text
src/jpeg_transcoder/header.rs
src/jpeg_transcoder/mod.rs
```

## 2.1 `probe_dct_support_full()` must use checked analysis

Required behavior:

```text
header policy fails -> existing unsupported reason
checked structure Err -> Unsupported(MalformedHeader) or the repository's equivalent malformed classification
scan_count != 1 -> MultipleScans
missing EOI -> malformed
restart marker -> RestartIntervals
trailing post-scan segment -> TrailingSegmentsAfterScan
otherwise -> Supported
```

Do not call the old best-effort structure analyzer.

Malformed input must remain distinguishable from a valid-but-unsupported JPEG at the decode/process boundary where the current API already distinguishes those classes.

## 2.2 Avoid contradictory double analysis

Current decode flow probes and then analyzes again. Acceptable options:

### Preferred bounded option

Introduce an internal helper:

```rust
fn checked_supported_structure(
    header: &JpegHeader,
    jpeg_data: &[u8],
) -> Result<JpegStructure>;
```

It performs checked structure analysis and validates the supported-path policy once.

`decode_coefficients()` then receives the validated structure directly.

### Acceptable simpler option

Call `analyze_structure_checked()` twice only if both calls are deterministic and both errors propagate. Do not use checked analysis in the probe and best-effort analysis in the decoder.

Avoid a broad support-probe architecture redesign.

## 2.3 Preserve exact entropy slicing

For supported JPEG:

```rust
let entropy_slice = &jpeg_data[span.entropy_start..span.entropy_end];
```

No marker fill, EOI, trailing segment, or unrelated bytes may be included.

Retain the existing decoder finalization checks for:

- expected vs decoded blocks;
- unread full entropy bytes;
- final all-one padding bits.

Do not reopen the entropy decoder unless a new focused fixture fails.

## 2.4 Required classification tests

```text
restart_marker_without_dri_is_unsupported
restart_marker_with_dri_is_unsupported
malformed_truncated_marker_run_is_not_supported
malformed_short_segment_is_not_supported
multiple_scan_valid_jpeg_remains_unsupported
post_scan_segment_remains_unsupported
supported_baseline_remains_supported
```

Where practical, assert the exact `DctUnsupportedReason` rather than only `is_err()`.

## 2.5 Required roundtrip regressions

Retain or add:

```text
supported_baseline_roundtrip_decodes
supported_baseline_metadata_rewrite_decodes
supported_baseline_entropy_slice_has_no_eoi
supported_baseline_entropy_slice_has_no_marker_fill
```

No progressive/restart implementation is required.

## Phase 2 acceptance criteria

- full support probing uses checked structural analysis;
- decoding uses the checked span;
- malformed boundaries cannot become `Supported`;
- restart-bearing scans remain unsupported;
- multi-scan/trailing-segment policy remains unchanged;
- exact entropy slice contains only entropy bytes/stuffing;
- existing canonical Huffman implementation remains intact;
- all focused and roundtrip tests pass.

Suggested commit:

```text
jpeg: route DCT support through checked scan structure
```

---

# Phase 3 — final verification and truthful planning reconciliation

This phase is the final gate for the Roadmap 045 / Plans 051-055 line.

Do not execute its completion edits until Plan 054 product work is also complete.

## 3.1 Verify Plan 054 state first

Read `plans/054-status.md`.

Required before historical closure:

```text
Disposition: COMPLETE
all XMP defect rows CLOSED
all animated-WebP defect rows CLOSED
all Plan 054 required verification commands recorded
```

If Plan 054 remains partial, Plan 055 may close its JPEG product rows but must remain `PARTIAL — waiting on Plan 054 for final roadmap reconciliation`.

## 3.2 Run focused JPEG commands

```bash
cargo test -p stegoeggo jpeg --all-features
cargo test -p stegoeggo --test jpeg_container_preservation --all-features
cargo test -p stegoeggo --test conformance_container_tests --all-features
```

If a test target/filter differs in the repository, record the exact command actually used.

## 3.3 Run final workspace verification

At the final product head after both plans:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo check -p stegoeggo --no-default-features
cargo test --workspace --exclude stegoeggo-fuzz --all-features
./scripts/check.sh
```

Record exact observed results and exact SHA.

Do not expand CI.

## 3.4 Current-head CI evidence policy

The existing CI is already:

```text
one `check` job
push to main / pull_request to main
./scripts/check.sh
```

Do not create an empty/no-op commit solely to "trigger CI". A normal implementation or final planning commit on `main` already triggers the configured push workflow.

For the exact final product/planning head, record one of:

```text
PASS — exact run/status URL or run ID observed for the exact SHA
FAIL — exact failing run/status and failing step observed
UNAVAILABLE — connector/API does not expose an exact status for that SHA
```

`UNAVAILABLE` is acceptable evidence bookkeeping when local required verification is green. It must not be relabeled PASS.

Do not cite CI from an earlier SHA as current-head evidence.

## 3.5 Reconcile Plan 053 plan header

The current Plan 053 plan document still says `Status: Ready for implementation` while its ledger has claimed completion.

After Plans 054 and 055 are genuinely complete, change its header to a truthful historical state, for example:

```text
Status: Completed after residual corrective Plans 054 and 055
```

If either plan remains partial, use:

```text
Status: Partially implemented — residual closure delegated to Plans 054 and 055
```

Do not leave contradictory ready/complete states.

## 3.6 Reconcile historical status ledgers

Only after both Plans 054 and 055 are product-complete:

Update:

```text
plans/045-status.md
plans/051-status.md
plans/052-status.md
plans/053-status.md
plans/054-status.md
plans/055-status.md
```

Rules:

- preserve the history of earlier premature closure claims;
- list exact corrective implementation SHAs;
- do not rewrite historical failures as if they never happened;
- distinguish local verification from GitHub CI evidence;
- remove `pending` placeholders from final disposition rows;
- do not claim more tests than actually ran;
- use the exact final SHA, not the prior planning SHA;
- Roadmap 045 may become COMPLETE only if every Plan 054 and 055 definition-of-done item is closed.

## 3.7 Static searches

Run:

```bash
rg -n 'analyze_structure\(' src/jpeg_transcoder
rg -n 'analyze_structure_checked' src/jpeg_transcoder
rg -n 'break;' src/jpeg_transcoder/header.rs
rg -n 'Disposition: \*\*COMPLETE|PENDING|pending final' plans/045-status.md plans/051-status.md plans/052-status.md plans/053-status.md plans/054-status.md plans/055-status.md
```

Interpretation:

- any remaining `break` must be audited; it may be legitimate in a successful terminal condition but must not silently replace a malformed structural error;
- supported-path calls must use the checked analyzer;
- no final ledger may simultaneously say COMPLETE and contain a required OPEN/PENDING row.

## 3.8 Publication remains separate

Even after final closure:

- do not bump version;
- do not tag;
- do not publish to crates.io;
- do not create a GitHub Release;
- do not add a release workflow.

Release remains a separate manual operation.

---

# Definition of Done — Plan 055

## JPEG structure

1. A checked JPEG structural analyzer exists for supported-path decisions.
2. Truncated FF marker runs return an error.
3. Missing segment length bytes return an error.
4. Segment length 0 returns an error.
5. Segment length 1 returns an error.
6. Segment extent beyond input returns an error.
7. Arithmetic overflow returns an error.
8. Malformed SOS extent returns an error.
9. Entropy reaching EOF without a terminating marker returns an error.
10. `sos_marker_offset` is the leading FF of SOS.
11. `sos_header_end` is the first byte after the complete SOS segment.
12. `entropy_start == sos_header_end`.
13. `entropy_end` points to the first FF of the terminating marker/fill run.
14. `terminating_marker_offset` points to that same first FF.
15. Exactly `FF 00` remains stuffed entropy.
16. Repeated FF fill before a marker is excluded completely from entropy.
17. Multiple FF before 00 is rejected under the current bounded contract.
18. Restart markers are recorded without falsely terminating the scan.
19. Restart-bearing scans remain unsupported.
20. Multi-scan JPEG remains unsupported.
21. Post-scan segments remain unsupported.
22. Malformed structural input cannot be classified Supported.
23. Decoder uses the checked exact entropy span.
24. Existing block-count/exhaustion/pad-bit validation remains green.
25. Existing shared canonical Huffman entry path remains green.
26. Supported baseline roundtrip remains green.

## Verification

27. Focused JPEG suite passes.
28. JPEG container preservation suite passes.
29. Conformance container suite passes.
30. `cargo fmt --all -- --check` passes.
31. clippy with `-D warnings` passes.
32. no-default-features check passes.
33. workspace tests pass.
34. `./scripts/check.sh` passes.
35. All results are recorded against the exact implementation/final head.

## Cross-plan closure

36. Plan 054 is COMPLETE before final historical reconciliation.
37. Plan 054 status contains no required OPEN rows.
38. Plan 055 JPEG rows contain no required OPEN rows.
39. Plan 053 plan header and status no longer contradict each other.
40. Plan 053 current-head CI evidence is either exact PASS/FAIL or honestly UNAVAILABLE.
41. Historical status files retain truthful premature-closure history.
42. No final status says COMPLETE while a required row remains OPEN/PENDING.
43. Roadmap 045 is marked COMPLETE only after all Plan 054/055 criteria pass.
44. No CI architecture expansion occurred.
45. No no-op CI-trigger commit is required or used as evidence of correctness.
46. No release/version/tag/publication work occurred.

If the JPEG product work is complete but Plan 054 is not, Plan 055 must remain `PARTIAL` for cross-plan closure even though its JPEG rows may be CLOSED.
