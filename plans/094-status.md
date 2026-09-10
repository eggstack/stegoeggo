# Plan 094 Status

Status: COMPLETE

Baseline: `27bdd3d429d663948021de43d3e6f818fa613319`
Depends on: Plans 091 and 092.

## Required evidence

- [x] one-shot repeated-operation decode baseline recorded
- [x] prepared JPEG ownership/borrowing API decision recorded before implementation
- [x] reusable decoded-state core shared by one-shot and prepared APIs
- [x] public prepared type exposes no header/coefficient/Huffman/F5 internals
- [x] repeated prepared read operations use exactly one coefficient decode
- [x] capacity/raw/framed/tiled parity with one-shot APIs verified
- [x] failed prepared embedding leaves reusable state unchanged
- [x] container preservation verified for prepared embedding
- [x] `Send`/`Sync` disposition recorded from actual representation
- [x] hidden parent `JpegSearchContext` reuse/reduction audited
- [x] direct standalone consumer compiles without `application-support`
- [x] `./scripts/check.sh` passes (local full gate green, 2026-09-10)

## Implementation notes

- Baseline: one-shot `capacity`/`extract`/`extract_framed` each decode
  coefficients per call (`one_shot_extract_decodes_once_per_call`: 10
  calls = 10 decodes); framed/tiled-framed retain one decode per
  operation. Repeated generic sequences (`capacity` + `extract` +
  `extract_framed` + tiled recovery) therefore decode 4+ times.
- Ownership decision: borrowing `PreparedJpeg<'a>` is the single public
  shape. The caller controls allocation; no encoded-JPEG copy is made
  (unit test asserts pointer identity with the source slice) and
  container-preserving re-encoding refers to the borrowed source. No
  owned form was added: no batch/service use case demonstrated a need.
- Shared core: one-shot `extract_framed`/`extract_tiled`/
  `extract_tiled_framed` now delegate to `pub(crate)`
  `extract_framed_from_decoded`/`extract_tiled_from_decoded`/
  `extract_tiled_framed_from_decoded`; `capacity`/`extract` already shared
  `capacity_from_decoded`/`extract_from_decoded`. New shared
  `embed_strict_from_decoded` serves one-shot `embed_strict` (which now
  decodes via `decode_supported_carrier`, keeping probe classification
  identical) and `PreparedJpeg::embed_strict`. Legacy best-effort `embed`
  keeps its direct decode path so existing decode-count tests are
  unaffected.
- Public surface (`prepared::PreparedJpeg`): `new`, `support`,
  `capacity`, `extract`, `extract_framed`, `extract_tiled`,
  `extract_tiled_framed`, `embed_strict`, `embed_framed_strict`,
  `seed_hint`. Unsupported-but-well-formed JPEGs construct successfully
  carrying `JpegSupport::Unsupported`; coefficient operations on them
  return `UnsupportedJpeg`. Manual `Debug` impl exposes only support and
  decode presence. No `Clone` (would imply a large coefficient-map copy
  contract). No async wrapper (CPU-bound work).
- Privacy: fields private; `JpegHeader`, `Coefficients`, Huffman/F5 types
  remain `pub(crate)`-internal. The existing `compile_fail` privacy tests
  plus the new module (which never names codec types in signatures) guard
  this; `cargo public-api`-style listing deferred to Plan 097 semver
  audit.
- `Send`/`Sync`: representation (`&[u8]` + support enum + owned
  header/coefficient map) is naturally `Send + Sync`; compile-time
  assertion test included. No synchronization added.
- Evidence: repeated prepared reads (capacity + framed + tiled-framed +
  seed hint) use exactly 1 decode; prepared/one-shot parity asserted for
  capacity, raw extraction at redundancies 1..=3, framed, tiled raw,
  tiled framed, strict and framed-strict bytes (byte-identical outputs,
  hence identical container preservation), and seed-hint reads.
- `JpegSearchContext` audit: retained. Its standard probing already
  shares the same decoded-state helpers; its tiled candidate
  classification spans redundancies and legacy payload versions, which
  the generic tiled API deliberately excludes. Retrofitting a borrow
  lifetime through verification-critical code buys no decode reduction
  (no double-decode exists per verification operation). Stale
  `PRIVATE-REUSE-SUFFICIENT` module wording updated to point generic
  consumers at `PreparedJpeg`.
- Standalone use: `prepared` is default-feature API with no
  `application-support` dependency; direct-consumer fixture expanded in
  Plan 097.

Record final `./scripts/check.sh` result and implementation commit SHA here
during closure.

Implementation commit SHA: `d42f8ba` (roadmap 090 implementation on `main`; this ledger closure is the follow-up commit).
