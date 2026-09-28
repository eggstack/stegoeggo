# stegoeggo Node.js bindings

Node.js frontend for the [`stegoeggo`](https://docs.rs/stegoeggo) Rust library.
Protect and verify rights-reservation metadata (with optional steganographic
markers) on PNG, JPEG, and WebP encoded bytes.

> **Status: experimental / local source build.** This package is not published
> to npm. M005 in
> `plans/implementation/language-bindings/005-node-binding-foundation-qualification.md`
> is the foundation and qualification milestone; the five native platform
> artifacts are produced by the manually-dispatched
> `.github/workflows/release-node.yml`, which performs no registry
> publication.

The Node surface is a thin projection of the canonical Rust byte API
(`process_request_bytes*` and `verify_image_bytes_report`). It does not
reimplement protection logic; the Rust crate is the single semantic source
of truth. Deprecated `ProtectionLevel`, `ProtectionContext`, and
`EvidenceProfile` adapters are intentionally not projected.

## Encoded bytes are required for metadata

Use the byte-oriented operations (`protect`, `protectWithWarnings`,
`protectWithReport`, `verify`) on PNG/JPEG/WebP encoded buffers. Pixel-only
round-trips through decode/encode strip container metadata.

CPU-heavy protection and verification run on libuv worker threads and return
Promises, so the JavaScript event loop stays free. Input bytes are copied
into Rust-owned storage before worker dispatch: mutating the caller's
`Buffer`/`Uint8Array` after the call cannot change the processed bytes.
There is no synchronous `protectSync`/`verifySync`.

## Install (local source build)

Prerequisites: Rust 1.89+, Node >= 22.13.0, pnpm 12.6.0 (via corepack).

```bash
cd bindings/node
pnpm install --frozen-lockfile
pnpm build
pnpm test
pnpm typecheck
```

Package identity is the unpublished development name `@eggstack/stegoeggo`.
Registry ownership/availability is a release decision, not an M005 concern.

## Usage

```js
const stegoeggo = require('@eggstack/stegoeggo')

const notice = new stegoeggo.RightsNotice().withCopyrightHolder('Example Corp')
const request = stegoeggo.ProtectionRequest.metadataOnly(
  notice,
  stegoeggo.RightsPolicy.ProhibitedAiMlTraining
)
  .withSeed(42n)
  .withTimestampOverride('2026-01-01T00:00:00Z')

const protected_ = await stegoeggo.protect(data, request)
const report = await stegoeggo.verify(protected_)
console.log(report.evidenceStrength)
```

ESM works the same way via `import stegoeggo from '@eggstack/stegoeggo'`.

## Seeds are bigint

JavaScript `number` cannot represent every Rust `u64`, so every seed path
uses `bigint`: `withSeed(42n)`, and report seeds read back as `bigint | null`.
Zero is a valid seed. Values above `Number.MAX_SAFE_INTEGER` round-trip
exactly, including `u64::MAX` (`18446744073709551615n`). Bounded
counts/sizes/dimensions accept `number` but reject NaN, infinity, fractions,
negatives, and unsafe integers before Rust narrowing.

Reproducible output needs both an explicit seed and
`withTimestampOverride(...)`; without the override the resolver uses
wall-clock time.

## Structured errors

Every public Promise rejects with a normal `Error` whose `code` is stable:

- `ERR_STEGOEGGO_INVALID_CONFIG`
- `ERR_STEGOEGGO_INVALID_FORMAT`
- `ERR_STEGOEGGO_ENCODE_DECODE` (includes truncated input)
- `ERR_STEGOEGGO_METADATA`
- `ERR_STEGOEGGO_STEGANOGRAPHY`
- `ERR_STEGOEGGO_INSUFFICIENT_CAPACITY` (with `required`/`available`)
- `ERR_STEGOEGGO_VERIFICATION`
- `ERR_STEGOEGGO_RESOURCE_LIMIT` (with `resource` plus the
  `size`/`limit`, `width`/`height`/`maxWidth`/`maxHeight`,
  `kind`/`count`/`limit` fields that triggered it)
- `ERR_STEGOEGGO_INTERNAL` (safe fallback)

The human-readable `message` is not the machine contract; never parse it to
recover data. Secret MAC/HMAC key bytes never appear in error messages,
error properties, reports, object inspection, generated declarations, or
logs.

```js
try {
  await stegoeggo.protect(payload, request)
} catch (error) {
  if (error.code === stegoeggo.ErrorCode.ResourceLimit) {
    console.log(error.resource, error.limit)
  }
}
```

## Runtime support

- Package engine floor: Node `>= 22.13.0`.
- Node-API level: 6 (exact `u64` ↔ `BigInt` conversion).
- Tested runtime lines: Node 22, 24, 26.
- Native targets: `x86_64-unknown-linux-gnu`,
  `aarch64-unknown-linux-gnu`, `x86_64-apple-darwin`,
  `aarch64-apple-darwin`, `x86_64-pc-windows-msvc`. GNU Linux artifacts are
  built with the napi cross toolchain (glibc 2.17 build floor).
- Out of scope: Bun, Deno, Electron-specific qualification, browser/WASM,
  WASI, streaming, batch/parallel, and file helpers (`protectFile`).

## Tests

```bash
pnpm test        # unit + integration (no Rust CLI needed)
pnpm typecheck   # TypeScript consumer contract
pnpm smoke       # self-contained package smoke (builds its own tiny PNG)
```

Cross-language parity against the locally built Rust CLI and the
binding-local Rust byte oracle:

```bash
cargo build -p stegoeggo-cli          # from the repository root
STEGOEGGO_CLI=../../target/debug/stegoeggo pnpm test:parity
```

See `test/` for full coverage of every entry point; `test/types/contract.ts`
is the compile-only consumer contract.

## License

MIT, same as the underlying Rust crate.
