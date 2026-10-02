# Local reusable release/bootstrap actions

Local composite actions for genuinely repeated multi-line setup in
release/binding workflows (maintenance-quality M001). They contain no
publication steps, receive explicit inputs, and fail closed on checksum,
tool-version, or missing-artifact mismatch. Target-specific build commands
stay visible in the caller workflows so failures remain diagnosable from logs.

## Actions

| Action | Purpose | Replaces inline blocks in |
|---|---|---|
| `install-cargo-zigbuild` | Exact `cargo install cargo-zigbuild --locked` into an isolated root | `release-c.yml` (Linux matrix rows) |
| `provision-verified-zig` | Checksum-pinned Zig 0.14.1 download, archive-shape audit, binary/version assertion | `release-c.yml` (Linux matrix rows) |

`release-binaries.yml` is intentionally NOT migrated: it is Eggpack-generated
and byte-compared by `eggpack ci check` (release-drift guard), so any step
restructure breaks the drift gate. Its identical Zig/cargo-zigbuild blocks stay
inline until the Eggpack producer itself emits the shared reference. `ci.yml`,
`assurance.yml`, `fuzz.yml`, and `external-verification.yml` are out of scope
(required-CI and specialist workflows are never touched by this maintenance
line without a maintainer decision).

## Third-party action pin table (release/binding workflows)

Every external `uses:` in `release-binaries.yml`, `release-python.yml`,
`release-node.yml`, and `release-c.yml` is pinned to a full commit SHA with an
adjacent `# <upstream release>` comment. `release-binaries.yml` was already
fully pinned and is unchanged.

| Action reference | Pinned SHA | Upstream release verified |
|---|---|---|
| `actions/checkout` | `11d5960a326750d5838078e36cf38b85af677262` | v4.4.0 |
| `actions/upload-artifact` | `ea165f8d65b6e75b540449e92b4886f43607fa02` | v4.6.2 |
| `actions/download-artifact` | `d3f86a106a0bac45b974a628896c90dbdf5c8093` | v4.3.0 |
| `actions/setup-python` | `a26af69be951a213d495a4c3e4e4022e16d87065` | v5.6.0 |
| `actions/setup-node` | `49933ea5288caeca8642d1e84afbd3f7d6820020` | v4.4.0 |
| `pnpm/action-setup` | `f40ffcd9367d9f12939873eb1018b921a783ffaa` | v4 |
| `ilammy/msvc-dev-cmd` | `0b201ec74fa43914dc39ae48a89fd1d8cb592756` | v1 |
| `dtolnay/rust-toolchain` | `2c7215f132e9ebf062739d9130488b56d53c060c` | pre-existing repo pin, shared with `release-binaries.yml` / `release-drift.yml` |

SHAs verified 2026-10-02 with `git ls-remote` against each upstream
repository's release tags (`refs/tags/vX.Y.Z` matching the intended major
line). `dtolnay/rust-toolchain` reuses the SHA already proven in this repo's
binary/drift workflows; the requested toolchain itself (`stable`, `1.89`)
remains an explicit `with: toolchain:` input at each caller.

## Pin update procedure

1. Resolve the intended upstream release tag to a full SHA:
   `git ls-remote https://github.com/<owner>/<action>.git 'refs/tags/v*apl*'`.
2. Confirm the tag object matches the major-line tag (e.g. `refs/tags/v4`).
3. Update the `uses:` line and its `# <release>` comment in every affected
   workflow, plus this table and `scripts/check-release-workflow-contract.py`.
4. Never switch a pin back to a moving tag. Never touch
   `release-binaries.yml` steps without re-running `eggpack ci check`
   (see the drift note above).
5. Re-run the contract checker and record the new SHAs in the release
   evidence for the next manual qualification.

## Contract checker

`scripts/check-release-workflow-contract.py` (manual, never required CI)
asserts full-SHA pinning plus target/artifact/toolchain equivalence across
the four release/binding workflows. Run it after any edit here:

```bash
python3 scripts/check-release-workflow-contract.py
```
