import assert from 'node:assert/strict'
import { createRequire } from 'node:module'
import { deflateSync } from 'node:zlib'

const require = createRequire(import.meta.url)
const spec = process.env.STEGOEGGO_SMOKE_SPEC ?? '../stegoeggo.js'
const api = require(spec)
const pkgRequire = createRequire(require.resolve(spec))

function crc32 (buffer) {
  let table = crc32.cache
  if (!table) {
    table = new Int32Array(256)
    for (let n = 0; n < 256; n += 1) {
      let c = n
      for (let k = 0; k < 8; k += 1) {
        c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1
      }
      table[n] = c
    }
    crc32.cache = table
  }
  let crc = 0xffffffff
  for (const byte of buffer) {
    crc = table[(crc ^ byte) & 0xff] ^ (crc >>> 8)
  }
  return (crc ^ 0xffffffff) >>> 0
}

function pngChunk (type, data) {
  const body = Buffer.concat([Buffer.from(type, 'ascii'), data])
  const out = Buffer.alloc(12 + data.length)
  out.writeUInt32BE(data.length, 0)
  body.copy(out, 4)
  out.writeUInt32BE(crc32(body), 8 + data.length)
  return out
}

function makePng (width = 16, height = 16) {
  const signature = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a])
  const ihdr = Buffer.alloc(13)
  ihdr.writeUInt32BE(width, 0)
  ihdr.writeUInt32BE(height, 4)
  ihdr[8] = 8
  ihdr[9] = 2
  const rows = []
  for (let y = 0; y < height; y += 1) {
    rows.push(Buffer.concat([Buffer.from([0]), Buffer.alloc(width * 3, 0x80)]))
  }
  const idat = deflateSync(Buffer.concat(rows))
  return Buffer.concat([
    signature,
    pngChunk('IHDR', ihdr),
    pngChunk('IDAT', idat),
    pngChunk('IEND', Buffer.alloc(0)),
  ])
}

const version = api.stegoeggoVersion()
assert.match(version, /^\d+\.\d+\.\d+$/)
assert.equal(version, pkgRequire('./package.json').version)
assert.equal(pkgRequire('./index.js').__napiBindingTarget, 'native')

const input = makePng()
assert.equal(api.detectFormat(input), 'png')

const notice = new api.RightsNotice().withCopyrightHolder('package smoke')
const request = api.ProtectionRequest.metadataOnly(notice, api.RightsPolicy.ProhibitedAiMlTraining)
  .withSeed(42n)
  .withTimestampOverride('2026-01-01T00:00:00Z')

const pending = api.protect(input, request)
assert.ok(pending instanceof Promise)
const output = await pending
assert.ok(Buffer.isBuffer(output))
assert.ok(output.length > 0)

const report = await api.verify(output)
assert.equal(report.rights.found, true)
assert.equal(report.rights.copyrightHolder, 'package smoke')

console.log(`stegoeggo package smoke ok (version ${version})`)
