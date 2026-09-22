# Plan 106 Status: Real Eggfetch-to-Eggfetch Self-Update Closure

Status: planned; blocked. Plan 105 is complete (A = 0.4.2 exists); no newer
stable eggfetch-enabled B exists yet, so the public A→B transition cannot be
executed without publishing a throwaway version, which the plan forbids.

## Objective

Prove one real public stable self-update transition where both the starting
binary and the replacement binary contain the native `eggfetch-core` updater.

## Preconditions

- Plan 105 complete: yes (0.4.2, 2026-09-22; see `plans/105-status.md`).
- Stable release A contains native eggfetch updater: yes.
  - A = `0.4.2`, tag `v0.4.2`, source `3b75c70660f7d3842288188131dd88c709130d11`.
  - `cargo info stegoeggo-cli@0.4.2` / crates.io API shows `0.4.2` stable, not yanked.
  - Source includes `eggfetch-core 0.2.0` in `stegoeggo-cli/Cargo.toml`, Rust 1.89 MSRV, native updater in `stegoeggo-cli/src/update.rs`, five-target contract.
  - GitHub Release https://github.com/eggstack/stegoeggo/releases/tag/v0.4.2 has complete five-asset matrix plus sidecars and installers.
- Newer stable release B contains native eggfetch updater: not yet.
  - crates.io max stable remains `0.4.2`; no `B > A` published.
  - Git tags max `v0.4.2`; no newer release.
- B published on crates.io with complete asset matrix: not yet.

## Required closeout evidence

Record before marking complete (all pending until B exists):

- A and B versions/tags/source SHAs: A recorded above; B TBD;
- isolated A installation method and target: TBD at B time (use pinned installer or verified asset in disposable HOME/PATH; do not overwrite primary);
- A pre-update version output: TBD (`stegoeggo A` expected);
- real public `stegoeggo update` A -> B result: TBD (must use real `STEGOEGGO_CRATES_API_URL` / `STEGOEGGO_RELEASES_URL`, capture stdout/stderr/exit without secrets);
- B post-update version output: TBD (`stegoeggo B` expected);
- protect/inspect/verify smoke after replacement: TBD;
- second update already-current result: TBD (exit 0, no download/replace/fallback);
- confirmation that no Cargo fallback occurred on the supported target: TBD;
- confirmation that external curl was not required: TBD (run with curl absent from PATH where practical);
- focused deterministic updater regression results: TBD at B time (`cargo test -p stegoeggo-cli --all-features`, `./scripts/test-release-updater.sh`; current 0.4.2 rehearsals pass, but must re-run at B);
- real Windows A -> B result if performed, otherwise explicit truthful note: TBD. Minimum is Plan 105 Windows B build/smoke plus code review; do not claim an unperformed Windows transition.

Do not substitute a fixture-server transition or a pre-Plan-103 curl-based
release for this evidence. Do not publish a meaningless stable version solely
to make A -> B available; Plan 106 executes at the first ordinary stable
release after Plan 105 that provides a newer eggfetch-enabled CLI version.
