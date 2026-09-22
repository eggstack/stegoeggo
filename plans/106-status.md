# Plan 106 Status: Real Eggfetch-to-Eggfetch Self-Update Closure

Status: planned; blocked until Plan 105 is complete and a second stable
eggfetch-enabled CLI release exists.

## Objective

Prove one real public stable self-update transition where both the starting
binary and the replacement binary contain the native `eggfetch-core` updater.

## Preconditions

- Plan 105 complete.
- Stable release A contains native eggfetch updater.
- Newer stable release B contains native eggfetch updater.
- B is published on crates.io and has a complete GitHub Release asset matrix.

## Required closeout evidence

Record before marking complete:

- A and B versions/tags/source SHAs;
- isolated A installation method and target;
- A pre-update version output;
- real public `stegoeggo update` A -> B result;
- B post-update version output;
- protect/inspect/verify smoke after replacement;
- second update already-current result;
- confirmation that no Cargo fallback occurred on the supported target;
- confirmation that external curl was not required;
- focused deterministic updater regression results;
- real Windows A -> B result if performed, otherwise an explicit truthful
  note that Windows replacement was not directly exercised.

Do not substitute a fixture-server transition or a pre-Plan-103 curl-based
release for this evidence.
