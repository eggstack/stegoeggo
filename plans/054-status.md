# Plan 054 Status Ledger

Plan baseline SHA: `e683c87785d7e4ec60b17fa8ec961d983e4b6fac`

Disposition: **OPEN**

Implementation head: **UNSET**

Plan 054 owns only the residual XMP semantic-preservation and animated-WebP correctness defects identified after the Plan 053 audit.

No release, version, tag, publication, or CI expansion is authorized.

---

## Workstream state

```text
XMP qualified RDF description serialization: OPEN
XMP owned-subtree suppression: OPEN
XMP expanded-name description close handling: OPEN
XMP deterministic namespace serialization: OPEN
XMP structural merge without substring parsing: OPEN
XMP exact filtered-description deduplication: OPEN
XMP semantic three-round idempotence: OPEN
XMP safe scoped-prefix behavior: OPEN
ANMF uint24/header decoding: OPEN
ANMF frame flag validation: OPEN
ANMF rectangle/canvas bounds: OPEN
ANMF nested cursor/pad exactness: OPEN
ANMF ALPH/VP8L order independence: OPEN
Animated top-level ALPH rejection: OPEN
Malformed top-level VP8L propagation: OPEN
Malformed nested VP8L propagation: OPEN
Animated alpha/feature derivation: OPEN
Valid animated metadata rewrite: OPEN
Focused verification: OPEN
Workspace verification: OPEN
Publication hold: RETAINED
```

---

## Defect ledger

| item | baseline behavior | exact closure contract | implementation SHA | focused evidence | disposition |
|---|---|---|---|---|---|
| RDF description qualification | preserved description serialized as bare `Description` | serialize an RDF-qualified description that reparses with RDF expanded name | — | `preserved_description_remains_rdf_qualified` | OPEN |
| Owned subtree filtering | nested events can escape an owned element skip | suppress the entire owned element subtree until matching end | — | `owned_other_constraints_with_rdf_alt_is_removed_whole` | OPEN |
| Description close identity | local-name-only close handling | require RDF namespace + local `Description` | — | expanded-name close fixture | OPEN |
| Namespace self-containment | inherited bindings are partially reconstructed | every serialized preserved description reparses standalone | — | outer/RDF namespace fixtures | OPEN |
| Structural XMP merge | `metadata_trap.rs` uses `find/rfind` on `rdf:RDF` | insert preserved descriptions through XML events | — | structural merge fixtures | OPEN |
| Description dedup | filtered descriptions are appended without exact dedup | deduplicate byte-identical filtered serialization, preserving first order | — | identical/differing packet fixtures | OPEN |
| Semantic idempotence | current test checks one chunk + substring presence | parser-based 3-round uniqueness/preservation proof | — | semantic three-round fixture | OPEN |
| Scoped prefix reuse | global prefix conflict can over-reject | sibling self-contained scopes may reuse textual prefix for different URI | — | safe sibling-prefix fixture | OPEN |
| ANMF header | first four bytes treated as reserved | decode X/Y/W/H/duration/flags from all 16 bytes | — | header decode fixture | OPEN |
| ANMF flags | real reserved/blend/dispose layout not validated | require `(flags & 0xFC) == 0`, preserve bits 1/0 semantics | — | reserved flag fixture | OPEN |
| Frame rectangle | not validated against canvas | checked `x + width <= canvas_width`, `y + height <= canvas_height` | — | edge/outside fixtures | OPEN |
| Nested exactness | basic nested bounds exist | exact header/payload/pad/final cursor and one image payload | — | overrun/pad/cursor fixtures | OPEN |
| ALPH + VP8L ordering | rejection depends on encounter order | reject combination regardless of order | — | both-order fixtures | OPEN |
| Animated top-level ALPH | not explicitly forbidden | reject top-level ALPH whenever coherent animation path is used | — | top-level ALPH animation fixture | OPEN |
| Top-level malformed VP8L | parse error can be ignored during feature derivation | malformed classified VP8L fails | — | malformed top-level VP8L fixture | OPEN |
| Nested malformed VP8L | parse error can be ignored during frame feature derivation | malformed nested VP8L fails | — | malformed nested VP8L fixture | OPEN |
| Animation feature derivation | partial frame alpha support | derive alpha/animation only from validated payload semantics | — | opaque/alpha frame fixtures | OPEN |
| Animated rewrite | parser tests do not fully prove writer behavior | valid animation rewrites with unchanged ANMF payloads and correct flags | — | end-to-end animation fixtures | OPEN |

---

## Verification ledger

| command | observed result | exact SHA | status |
|---|---|---|---|
| `cargo test -p stegoeggo xmp --all-features` | not run for implementation | — | OPEN |
| `cargo test -p stegoeggo webp --all-features` | not run for implementation | — | OPEN |
| `cargo test -p stegoeggo --test conformance_container_tests --all-features` | not run for implementation | — | OPEN |
| `cargo fmt --all -- --check` | not run for implementation | — | OPEN |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | not run for implementation | — | OPEN |
| `cargo check -p stegoeggo --no-default-features` | not run for implementation | — | OPEN |
| `cargo test --workspace --exclude stegoeggo-fuzz --all-features` | not run for implementation | — | OPEN |
| `./scripts/check.sh` | not run for implementation | — | OPEN |

---

## Closure rule

Plan 054 may be marked `COMPLETE` only when every defect row is backed by focused evidence and all required verification commands are recorded against the actual implementation head.

Plan 054 completion does **not** close Roadmap 045 or Plans 051-053. Plan 055 owns the remaining JPEG structural exactness and final cross-plan evidence reconciliation.
