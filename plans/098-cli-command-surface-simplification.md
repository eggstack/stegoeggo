# Plan 098: CLI Command Surface Simplification

## Goal

Simplify `stegoeggo` from a dense flag-first interface into a small, discoverable command-oriented CLI while preserving 0.x compatibility. Keep one canonical protection request builder and avoid changing image-processing semantics.

The target user-facing model is:

```text
stegoeggo protect <INPUT>...
stegoeggo inspect <IMAGE>
stegoeggo verify <IMAGE>
stegoeggo version
stegoeggo update
```

Existing root protection syntax (`stegoeggo image.png ...`) remains accepted during 0.x as a compatibility alias for `protect`. Existing `--verify` remains accepted during 0.x but is hidden/de-emphasized and translated to the new inspection/verification path.

## Why

The current root parser mixes protection policy, legacy compatibility syntax, legal metadata, image encoding, execution controls, and verification into one help page. The implementation already has the correct internal normalization point (`request::build_protection_request_with_explicit_options`), so the cleanup should be primarily argument topology and presentation rather than processing refactoring.

The modern policy/channel vocabulary is already established (`--rights-policy`, `--preset`, `--hidden-marker`, `--authentication`). `--level`, `--profile`, `--dmi`, shorthand AI/TDM flags, `--metadata`, and `--legal-claims` are compatibility syntax and should no longer have equal prominence in default help.

## Scope

In scope:

- Add first-class `protect`, `inspect`, and `verify` commands.
- Preserve root protect invocation compatibility for all 0.x releases.
- Preserve `--verify` compatibility, but hide or clearly mark it legacy.
- Split inspection from assertion-oriented verification.
- Group help by concept using clap headings/flattened structs.
- Hide or move deprecated/legacy flags into a legacy help section.
- Clarify canonical versus compatibility options in docs and generated help.
- Establish meaningful exit behavior for `verify`.
- Keep JSON behavior stable where practical and version/document any intentionally changed shape.
- Add parser and help regression tests.

Out of scope:

- New steganography algorithms.
- Generic arbitrary-payload CLI commands for `stegoeggo-stego`.
- Removal of legacy 0.x syntax.
- Changes to `ProtectionRequest` semantics unless required to remove accidental CLI-only duplication.
- Installer/update implementation (Plans 099-100).

## Files to Inspect

- `stegoeggo-cli/src/args.rs`
- `stegoeggo-cli/src/main.rs`
- `stegoeggo-cli/src/request.rs`
- `stegoeggo-cli/src/protect.rs`
- `stegoeggo-cli/src/verify.rs`
- `stegoeggo-cli/src/output.rs`
- `stegoeggo-cli/src/manifest.rs`
- `stegoeggo-cli/Cargo.toml`
- `docs/cli-usage.md`
- `architecture/cli.md`
- `DEPRECATIONS.md`
- `STABILITY.md`
- CLI integration tests

## Workstream A: Command Topology

Introduce an explicit command enum with at least:

```text
protect
inspect
verify
version
update
```

Feature-gated signing commands may remain flat initially (`keygen`, `sign`, `verify-manifest`) to avoid combining two migrations in one pass. A later v1 cleanup may group them under `key` / `manifest` if that materially improves discoverability.

Compatibility rule:

- If the first positional token is not a recognized command, parse the invocation as legacy implicit `protect`.
- Do not silently reinterpret a misspelled command as a filename when doing so would be obviously ambiguous. Add tests around command/file collisions.
- Keep one protection execution path after parsing.

Preferred examples:

```bash
stegoeggo protect image.png --rights-policy prohibited-ai-ml-training --preset legal-notice
stegoeggo inspect image.png
stegoeggo verify image.png --key @key.bin
```

## Workstream B: Argument Grouping

Refactor the current monolithic `Args` into clap `Args` groups/flattened structs such as:

```text
ProtectionPolicyArgs
RightsMetadataArgs
ImageEncodingArgs
ExecutionArgs
LegacyCompatibilityArgs
```

Default `protect --help` should visually group options under headings similar to:

- Policy and evidence
- Rights metadata
- Image output
- Execution
- Compatibility (legacy; preferably hidden from short help)

Canonical options should be prominent:

```text
--rights-policy
--preset
--hidden-marker
--authentication
--key
```

Compatibility options should remain parseable but not dominate normal help:

```text
--level
--profile
--dmi
--no-ai-training
--no-genai-training
--tdm-reserved
--metadata
--legal-claims
```

Use clap `hide = true`, `hide_short_help`, or an explicit legacy heading according to what produces the clearest generated output. Prefer `--help` to remain concise; if useful, expose compatibility syntax in `--help-long` and documentation.

## Workstream C: Inspect versus Verify

Define two distinct operations:

### `inspect`

Read-only reporting. It should:

- extract and display rights metadata;
- report detected policy and evidence channels;
- report hidden marker presence/integrity when available;
- report authentication state when a key is supplied;
- remain useful when the image is unprotected;
- normally exit 0 when the image could be parsed and inspected, regardless of whether protection is present.

This is the natural replacement for most current `--verify` interactive use.

### `verify`

Assertion-oriented verification. It should:

- use the same canonical verification report as `inspect`;
- return non-zero for integrity/authentication failure according to existing exit-code conventions;
- make the success criterion explicit and documented;
- support `--json` for scripting;
- avoid inventing legal conclusions.

Recommended exit contract:

```text
0 success / requested verification condition satisfied
1 runtime or I/O error
2 invalid invocation/configuration
3 integrity/authentication verification failure
5 unexpected/internal failure
```

Do not reuse the current `--verify always exits 0` behavior for the new `verify` command.

Compatibility `--verify` may continue old exit semantics during 0.x if required by stability commitments; document the distinction clearly.

## Workstream D: Preset and Legacy Cleanup Audit

Before changing labels, verify whether `authenticated-provenance` and `maximal` resolve to materially distinct `ProtectionRequest` values. The current documentation presents the same metadata/marker/HMAC channel set for both. If they are functionally identical:

- do not remove either in 0.x;
- document the equivalence;
- mark one as a v1 consolidation candidate in `DEPRECATIONS.md`;
- avoid adding new behavior solely to justify a name.

Audit all legacy shorthands and ensure they are translation-only. New functionality must not be added to legacy fields.

## Workstream E: Help Quality

Add snapshot/string assertions for generated help. At minimum verify:

- root help presents commands first and does not dump every protection flag;
- `protect --help` emphasizes canonical policy/evidence controls;
- `inspect --help` contains only inspection-relevant options;
- `verify --help` clearly describes exit behavior;
- `version` and `update` have concise command descriptions;
- legacy flags remain accepted but do not dominate default help;
- signing commands appear only when the feature is compiled.

Keep descriptions short enough to scan in an 80-120 column terminal.

## Workstream F: Compatibility Tests

Add tests proving old and new protection invocations resolve to equivalent requests/output for representative cases:

```text
implicit_root_protect_matches_protect_command
legacy_profile_translates_under_protect
legacy_dmi_translates_under_protect
legacy_verify_still_parses
inspect_is_read_only
verify_integrity_failure_exits_3
protect_help_hides_legacy_noise
root_help_is_command_oriented
```

Cover files named `protect`, `verify`, and `version` where path ambiguity may exist. Require `./protect` or `--` when necessary and document the behavior rather than adding fragile heuristics.

## Documentation Changes

Update:

- `README.md` quickstart to use `stegoeggo protect` and `stegoeggo inspect`.
- `docs/cli-usage.md` to make command-oriented syntax canonical.
- `architecture/cli.md` with parsing/compatibility routing.
- `DEPRECATIONS.md` with hidden compatibility syntax and v1 removal candidates.
- `STABILITY.md` if command/exit compatibility needs an explicit statement.
- `AGENTS.md` CLI essentials after implementation.

Do not retain examples that teach legacy syntax as the preferred path.

## Required Checks

```bash
./scripts/check.sh
cargo test -p stegoeggo-cli --all-features
cargo run -p stegoeggo-cli -- --help
cargo run -p stegoeggo-cli -- protect --help
cargo run -p stegoeggo-cli -- inspect --help
cargo run -p stegoeggo-cli -- verify --help
```

Also run representative old/new invocation equivalence smokes on PNG, JPEG, and WebP.

## Acceptance Criteria

- Root help is substantially shorter and command-oriented.
- New users can discover protect/inspect/verify without reading external docs.
- Existing 0.x protection invocations continue to work.
- Legacy policy flags are accepted but visually de-emphasized.
- `inspect` and `verify` have distinct, documented semantics.
- `verify` returns meaningful process status.
- Protection still normalizes through one canonical `ProtectionRequest` builder.
- No duplicate processing pipeline is introduced.
- README/docs/architecture describe the same command model as the binary.

## Dependency / Ordering

Implement before Plan 100 so `version` and `update` land in the final command topology rather than being bolted onto the legacy root parser.
