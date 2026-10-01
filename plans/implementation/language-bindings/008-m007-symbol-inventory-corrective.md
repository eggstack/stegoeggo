# Language Bindings Milestone 008 — M007 Symbol-Inventory Corrective

Status: ready for handoff

Repository baseline: `c5fcbf5974ca2ac52ba6d071eaf4238335930c5a`

Source roadmap:
`plans/subsystems/language-bindings-roadmap.md#m008--m007-symbol-inventory-corrective`

Corrects:

- `plans/closure/language-bindings/007-status.md`
- `bindings/c/ABI-V1.md`
- the C ABI count references in `STABILITY.md`, the language-bindings
  roadmap, and `plans/registry.md`

Long-term requirements:

- `plans/000-long-term-specification.md#2-canonical-api-invariants`
- `plans/000-long-term-specification.md#3-execution-invariants`
- `plans/000-long-term-specification.md#5-release-invariants`

Applicable ADRs:

- `plans/adrs/ADR-0005-foreign-language-bindings.md`
- `plans/adrs/ADR-0006-versioned-c-abi.md`

Primary class: invariant

## 1. Objective

Correct the accepted M007 C ABI v1 symbol inventory before implementation
begins.

The normative contract currently states that ABI v1 contains 89 symbols:
3 unversioned bootstrap symbols plus 86 `stegoeggo_v1_*` symbols. Direct
enumeration of the function declarations in `bindings/c/ABI-V1.md` yields
90 unique callable symbols: 3 bootstrap plus 87 v1 symbols.

The discrepancy is isolated to bookkeeping around the execution-report
surface. Section 12.1 is labeled "Execution report (9 symbols)" but declares
10 distinct functions:

1. `execution_report_effective_policy`
2. `execution_report_has_dmi`
3. `execution_report_effective_dmi`
4. `execution_report_metadata_injected`
5. `execution_report_stego_attempted`
6. `execution_report_stego_succeeded`
7. `execution_report_format_transcoded`
8. `execution_report_warning_count`
9. `execution_report_warning_at`
10. `execution_report_free`

M008 must reconcile every normative/evidence reference to the actual
90-symbol design and create a machine-readable symbol manifest so M009/M010
cannot repeat the manual-count defect.

No ABI function is added, removed, renamed, or semantically changed.

## 2. Why this milestone is ready

M007 is closed and the defect is concrete, deterministic, and bounded.

Evidence on `c5fcbf59`:

- `ABI-V1.md` declares 90 unique callable `stegoeggo*` function names;
- the document claims 89 total at its top-level count, header-generation
  contract, and symbol-manifest contract;
- section 12.1 contains 10 declarations while its heading says 9;
- the M007 closure repeats the 89/86 arithmetic;
- `STABILITY.md` and planning surfaces repeat the stale 89 count.

The extra execution-report function is not redundant. The
`effective_dmi` value uses `DMI_UNSPECIFIED` as a legitimate scalar value,
so `has_dmi` remains necessary to distinguish absence from an explicit
unspecified value. Removing that function to force the old count would weaken
the accepted report contract.

No new ADR is required because ADR-0006 fixes architecture, not the arithmetic
count.

## 3. Current implementation evidence

M007 correctly remained design-only:

- no `bindings/c/Cargo.toml`;
- no Rust FFI source;
- no generated `stegoeggo.h`;
- no dynamic/static library;
- no C-specific CI/release workflow.

The closure commit `c5fcbf59` has green standard CI and release-drift guard.

The semantic contract is otherwise internally consistent. This corrective is
not permission to reopen ownership, error, panic, threading, or report-shape
decisions.

## 4. Invariants that must not regress

- The exact function signatures in `ABI-V1.md` remain unchanged.
- The seven opaque handle types remain unchanged.
- Numeric ABI constants remain unchanged.
- Execution-report `has_dmi` remains present.
- ABI major/minor remain 1.0.
- No C implementation or generated shipping header is introduced.
- No stable ABI is claimed before implementation/qualification closes.
- ADR-0006 remains accepted and unchanged.
- Python and Node remain unaffected.
- Root required CI gains no C/cbindgen prerequisite.

## 5. Scope

### In scope

- Correct 89 -> 90 and 86 -> 87 where they describe the full/v1 function
  inventory.
- Correct "Execution report (9 symbols)" -> "(10 symbols)".
- Recalculate and document the category arithmetic.
- Add `bindings/c/abi-v1-symbols.txt` containing exactly the 90 normative
  symbol names, one sorted symbol per line.
- Add a small deterministic contract-check script under
  `bindings/c/scripts/` that:
  - extracts callable `stegoeggo*` names from `ABI-V1.md`;
  - compares the unique sorted set to `abi-v1-symbols.txt`;
  - asserts 90 total and 87 `stegoeggo_v1_*`;
  - fails on duplicate declarations or manifest drift.
- Re-run the C11/C++17 signature-sketch smoke using all 90 declarations.
- Correct factual count references in:
  - `plans/closure/language-bindings/007-status.md`;
  - `STABILITY.md`;
  - `plans/subsystems/language-bindings-roadmap.md`;
  - `plans/registry.md`.
- Write M008 closure evidence.

### Explicitly out of scope

- Changing any ABI function signature or numeric code.
- Removing `execution_report_has_dmi`.
- Adding new ABI functions.
- Creating the Rust C binding crate.
- Running cbindgen over Rust source.
- Producing a shared library or shipping header.
- M009 implementation work.
- M010 cross-platform qualification work.

## 6. Required production/design changes

### A. Correct the normative count

Update every count in `bindings/c/ABI-V1.md` to:

- bootstrap: 3;
- notice: 20;
- request: 18;
- resource limits: 20;
- operations: 4;
- buffer: 3;
- error: 6;
- execution report: 10;
- verification report: 6;
- versioned v1 total: 87;
- grand total: 90.

Do not alter the underlying 90 declarations.

### B. Materialize the manifest now

Create `bindings/c/abi-v1-symbols.txt`.

Requirements:

- exactly 90 lines;
- unique;
- bytewise sorted;
- exactly 3 unversioned bootstrap names;
- exactly 87 names beginning `stegoeggo_v1_`;
- no constants/type names in the file;
- the manifest is the future export-audit oracle for M009/M010.

This changes the M007 statement that the manifest would first be created during
implementation. That was an operational timing choice, not architecture. The
corrective creates it earlier specifically to prevent future count drift.

### C. Add deterministic contract checking

Prefer a dependency-free Python or POSIX-shell script. It is a specialist
binding check, not part of `./scripts/check.sh`.

The checker must parse only fenced C declarations or otherwise avoid counting
pseudocode mentions in prose/examples.

It must fail if:

- ABI document declarations and manifest differ;
- declaration names duplicate;
- total != 90;
- bootstrap != 3;
- v1 != 87.

M009 will reuse this checker after Rust/header generation.

### D. Correct historical evidence transparently

Do not silently rewrite M007 history.

In `plans/closure/language-bindings/007-status.md`:

- correct the stale numeric claims;
- add a short explicit correction note referencing language-bindings M008 and
  explaining that the function set/signatures were unchanged, only arithmetic
  evidence was wrong.

Update `STABILITY.md` and roadmap/registry counts to 90.

### E. Re-run the design smoke

Regenerate the review-only C signature sketch from the corrected 90-function
set and compile it as C11 and C++17.

The sketch remains outside the repository and is not a shipping header.

## 7. Ordered work packages

### WP1 — Freeze the corrected inventory

Correct all count arithmetic in `ABI-V1.md`.

Acceptance:

- 90/87 arithmetic matches category sums;
- execution-report heading says 10;
- no function declaration diff other than surrounding count text.

### WP2 — Add manifest + checker

Create `abi-v1-symbols.txt` and the deterministic consistency checker.

Acceptance:

- checker exits 0;
- manifest has exactly 90 unique sorted names;
- deliberate temporary removal/addition causes checker failure.

### WP3 — Correct historical/planning references

Correct M007 closure, STABILITY, roadmap, and registry with an explicit M008
correction trail.

Acceptance:

- no active authoritative document claims 89 symbols;
- M007 remains closed historical design work, corrected by M008.

### WP4 — Re-run syntax evidence

Compile all 90 signatures in C11/C++17 review smoke.

Acceptance:

- both compilers exit 0;
- exact compiler/version recorded in M008 closure.

### WP5 — Close and unblock implementation

Write `plans/closure/language-bindings/008-status.md`.

Only after accepted closure may M009 become ready.

## 8. Failure, cancellation, restart, and contention semantics

- Any mismatch between document declarations and manifest blocks closure.
- Do not resolve mismatch by deleting a function unless a separate durable
  architecture decision requires it.
- Do not let M009 implementation silently choose 89 or 90 while M008 remains
  open.
- If another real ABI semantic contradiction is discovered during the pass,
  stop and write a new corrective/ADR as appropriate rather than hiding it in
  arithmetic cleanup.
- Documentation-only commits after successful syntax smoke do not invalidate
  the smoke if the declaration set is proven unchanged.

## 9. Compatibility and migration

No external C ABI exists yet, so there is no consumer migration.

This corrective changes only the design evidence from an internally
inconsistent 89-symbol claim to the already-declared 90-symbol contract.

ABI major/minor stay 1.0.

## 10. Required tests

1. manifest contains 90 lines;
2. manifest lines are unique;
3. manifest sorted;
4. three bootstrap symbols present;
5. 87 v1 symbols present;
6. execution-report category contains 10;
7. ABI document extraction equals manifest exactly;
8. no old full-inventory "89" claim remains in active C ABI docs;
9. no old "86 v1" claim remains;
10. C11 syntax smoke includes every manifest symbol;
11. C++17 syntax smoke includes every manifest symbol.

## 11. Required verification commands

Minimum:

```bash
./scripts/check.sh
python3 bindings/c/scripts/check-contract.py
wc -l bindings/c/abi-v1-symbols.txt
sort -c bindings/c/abi-v1-symbols.txt
grep -c '^stegoeggo_v1_' bindings/c/abi-v1-symbols.txt
```

Expected manifest results:

- `wc -l` => 90;
- v1 grep => 87.

Review-only smoke:

```bash
cc -std=c11 -Wall -Wextra -Werror -fsyntax-only /tmp/m008_consumer.c
c++ -std=c++17 -Wall -Wextra -Werror -fsyntax-only /tmp/m008_consumer.cpp
```

Record actual compilers/versions.

## 12. Documentation updates

Update:

- `bindings/c/ABI-V1.md`;
- `bindings/c/README.md` only if it repeats the count;
- `bindings/c/abi-v1-symbols.txt`;
- `plans/closure/language-bindings/007-status.md` factual correction note;
- `STABILITY.md`;
- language-bindings roadmap;
- registry;
- new `plans/closure/language-bindings/008-status.md`.

## 13. Acceptance criteria

M008 closes only when:

- all authoritative count references say 90 total / 87 v1;
- execution-report count is 10;
- manifest exactly equals the normative declaration set;
- automated checker proves the equality;
- C11/C++17 90-signature smoke is green;
- no semantic ABI signature/code/ownership change occurred;
- root `./scripts/check.sh` is green;
- M008 closure explicitly records the M007 factual correction.

Then M009 C ABI implementation foundation becomes dependency-ready.

## 14. Stop conditions

Stop and report if:

- declaration extraction yields a semantic ambiguity rather than a count bug;
- correcting the inventory requires changing a function signature;
- a duplicate symbol name is discovered;
- one of the 90 functions cannot be represented in C11/C++17 as already
  designed;
- any other ABI-level TBD/contradiction appears.

## 15. Closure evidence required

Record:

- exact corrected commit(s);
- before/after arithmetic;
- final category counts;
- manifest SHA/content count;
- checker command/result;
- proof declaration set did not change;
- C11/C++17 compiler versions/results;
- `./scripts/check.sh` result;
- corrected M007 closure/STABILITY/roadmap/registry references;
- unresolved findings;
- transition making M009 ready.

## 16. Handoff notes

This is intentionally narrow.

The accepted function set is already 90 functions. Fix the evidence and add a
machine-checkable source of truth; do not redesign the ABI.

The manifest created here becomes the export oracle for M009 local
implementation checks and M010 five-platform qualification.
