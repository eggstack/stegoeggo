# Plan 085: Container Observer and Resource-Accounting Reuse

Status: READY FOR IMPLEMENTATION

Parent roadmap: `plans/081-pre-v1-consolidation-maintainability-and-portability-roadmap.md`

Audited baseline: `07bd05304a6942a843a76c28013a5bc0f08179cc`.

## 1. Objective

Remove duplicate PNG/JPEG/WebP structural walking used only for resource accounting and make accounting consume canonical bounded container traversal or shared iterators. This reduces parser drift on malformed/adversarial input while preserving all existing resource-limit semantics.

At baseline, `observe_metadata_work()` in `src/lib.rs` independently walks PNG chunks, JPEG markers, and WebP RIFF chunks even though the repository already contains format-aware parser/container logic elsewhere.

## 2. Audit requirements

Before editing, map every structural walker for:

- PNG chunks and text/iTXt/XMP ownership;
- JPEG markers/segments and metadata APP/COM handling;
- WebP RIFF chunks and XMP/EXIF handling;
- preflight JPEG inspection;
- metadata injection/removal;
- resource accounting.

For each walker record bounds checks, overflow behavior, malformed-input disposition, and resource-observer events. Identify the canonical traversal for each format.

## 3. Shared traversal design

Choose the smallest reuse mechanism that fits the existing code. Preferred options, in order:

1. expose private bounded iterators/callback traversal from existing container modules;
2. allow canonical parsers to accept a lightweight private observer/callback;
3. return compact structural metadata that accounting can consume.

Do not introduce a public visitor framework or generic container trait.

The observer should be capable of reporting only facts already used by `OperationObserver`, such as:

- chunk/segment bytes traversed;
- metadata field bytes encountered;
- format-specific segment/chunk counts if required by existing limits.

Parsing remains owned by canonical format code. Resource accounting must not be able to reinterpret malformed structure differently.

## 4. Correctness invariants

- Preserve `ResourceLimits` behavior and error classification.
- Preserve checked arithmetic and no-panic behavior on truncated/oversized input.
- Preserve metadata update policy behavior and ownership detection.
- Preserve JPEG scan/entropy boundaries; do not attempt to parse compressed entropy as metadata segments.
- Preserve RIFF padding rules and PNG chunk-length/CRC handling as currently intended.
- Do not increase unbounded allocation or collect all chunks merely for accounting.
- Keep traversals streaming/index-based over slices where possible.

## 5. Tests

Add regression tests comparing old expected accounting values/limit outcomes for representative valid PNG/JPEG/WebP files and malformed cases including:

- truncated chunk/segment headers;
- oversized declared lengths;
- arithmetic overflow boundaries where constructible;
- JPEG SOS boundary behavior;
- WebP odd-size padding;
- PNG text/iTXt metadata payloads;
- files with many small metadata chunks/segments;
- resource limits triggered exactly at and just beyond configured thresholds.

If exact `ResourceUsage` values are part of public reports, preserve them or document/correct any previously double-counted value only with explicit evidence and compatibility consideration.

Fuzz targets that cover PNG metadata, WebP RIFF parsing, XMP extraction, and pipeline bytes should continue to compile and should be run for a short local smoke interval where tooling permits.

## 6. Cleanup

Delete the duplicate structural walker from `src/lib.rs` once all callers use shared traversal. Search the repository for remaining hand-written loops over PNG signature/chunks, JPEG `0xFF` markers, and RIFF chunk headers; classify each as canonical parser, test fixture helper, or remaining duplication.

Document intentional multiple traversals only where they serve genuinely different parsing domains.

## 7. Acceptance criteria

- `src/lib.rs` no longer contains an independent format parser solely for metadata resource accounting;
- accounting receives events/results from canonical shared bounded traversal;
- malformed-input and limit behavior is covered by focused tests;
- no public parser/visitor API is added;
- no unbounded allocations are introduced;
- fuzz targets still build;
- `./scripts/check.sh` passes;
- `plans/085-status.md` records the final traversal ownership map.

## 8. Non-goals

No metadata schema redesign, no new image format, no parser rewrite from scratch, no carrier changes, and no externally public container abstraction.
