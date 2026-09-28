import test from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { binding, fixture } from './helpers.mjs'

const here = path.dirname(fileURLToPath(import.meta.url))
const rootManifest = readFileSync(path.resolve(here, '..', '..', '..', 'Cargo.toml'), 'utf8')

test('module loads from the generated loader', () => {
  const api = binding()
  assert.equal(typeof api.protect, 'function')
  assert.equal(typeof api.protectWithWarnings, 'function')
  assert.equal(typeof api.protectWithReport, 'function')
  assert.equal(typeof api.verify, 'function')
  assert.equal(typeof api.detectFormat, 'function')
  assert.equal(api.__napiBindingTarget, 'native')
})

test('module version equals the wrapped StegoEggo version', () => {
  const api = binding()
  const rootVersion = /^\s*version\s*=\s*"([^"]+)"/m.exec(rootManifest)
  assert.ok(rootVersion, 'root Cargo.toml declares a version')
  assert.equal(api.stegoeggoVersion(), rootVersion[1])
  assert.equal(
    api.stegoeggoVersion(),
    JSON.parse(readFileSync(path.resolve(here, '..', 'package.json'), 'utf8')).version
  )
})

test('detectFormat recognises PNG, JPEG, and WebP', () => {
  const { detectFormat } = binding()
  assert.equal(detectFormat(fixture('canonical_independent.png')), 'png')
  assert.equal(detectFormat(fixture('canonical_independent.jpg')), 'jpg')
  assert.equal(detectFormat(fixture('canonical_independent.webp')), 'webp')
  assert.equal(detectFormat(Buffer.from(fixture('canonical_independent.png'))), 'png')
  assert.equal(detectFormat(new Uint8Array(fixture('canonical_independent.jpg'))), 'jpg')
})

test('detectFormat returns null for unknown input', () => {
  const { detectFormat } = binding()
  assert.equal(detectFormat(Buffer.from('not a real image')), null)
  assert.equal(detectFormat(new Uint8Array([0x89, 0x50])), null)
  assert.equal(detectFormat(new Uint8Array(0)), null)
})

test('the addon is a loadable native binary, not a WASI flavor', () => {
  const { __napiBindingTarget } = binding()
  assert.equal(__napiBindingTarget, 'native')
})
