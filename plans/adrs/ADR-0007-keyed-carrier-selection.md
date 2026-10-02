# ADR-0007 — Keyed Carrier Placement Without Reinterpreting Legacy Seeds

Status: proposed  
Date: 2026-10-02  
Decision owners: StegoEggo maintainers  
Related specification sections: `plans/000-long-term-specification.md#2-canonical-api-invariants`, `#3-execution-invariants`  
Affected subsystem roadmaps: `plans/subsystems/stego-library-evolution-roadmap.md`

## Context

The generic `stegoeggo-stego` crate is now useful outside the rights-reservation
application, but its carrier-position selection is intentionally deterministic from
a public `u64` seed. That is sufficient for reproducibility and compatibility; it
must not be represented as secret-key security.

Generic steganography consumers may need a placement schedule derived from secret
key material so an observer without the key cannot trivially reproduce the carrier
selection sequence. This is distinct from payload confidentiality and authentication.

The current seed-based LSB/JPEG mappings are compatibility formats. Existing images,
known-answer vectors, parent verification, and legacy extraction depend on their exact
byte behavior. Reinterpreting `u64 seed` would break durable carrier compatibility.

Current ecosystem evidence reinforces the separation:

- `rand_chacha` documents portable deterministic ChaCha generators and explicitly
  warns that `seed_from_u64` is unsuitable when security matters.
- RustCrypto's `chacha20` exposes a pure-Rust 256-bit-key stream primitive, but
  correctly warns that a stream cipher alone does not authenticate data.
- Modern steganography crates such as `stenoxide-core` derive placement/permutation
  state from stronger key material while keeping encryption/authentication as a
  separate layer.

Sources reviewed 2026-10-02:
- https://docs.rs/rand_chacha/latest/rand_chacha/
- https://docs.rs/chacha20/latest/chacha20/
- https://docs.rs/stenoxide-core/latest/stenoxide_core/

## Decision drivers

1. Never change the meaning of existing seed-based carriers.
2. Keep the generic carrier API honest about what placement secrecy does and does not prove.
3. Preserve `#![forbid(unsafe_code)]` and deterministic known-answer testing.
4. Avoid forcing cryptography into callers that only need deterministic public seeds.
5. Keep LSB, JPEG, tiled, borrowed-buffer, and prepared-JPEG behavior interoperable
   under one placement-scheme contract.
6. Do not couple keyed placement to StegoEggo rights metadata, HMAC payload semantics,
   or detached-signature semantics.

## Considered options

### A. Reinterpret the existing `u64 seed` through a cryptographic RNG

Rejected. It silently breaks every existing carrier and known-answer vector while
still exposing only 64 bits of caller-supplied state.

### B. Replace all seed APIs with byte-string keys at v1

Rejected. Reproducible non-secret seeds are useful and are part of the existing
generic contract. A secret key is not required for every carrier use case.

### C. Add an explicit versioned keyed-placement scheme beside legacy seeds

Proposed. Existing seed APIs remain byte-stable. A new 256-bit key type and explicit
placement-scheme selection drive a cryptographic deterministic permutation. Callers
choose the scheme; no implicit auto-upgrade occurs.

## Proposed decision

Adopt option C.

The compatibility contract is:

- Existing seed-based LSB/JPEG/tiled mappings remain unchanged and continue to read
  every carrier they read today.
- A future public type such as `CarrierKey` owns exactly 32 bytes of key material.
  Its `Debug` representation MUST redact the bytes.
- A public placement-scheme enum/config MUST identify legacy seed placement separately
  from the new keyed scheme. The final public names are chosen during implementation,
  but the scheme identity is versioned from its first release.
- The keyed scheme MUST use a deterministic, portable cryptographic generator with
  frozen known-answer vectors. The implementation plan must benchmark and select the
  exact primitive/dependency before freezing the scheme. ChaCha20/ChaCha20Rng are the
  current leading candidates, not a pre-authorized dependency.
- Domain separation MUST distinguish at least LSB, JPEG-DCT, tiled coordinate
  derivation, and any future placement subdomains so the same key does not reuse one
  stream across logically different carrier roles.
- Placement keys influence only carrier-position selection. They DO NOT encrypt the
  payload, authenticate it, prove ownership, or make CRC32 adversarially secure.
- Framed payload bytes remain independent of placement choice unless a separately
  accepted ADR establishes a wire-format identifier. Callers performing generic
  extraction therefore provide the matching placement config explicitly.
- Parent StegoEggo rights-marker behavior stays on the existing seed-compatible scheme
  unless a later application-level plan explicitly opts into keyed placement.

## Consequences

Positive: generic consumers gain a security-appropriate placement primitive without
invalidating existing images. The distinction between reproducible seed and secret key
becomes explicit in the type system.

Negative: the carrier crate gains cryptographic dependency/supply-chain surface and
must maintain another frozen placement algorithm indefinitely after release.

Neutral: this does not improve resistance to recompression, cropping, statistical
steganalysis, or payload forgery by itself.

## Compatibility and migration

This is additive in 0.x only if legacy defaults and existing signatures retain their
behavior. No existing image is migrated. Existing `LsbConfig`, `JpegConfig`, and
`TileConfig` seed constructors remain valid. New keyed constructors/configuration are
opt-in.

If implementation cannot remain additive without changing existing defaults, stop and
defer the change to an explicit breaking-version plan.

## Security and reliability implications

Key bytes MUST never appear in `Debug`, errors, reports, tracing, examples, or CLI
output. Temporary derived material should be minimized and zeroized where practical.
The API documentation MUST state that position secrecy is not payload encryption or
authentication.

All arithmetic and bounded-search guarantees from the existing carrier surface remain
in force.

## Verification

Before acceptance/implementation:

1. Prototype at least two candidate deterministic generators and record dependency,
   MSRV, portability, and known-answer implications.
2. Prove legacy vectors are byte-identical with the new feature disabled and enabled.
3. Add independent keyed-placement known-answer vectors for LSB and JPEG.
4. Verify packed/strided pixel views, tiled paths, and `PreparedJpeg` share the same
   keyed scheme semantics.
5. Run `./scripts/check.sh`, carrier direct-consumer tests, MSRV package checks, and
   `cargo-semver-checks` evidence before release.

## Supersession

If accepted, this ADR governs keyed placement until superseded by a later ADR. It does
not supersede ADR-0002 or the seed-compatibility decisions recorded in Plans 090–097.
