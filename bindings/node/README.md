# stegoeggo Node.js binding

Node.js interface to the `stegoeggo` Rust library for protecting and verifying
rights metadata on PNG, JPEG, and WebP encoded bytes.

**Status:** experimental and not published to npm. Build it from a source
checkout. The supported runtime lines are Node 22, 24, and 26, with a minimum
engine version of 22.13.0.

## Install from source

Prerequisites: Rust 1.89 or later, Node 22.13.0 or later, and pnpm 12.6.0 via
Corepack. From the repository root:

```bash
cd bindings/node
corepack enable
pnpm install --frozen-lockfile
pnpm build
```

## Usage

Protection and verification are asynchronous. Use encoded bytes when rights
metadata must remain in the output file. Pixel-only decode/encode round-trips
can discard container metadata.

```js
const fs = require('node:fs/promises')
const stegoeggo = require('@eggstack/stegoeggo')

async function main() {
  const data = await fs.readFile('input.png')
  const notice = new stegoeggo.RightsNotice().withCopyrightHolder('Example Corp')
  const request = stegoeggo.ProtectionRequest.metadataOnly(
    notice,
    stegoeggo.RightsPolicy.ProhibitedAiMlTraining
  )
    .withSeed(42n)
    .withTimestampOverride('2026-01-01T00:00:00Z')

  const protectedBytes = await stegoeggo.protect(data, request)
  const report = await stegoeggo.verify(protectedBytes)
  console.log(report.evidenceStrength)
}

main().catch((error) => {
  console.error(error)
  process.exitCode = 1
})
```

ES modules can use `import stegoeggo from '@eggstack/stegoeggo'`.
Seeds use JavaScript `bigint` so every Rust `u64` value is represented exactly.
The library call runs on a worker thread and does not block the JavaScript event
loop.

## Errors and keys

Rejected promises contain an `Error` with a stable `code` and structured fields
where applicable. Use `error.code` and documented properties rather than
parsing `message`. Secret MAC key bytes are not included in errors or reports.

## License

MIT, same as the Rust library.
