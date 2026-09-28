import test from 'node:test'
import assert from 'node:assert/strict'
import { execFileSync } from 'node:child_process'
import { existsSync, mkdtempSync, readFileSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { binding, fixture, hiddenMarkerRequest, metadataRequest, sha256 } from './helpers.mjs'

const here = path.dirname(fileURLToPath(import.meta.url))
const packageRoot = path.resolve(here, '..')

const cliPath = process.env.STEGOEGGO_CLI ??
  path.resolve(packageRoot, '..', '..', 'target', 'debug', 'stegoeggo')
const oraclePath = path.resolve(packageRoot, 'target', 'debug', 'oracle')

if (!existsSync(cliPath)) {
  throw new Error(
    `Rust CLI oracle not found at ${cliPath}; run "cargo build -p stegoeggo-cli" from the repository root first`
  )
}
if (!existsSync(oraclePath)) {
  throw new Error(
    `Rust byte oracle not found at ${oraclePath}; run "cargo build --manifest-path bindings/node/Cargo.toml --bin oracle" first`
  )
}

const { protect, verify } = binding()
const MAC_HEX = '0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef'
const workdir = mkdtempSync(path.join(tmpdir(), 'stegoeggo-node-parity-'))

function cli (args) {
  return execFileSync(cliPath, args, { encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] })
}

function cliStatus (args) {
  try {
    cli(args)
    return 0
  } catch (error) {
    return error.status
  }
}

function scratch (name) {
  return path.join(workdir, name)
}

function oracle (inputName, outputName, mode, seed, timestamp, holder, macHex) {
  const input = scratch(`${inputName}.in`)
  writeFileSync(input, fixture(inputName))
  const output = scratch(outputName)
  const args = [input, output, mode, seed, timestamp, holder]
  if (macHex !== undefined) {
    args.push(macHex)
  }
  execFileSync(oraclePath, args, { stdio: ['ignore', 'pipe', 'pipe'] })
  return readFileSync(output)
}

test('node metadata-only output verifies with the Rust CLI', async () => {
  const out = await protect(
    fixture('canonical_independent.png'),
    metadataRequest({ holder: 'Node Parity', seed: 4242n })
  )
  const target = scratch('node-meta.png')
  writeFileSync(target, out)
  assert.equal(cliStatus(['verify', target]), 0)
})

test('node HMAC marker output verifies with the Rust CLI key', async () => {
  const key = Buffer.from(MAC_HEX, 'hex')
  const out = await protect(
    fixture('canonical_complete.png'),
    hiddenMarkerRequest({ holder: 'Node Parity', macKey: key })
  )
  const target = scratch('node-hmac.png')
  writeFileSync(target, out)
  assert.equal(cliStatus(['verify', target, '--key', MAC_HEX]), 0)
  assert.equal(cliStatus(['verify', target, '--key', '00'.repeat(32)]), 3)
})

test('Rust CLI output verifies with node', async () => {
  const target = scratch('cli-meta.png')
  cli([
    'protect',
    path.join('..', '..', 'tests', 'fixtures', 'conformance', 'canonical', 'canonical_independent.png'),
    '-o', target,
    '--rights-policy', 'prohibited-ai-ml-training',
    '--preset', 'legal-notice',
    '--copyright-notice', 'CLI Parity',
    '--seed', '4242',
  ])
  const report = await verify(readFileSync(target))
  assert.equal(report.rights.found, true)
  assert.equal(report.rights.copyrightHolder, 'CLI Parity')
})

test('Rust CLI marker output verifies with node', async () => {
  const target = scratch('cli-marker.png')
  cli([
    'protect',
    path.join('..', '..', 'tests', 'fixtures', 'conformance', 'canonical', 'canonical_independent.png'),
    '-o', target,
    '--rights-policy', 'prohibited-ai-ml-training',
    '--preset', 'legal-notice-with-stego',
    '--copyright-notice', 'CLI Marker',
    '--seed', '7',
  ])
  const report = await verify(readFileSync(target))
  assert.equal(report.hiddenMarker.status, 'VERIFIED')
  assert.equal(report.rights.copyrightHolder, 'CLI Marker')
})

test('node and Rust agree byte-for-byte on metadata-only output', async () => {
  const expected = oracle(
    'canonical_independent.png', 'oracle-meta.png',
    'metadata', '4242', '2026-03-03T03:03:03Z', 'Parity Co'
  )
  const actual = await protect(fixture('canonical_independent.png'), metadataRequest())
  assert.equal(sha256(actual), sha256(expected))
  assert.ok(actual.equals(expected))
})

test('node and Rust agree byte-for-byte on HMAC marker output', async () => {
  const expected = oracle(
    'canonical_complete.png', 'oracle-marker.png',
    'marker', '7', '2026-04-04T04:04:04Z', 'Marker Co', MAC_HEX
  )
  const actual = await protect(
    fixture('canonical_complete.png'),
    hiddenMarkerRequest({ macKey: Buffer.from(MAC_HEX, 'hex') })
  )
  assert.equal(sha256(actual), sha256(expected))
  assert.ok(actual.equals(expected))
})

test('node and Rust agree byte-for-byte above MAX_SAFE_INTEGER', async () => {
  const seed = 18446744073709551615n
  const expected = oracle(
    'canonical_independent.png', 'oracle-big.png',
    'metadata', seed.toString(), '2026-05-05T05:05:05Z', 'Big Seed'
  )
  const { RightsNotice, ProtectionRequest, RightsPolicy } = binding()
  const notice = new RightsNotice().withCopyrightHolder('Big Seed')
  const request = ProtectionRequest.metadataOnly(notice, RightsPolicy.ProhibitedAiMlTraining)
    .withSeed(seed)
    .withTimestampOverride('2026-05-05T05:05:05Z')
  const actual = await protect(fixture('canonical_independent.png'), request)
  assert.ok(seed > BigInt(Number.MAX_SAFE_INTEGER))
  assert.equal(sha256(actual), sha256(expected))
  assert.ok(actual.equals(expected))
})
