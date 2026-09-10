# Plan 093 Status

Status: COMPLETE

Baseline: `27bdd3d429d663948021de43d3e6f818fa613319`

## Required evidence

- [x] exact accepted LSB V2 permutation domain recorded
- [x] permutation isolated/instrumented without byte change
- [x] stale 64-step diagnostic corrected
- [x] deterministic exhaustive small-domain injectivity tests added
- [x] non-power-of-two/pathological domain coverage recorded
- [x] maximum observed cycle-walk depth recorded
- [x] 256-step fallback reachability analyzed analytically
- [x] disposition A/B/C selected with proof/evidence
- [x] current known-answer vectors preserved
- [x] capacity/no-collision claims reconciled with disposition
- [ ] if a successor mapping is needed, explicit compatibility/versioning design recorded before implementation
- [x] no unsupported cryptographic/FPE claim introduced
- [x] `./scripts/check.sh` passes (local full gate green, 2026-09-10)

## Implementation notes

- Accepted domain: `slot_count = width * height * 3` via checked
  `lsb_available_slots` (`u32` dimensions, `None` on overflow);
  `stego_permutation_v2` additionally returns `None` when
  `checked_next_power_of_two` overflows (`slot_count == usize::MAX`).
  No larger abstract domain is claimed.
- Instrumentation: production `stego_permutation_v2` now delegates to one
  shared private core also serving test-only `stego_permutation_v2_debug`,
  which reports `(slot, walk_depth, used_fallback)`. Carrier output bytes
  are unchanged (same operations, verified by the untouched known-answer
  vector `tests/public_stego_api.rs:40-59`).
- Stale `64-step` diagnostics corrected to `256-step` (2 sites).
- Tested ranges/seeds: full-domain injectivity for every
  `slot_count in 2..=64`, power-of-two-adjacent and medium sizes
  `{65,100,127,128,129,255,256,257,511,512,513,1000,1023,1024,1025,2048,
  2049,4095,4096,4097,8191,8192,8193,12288,16383,16384,16385}` x seeds
  `{0,1,42,12345,0x0123456789ABCDEF,0xDEADBEEF,u64::MAX}`, plus
  `{65535,65536,65537,100000}` x `{42,u64::MAX}`; pathological
  `{2,3,5,6,7,9,17,31,33}` x seeds `0..512` plus `u64::MAX-1/u64::MAX`;
  exact-capacity 512x512 embed/extract round-trip; 32-bit overflow unit
  checks (`lsb_available_slots(u32::MAX,u32::MAX) == None`).
- Maximum observed walk depth: 30 (bound is 256). Fallback hits in all
  tested domains: 0.
- Disposition: C. The affine step over `Z_m` with arbitrary odd `a` does
  not guarantee every cycle re-enters `0..slot_count`, so true
  cycle-walking need not terminate for all `(seed, slot_count)` pairs and
  the fixed 256-step escape cannot be proven unreachable (A) over the
  `u64` seed space by testing; the `splitmix64(x) % slot_count` fallback
  is likewise not proven injective over its reaching subset (B).
  Therefore V2 is frozen byte-for-byte as the compatibility mapping, all
  `true bijection` claims are corrected to documented-domain injectivity
  plus an operational assumption elsewhere, and no new mapping is
  introduced by this plan. A versioned successor mapping requires a
  separately approved wire-format/negotiation design; that design is
  explicitly deferred, not silently skipped.
- Capacity reconciliation: `lsb_required_capacity_v2` exactness now reads
  as verified-for-documented-domains; parent `embed.rs` V2 docs, carrier
  rustdoc, architecture/AGENTS/skill wording reconciled in Plans 093+097.
- NIST SP 800-38G is not cited as an implementation claim; no FPE or
  cryptographic-security claim added. No production dependency added.

Record final `./scripts/check.sh` result and implementation commit SHA here
during closure.

Implementation commit SHA: `d42f8ba` (roadmap 090 implementation on `main`; this ledger closure is the follow-up commit).
