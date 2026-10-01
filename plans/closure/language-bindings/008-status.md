# Language Bindings Milestone 008 — Closure Status

Status: closed
Source implementation plan: `plans/implementation/language-bindings/008-m007-symbol-inventory-corrective.md`
Source subsystem roadmap: `plans/subsystems/language-bindings-roadmap.md#m008--m007-symbol-inventory-corrective`
Repository baseline reviewed: `839c4c8` (work-start HEAD; plan baseline `c5fcbf5974ca2ac52ba6d071eaf4238335930c5a`)
Implementation commit: `6cbc0d4` — fix: correct C ABI v1 symbol inventory 89->90 with checked manifest (language M008 implementation)
Closing commit: this record plus roadmap/registry reconciliation (see git log for `plans: close language-bindings M008 symbol-inventory corrective`).

## 1. Executive finding

The M007 bookkeeping defect is corrected without any semantic change:
`bindings/c/ABI-V1.md` always declared 90 unique callable symbols
(3 bootstrap + 87 `stegoeggo_v1_*`, execution report 10 — not 9) while
its arithmetic claimed 89 (3 + 86). All count text now says 90/87, the
sorted 90-line `bindings/c/abi-v1-symbols.txt` manifest is materialized,
and the dependency-free `bindings/c/scripts/check-contract.py` proves
document↔manifest equality (90 total, 3 bootstrap, 87 v1, no
duplicates, no drift). The 90-signature C11/C++17 review smoke is green
on Apple clang 21.0.0. No function was added, removed, renamed, or
resigned; no Crate, header, or library was introduced. No acceptance
criterion remains outstanding; M008 closes and M009 becomes
dependency-ready.

## 2. Requirement-to-evidence matrix

| Plan requirement (§6/§13) | Evidence |
|---|---|
| Correct 89→90 and 86→87 full/v1 inventory | `ABI-V1.md` count line now `90` (3 bootstrap + 87 v1) plus explicit category totals; §15/§16 `89`→`90` (§3) |
| Correct execution-report heading to 10 | §12.1 heading now `(10 symbols)`; the 10 declarations underneath unchanged (§3) |
| Recalculate and document category arithmetic | New category-totals line: notice 20, request 18, limits 20, operations 4, buffer 3, error 6, execution report 10, verification report 6 = 87 v1 + 3 bootstrap = 90 (§3) |
| `abi-v1-symbols.txt`: exactly 90 sorted unique names | 90 lines, `sort -c` clean, 3 bootstrap + 87 `stegoeggo_v1_` (§4) |
| Deterministic contract checker | `bindings/c/scripts/check-contract.py` exits 0; parses only fenced C declarations; fails on drift/duplicates/count mismatch (§4) |
| Re-run C11/C++17 smoke over all 90 declarations | Review-only `/tmp` sketch + consumer naming every manifest symbol: `cc -std=c11` and `c++ -std=c++17` `-Wall -Wextra -Werror -fsyntax-only` both exit 0 on Apple clang 21.0.0 (§4) |
| Correct M007 closure, STABILITY, roadmap, registry | `007-status.md` numbers corrected with explicit M008 note; `STABILITY.md` and `README.md` say 90; roadmap/registry reconciled here (§9) |
| No semantic ABI change | `git diff` on `ABI-V1.md` touches only count prose; zero declaration lines changed (§3) |
| `./scripts/check.sh` green | Exit 0 at the implementation commit (§4) |

## 3. Production implementation evidence

Commit `6cbc0d4` (6 files, +204/−15):

- `bindings/c/ABI-V1.md` (+7/−4): three count corrections plus the new
  category-totals line. Every function prototype byte-identical.
- `bindings/c/abi-v1-symbols.txt` (new, 90 lines): bytewise-sorted
  normative symbol list; SHA-256
  `faa8b671429ce6db4d9cf819c0b33f6010e87a468ba0ac1dfa889ba98da06526`.
- `bindings/c/scripts/check-contract.py` (new, executable): stdlib-only
  checker as specified by the plan.
- `bindings/c/README.md`, `STABILITY.md` (1 line each): 89→90.
- `plans/closure/language-bindings/007-status.md`: stale 89/86/9
  numbers corrected to 90/87/10 plus an explicit M008 correction note;
  M007 remains closed history.

No Rust source touched; no crate/header/library shipped, per plan §4.

## 4. Verification executed (exact commands + results)

At work-start HEAD `839c4c8` plus the implementation commit:

```bash
./scripts/check.sh   # exit 0; closes with `./scripts/check-docs-contract.sh`: "Documentation contracts valid: 5 release targets"
python3 bindings/c/scripts/check-contract.py   # "ABI v1 contract OK: 90 total (3 bootstrap + 87 v1)", exit 0
wc -l bindings/c/abi-v1-symbols.txt   # 90
sort -c bindings/c/abi-v1-symbols.txt   # clean (exit 0)
grep -c '^stegoeggo_v1_' bindings/c/abi-v1-symbols.txt   # 87
```

Negative checker proof (temporary manifest truncation to 89 lines,
restored afterwards): checker exits 1 reporting the missing
`stegoeggo_v1_verify`, total 89 ≠ 90, v1 86 ≠ 87; after restore it
exits 0.

Review-only syntax smoke (sketch + consumer generated in `/tmp` from
the corrected document, naming all 90 manifest symbols; never
committed):

```bash
cc -std=c11 -Wall -Wextra -Werror -fsyntax-only /tmp/m008_consumer.c   # OK
c++ -std=c++17 -Wall -Wextra -Werror -fsyntax-only /tmp/m008_consumer.cpp   # OK
# Apple clang 21.0.0 (clang-2100.1.1.101), both compilers
```

Declaration-set immutability proof: the `ABI-V1.md` diff contains no
added/removed/altered prototype line — only the four count-prose hunks
(§0 totals + category line, §12.1 heading, §15, §16).

## 5. Invariant review

- All 90 C signatures byte-identical to the M007 design; seven opaque
  handle types, numeric codes, and `has_dmi` presence semantics
  unchanged; ABI major/minor stay 1.0.
- No Rust FFI source, generated header, or library introduced; ADR-0006
  untouched; Python/Node unaffected.
- Root required CI gained no C/cbindgen prerequisite (checker is
  stdlib-only Python, run manually, not wired into `check.sh`).

## 6. Failure and recovery review

- Any document↔manifest mismatch blocks closure by checker exit 1; the
  negative proof above exercises exactly that gate.
- The mismatch was resolved by fixing arithmetic to the declared set,
  never by deleting `execution_report_has_dmi` or any other function.
- No second semantic contradiction surfaced during the pass, so no new
  corrective/ADR was needed.

## 7. Migration and compatibility review

No C ABI exists in the wild yet, so there is no consumer migration.
The contract consumers (M009/M010) now pin to the checked 90-symbol
manifest instead of hand-counted prose. ABI major/minor stay 1.0.

## 8. Security review

Design-evidence-only change: no parser, crypto, key handling, or
`unsafe` involved. The public checker reads two text files and prints
one line; no secret material exists anywhere on this surface.

## 9. Documentation and operations

- `bindings/c/ABI-V1.md`: counts corrected, category arithmetic stated.
- `bindings/c/abi-v1-symbols.txt` + `bindings/c/scripts/check-contract.py`:
  new machine-checked source of truth; M009/M010 reuse it as the export
  oracle.
- `bindings/c/README.md`, `STABILITY.md`: 90-symbol wording.
- `plans/closure/language-bindings/007-status.md`: factual correction
  with an explicit M008 trail (history preserved, not rewritten).

## 10. Unresolved findings (critical/high/medium/low)

None. One observation for M009 hygiene (low): the `ABI-V1.md` header
banner still names M008 as the implementer milestone and the §14/§16
text still says "unless M008 qualifies it" / "First M008 matrix" where
M009/M010 now own that work. Those labels predate the M008/M009/M010
split and do not affect the normative symbol/ownership contract; M009
may correct the milestone labels as a documentation touch-up while
proving the declaration set unchanged with this same checker.

## 11. Roadmap disposition

Language-bindings M008 is closed with all required evidence accepted.
The M009 C ABI implementation-foundation hard dependency (corrected,
automated 90-symbol source of truth) is now satisfied: M009 becomes
dependency-ready and may be handed off. M010 remains blocked on M009
closure. No new subsystem-local milestone is required by this closure.

## 12. Registry updates

- `plans/registry.md`: language-bindings M008 implementation plan
  `ready` → `closed` with this closure record linked; subsystem current
  milestone notes M008 closed and M009 dependency-ready; M009 blocker
  row removed; M010 blocker row repointed to M009 closure; recent-closure
  entry added.
- `plans/subsystems/language-bindings-roadmap.md`: §7 M008 text closed;
  §12 M008 row `ready` → `closed` with this closure record linked; M009
  row blocker cleared to "M008 manifest accepted".
- `plans/implementation/language-bindings/009-c-abi-v1-implementation-foundation.md`:
  `blocked` → `ready for handoff`.
