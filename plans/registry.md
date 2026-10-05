# StegoEggo Active Planning Registry

Compact control surface for active interim planning. Detailed requirements
stay in roadmaps, implementation plans, `closure/`, and Git history.

Canonical direction: `plans/000-long-term-specification.md`,
`plans/001-terminology-and-domain-model.md`,
`plans/002-long-term-roadmap.md`, `plans/003-planning-process.md`.
System guide: `plans/README.md`. Flat `001`–`107` are immutable
predecessor history, indexed by the roadmaps below.

## Status vocabulary

- **proposed** — exists but not approved for execution.
- **ready** — dependencies satisfied; may be handed off.
- **active** — implementation or closure work in progress.
- **blocked** — a named dependency or evidence requirement prevents progress.
- **closing** — implementation landed; closure evidence being gathered.
- **closed** — closure record accepted.
- **conditionally closed** — substantial work landed, named evidence outstanding.
- **superseded** — replaced by another document.
- **archived** — inactive; retained for traceability.

## Active subsystem roadmaps

| Subsystem | Status | Roadmap | Current milestone | Dependencies or blockers |
|---|---|---|---|---|
| rights-metadata | closed | `plans/subsystems/rights-metadata-roadmap.md` | all milestones closed | none |
| container-correctness | closed | `plans/subsystems/container-correctness-roadmap.md` | all milestones closed | none |
| stego-carrier | closed | `plans/subsystems/stego-carrier-roadmap.md` | all milestones closed | none |
| verification-conformance | closed | `plans/subsystems/verification-conformance-roadmap.md` | all milestones closed | none |
| api-cli-contract | closed | `plans/subsystems/api-cli-contract-roadmap.md` | all milestones closed | none |
| release-distribution | active | `plans/subsystems/release-distribution-roadmap.md` | M004 0.4.3 release ready; M001 blocked pending public B; M002 conditionally closed; M003 closed | M004 is the ordinary stable B handoff intended to supply both M001 and M002 live evidence |
| language-bindings | closed | `plans/subsystems/language-bindings-roadmap.md` | all milestones closed; initial Python -> Node -> C sequence complete | none |
| stego-library-evolution | active | `plans/subsystems/stego-library-evolution-roadmap.md` | M001–M005 and M008 closed; M006 blocked; M007 proposed | M006 waits only for ADR-0007 acceptance |
| maintenance-quality | closed | `plans/subsystems/maintenance-quality-roadmap.md` | M001, M003, M004 closed; M002 proposed optional polish | none |

## Dependency-ready implementation plans

| Subsystem | Milestone | Status | Plan | Notes |
|---|---|---|---|---|
| release-distribution | M002 Eggpack producer adoption and second-consumer qualification | conditionally closed | `plans/implementation/release-distribution/002-eggpack-producer-adoption-and-second-consumer-qualification.md` | cutover landed at `3b96fae` (Eggpack pin `56ed7e7`); live B > 0.4.2 evidence remains outstanding, shared with M001 |
| release-distribution | M004 0.4.3 release and planning reconciliation | ready | `plans/implementation/release-distribution/004-0.4.3-release-and-planning-reconciliation.md` | ordinary stable release; produces first live Eggpack B and public 0.4.2→0.4.3 updater evidence |

## Active closure work

None.

## Blocked work

| Subsystem | Milestone | Blocker |
|---|---|---|
| release-distribution | M001 real eggfetch-to-eggfetch A→B transition (flat `plans/106-real-eggfetch-to-eggfetch-self-update-closure.md`) | Public 0.4.3 not yet published; ready M004 is the authorized ordinary stable B release. Evidence remains in `plans/106-status.md` until the live transition runs. |
| release-distribution | M002 live second-consumer proof (WP9 first Eggpack-produced B + WP10 shared A→B) | Live 0.4.3 Eggpack publication/update evidence not yet recorded; ready M004 owns that shared release event. |
| stego-library-evolution | M006 keyed carrier placement | ADR-0007 acceptance only. |

## Recently closed work

- Maintenance Quality M004 CLI relative-path fix and README quickstart
  verification: closure recorded at `plans/closure/maintenance-quality/004-status.md`.
  Executing the README quickstart instead of only reading it exposed a real
  user-facing bug — `stegoeggo protect image.png -o out.png` failed on every bare
  relative output path, because `Path::parent()` returns `Some("")` for a bare
  name so the `unwrap_or_else(|| Path::new("."))` fallback in
  `check_input_output_disjoint` never fired and the empty path failed to
  canonicalize. Absolute and directory-qualified paths worked, and the CLI test
  suite only ever used absolute `tempfile` paths, so CI could not see it. Fixed in
  `stegoeggo-cli/src/protect.rs` (both `parent()` sites) with three regression
  tests, one of which was confirmed to fail against the pre-fix code. README
  rewritten quickstart-first with detail delegated to `docs/` and `SUPPORT.md`;
  every command and the Rust example were re-executed and all 13 links verified.
  Two documentation inaccuracies corrected (missing `webp` row in the SUPPORT.md
  feature table; `image-webp` dependency attribution in `docs/carrier-crate.md`).
  `check.sh` green. The pre-existing `release_eggpack` temp-dir flake remains
  open.
- Maintenance Quality M003 agent guidance and documentation currency: closure
  recorded at `plans/closure/maintenance-quality/003-status.md` (new
  `.skills/bindings/SKILL.md` for the previously undocumented FFI subsystem;
  `AGENTS.md` CI topology corrected from "everything else is scheduled/manual" to
  the four additional push/PR workflows, architecture count 39 → 42, bindings
  subsection and change-area → deep-dive index added; stale counts, carrier
  `limits`/`webp` modules, and `metadata_trap/spec.rs` corrected across the
  remaining skills; re-verification table added so future agents re-derive counts
  from the tree; no product code changed, `./scripts/check.sh` green). M002
  remains proposed and still needs a maintainer decision on CI placement. M003
  verification also surfaced a pre-existing required-CI flake —
  `tests/release_eggpack.rs::validator_temp_state_is_cleaned` races sibling
  validator tests over the shared temp directory — recorded as an unresolved
  finding, not fixed in this milestone.

- Stego Library Evolution M008 M003 default-limit compatibility corrective:
  closure recorded at `plans/closure/stego-library-evolution/008-status.md`
  (legacy structural-domain profile and split error mapping; explicit bounded
  variants retained; wide JPEG and parser-limit regressions; v0.4.2 behavior
  comparison; `./scripts/check.sh` green). M006 remains blocked only on
  ADR-0007 acceptance; M007 remains proposed pending explicit authorization.

- Stego Library Evolution M005 lossless WebP byte carrier: closure
  recorded at `plans/closure/stego-library-evolution/005-status.md`
  (implementation `a70aa75`; opt-in `webp` facade over the LSB carrier
  with explicit `CarrierLimits` on every operation, structured
  lossy/animated rejection, metadata-drop contract, 14 probe unit tests
  + 16 direct-consumer tests + module doctest green, semver-checks 196
  pass with no update required, `./scripts/check.sh` green).
- Stego Library Evolution M004 transactional in-place allocation
  hardening: closure recorded at
  `plans/closure/stego-library-evolution/004-status.md` (implementation
  `6616db2`; complete checked preflight replaces the full-image rollback
  clone, peak aux 1,053,952 → 5,376 bytes on a 1 MiB image with no
  runtime regression, byte-identical success/failure semantics,
  `./scripts/check.sh` green).
- Stego Library Evolution M003 carrier resource/prepared hardening:
  closure recorded at `plans/closure/stego-library-evolution/003-status.md`
  (implementation `6d87c57`; carrier-owned `CarrierLimits` with 13
  `*_with_limits` JPEG variants plus `PreparedJpeg::new_with_limits` and
  tiled exact-embed parity sharing one decode path, adversarial/tiled/
  prepared-reuse suites, facade/robustness regression coverage, fuzz
  `jpeg_parser` 1,531,200 runs + `tiled_round_trip` 1,323,044 runs clean,
  focused suites 195 + 13 + 35 carrier, 56 public API, 10 known-answer,
  71 robustness green, `./scripts/check.sh` green with 2054 passed /
  0 failed). M004 and M005 subsequently closed. A post-closure audit found
  that pre-existing JPEG inspection/prepared APIs inherited M003 input/dimension
  policy caps and error mapping; M008 corrected the discrepancy before a newer
  stable release. Its closure confirms direct-crate compatibility while the root
  continues applying its own `ResourceLimits`.
- Stego Library Evolution M002 generic carrier API normalization: closure
  recorded at `plans/closure/stego-library-evolution/002-status.md`
  (implementation `beb4145`; explicit `embed_best_effort` /
  `embed_framed_best_effort` aliases with old/new parity, `EmbedReport` /
  `InPlaceEmbedReport` accessor coverage, direct-consumer/public-API
  migration to strict/explicit names, semver-checks 196 pass / 58 skip
  with no update required, focused suites 192 + 10 + 35 carrier and
  55 public API / 10 known-answer green, `./scripts/check.sh` green with
  2045 passed / 0 failed). M003 promoted blocked → ready.
- Maintenance Quality M001 release workflow consolidation: closure
  recorded at `plans/closure/maintenance-quality/001-status.md`
  (implementation `f9bee3d`; all third-party actions full-SHA pinned,
  shared Zig/cargo-zigbuild composite actions consumed by `release-c.yml`,
  `release-binaries.yml` untouched per the Eggpack byte-shape stop condition,
  contract checker green, `release-c` run `37045104837` on `f9bee3d` 11/11
  green, `./scripts/check.sh` green; no publication). M002 drift linting
  stays proposed and needs only the maintainer CI-placement decision.
- Stego Library Evolution M001 boundary and metadata convergence: closure
  recorded at `plans/closure/stego-library-evolution/001-status.md`
  (implementation `ed1dd6d`; private `MetadataWriteSpec` plus one executor,
  explicit JPEG render values, six golden byte-identity hashes unchanged,
  dead `PixelSelectionRng` removed with grep proof, 18 hidden
  `application_support` symbols inventoried and retained, focused suites
  179 green, non-strict conformance 44/44, `./scripts/check.sh` green with
  2039 passed / 0 failed). M002 promoted blocked → ready.
- Language Bindings M010 C ABI v1 cross-platform qualification:
  closure recorded at `plans/closure/language-bindings/010-status.md`
  (final `release-c` run `36952787634` on `0e52feb`, 11/11 green:
  five native builds with exact 90-symbol exports, glibc 2.17 floor,
  C11/C++17 smokes, five clean artifact smokes, collector audit;
  ABI v1 stability activated in `STABILITY.md`). The initial
  Python → Node → C sequence is complete.
- Language Bindings M009 C ABI v1 implementation foundation: closure
  recorded at `plans/closure/language-bindings/009-status.md`
  (implementation `ee0f99e`; all 90 exports with opaque handles,
  generated header, 26 Rust + 171 C checks green, two-way parity green,
   `c-binding` runs `36945460318`/`36946341300` green on Linux
   x86_64, required CI green; no stable ABI claimed; toolchain record
   corrected at M010 closure). M010 closed after it.
- Language Bindings M008 M007 symbol-inventory corrective: closure
  recorded at `plans/closure/language-bindings/008-status.md`
  (implementation `6cbc0d4`; 90/87 arithmetic corrected with no signature
  change; checked 90-line manifest plus stdlib-only contract checker;
  C11/C++17 90-signature smoke green on Apple clang 21.0.0;
  `./scripts/check.sh` green). M009 subsequently implemented the corrected
  manifest and closed before M010 qualification.
- Language Bindings M007 C ABI contract design: closure recorded at
  `plans/closure/language-bindings/007-status.md` (normative
  `bindings/c/ABI-V1.md` plus `bindings/c/README.md`; design commit
  `3e3e463`; `./scripts/check.sh` green; C11/C++17 signature sketch
  smoke on Apple clang 21.0.0; no C symbols, header, or library shipped).
   A post-closure audit found a bounded count defect: the document declares
   90 callable functions while its arithmetic says 89. M008 corrected the
   evidence and added the checked 90-symbol manifest (closed; see above).
- Language Bindings M006 post-core compatibility corrective and
  requalification: closure recorded at
  `plans/closure/language-bindings/006-status.md` against final SHA
  `d24b377` (implementation `e6c1e2b` + test-only `d24b377`; CI
  `36922556633`, `python-binding` `36922556590`, fresh `release-python`
  `36922890804` and `release-node` `36922895760` all green; no
  registry publication). M007 subsequently completed the C ABI design line.
- Language Bindings M005 Node binding foundation and qualification: historical
  closure remains valid at `plans/closure/language-bindings/005-status.md`
  with release-node run `36488540671` on `abc0354`. Later canonical-core
  changes made current Python/Node compatibility CI red; that post-closure
  drift is owned by verification-conformance M006 + language-bindings M006 and
  does not rewrite the original M005 evidence.
- Language Bindings M004 M003 closure corrective pass: closure recorded
  at `plans/closure/language-bindings/004-status.md`. Lightweight
  `python-binding` CI corrected to a self-contained wheel
  build + install path (run `36456938690`, success, 71 Python tests
  collected); release qualification identity corrected to the
  GitHub-recorded SHA `d40f1b37cf031b05e4f9c76af1cdfcf789d739b5` for
  run `36337194059`; all five SUPPORT wheel rows promoted to
  `Qualified`; M003 closure accepted and its stale claims removed.
- Language Bindings M003 Python corrective qualification: closure
  recorded at `plans/closure/language-bindings/003-status.md`
  (corrected by M004; corrections listed in its §13). Structured Python
  exception attributes (InsufficientCapacity + every ResourceLimit
  variant; ImageTruncated mapped to EncodeDecodeError), failure-safe
  pure-Python file helpers with optional `output_path`, lightweight
  path-filtered python-binding workflow, corrected five-platform manual
  wheel workflow with Rust provisioned inside the cibuildwheel Linux
  container and native-arch smokes, single-sdist/direct-pip-install
  proof, documentation/registry reconciliation. Five matrix wheels
  qualified natively in release run `36337194059`.
- Language Bindings M002 Python packaging and qualification: closure
  recorded at `plans/closure/language-bindings/002-status.md`.
  Conditionally closed at M002 publication; the outstanding
  four-platform native-smoke condition is satisfied by M003's evidence,
  accepted in `plans/closure/language-bindings/004-status.md` §11. The
  M002 record itself is unchanged.
- Language Bindings M001 Python binding foundation: closure recorded at
  `plans/closure/language-bindings/001-status.md`. PyO3/maturin
  leaf binding over canonical byte APIs, 57 Python tests, panic
  profile `unwind`, required `./scripts/check.sh` unchanged.
- Plan 107 planning-convention migration (this registry, `000`–`003`,
  ADRs, six original roadmaps, planning skill): `plans/107-status.md`.
- First eggfetch-enabled release 0.4.2 five-target qualification:
  `plans/105-status.md`.
- Carrier v1 packaging/documentation/evidence closure:
  `plans/097-status.md`.
- Roadmap-081 corrective closure: `plans/088-status.md`.
- Fuzz assurance corrective closure: `plans/089-status.md`.
