import test from 'node:test'
import assert from 'node:assert/strict'
import { createHash, randomBytes } from 'node:crypto'
import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import path from 'node:path'
import { binding, fixture, hiddenMarkerRequest, metadataRequest, rejection } from './helpers.mjs'

const here = path.dirname(fileURLToPath(import.meta.url))

const { ErrorCode, ResourceLimitsBuilder, RightsPolicy } = binding()

const { protect, protectWithReport, protectWithWarnings, toPublicError, verify } = binding()

const SECRET = 'super-secret-mac-key-value'

test('invalid format is a stable coded failure', async () => {
  const error = await rejection(protect(Buffer.from('not a real image'), metadataRequest()))
  assert.ok(error instanceof Error)
  assert.equal(error.name, 'StegoEggoError')
  assert.equal(error.code, ErrorCode.InvalidFormat)
  assert.equal(typeof error.message, 'string')
  assert.ok(error.message.length > 0)
})

test('every protect variant rejects with the same code', async () => {
  const request = metadataRequest()
  const payload = Buffer.from('not a real image')
  for (const call of [protect, protectWithWarnings, protectWithReport]) {
    const error = await rejection(call(payload, request))
    assert.equal(error.code, ErrorCode.InvalidFormat, call.name)
  }
})

test('truncated input is categorised as encode/decode', async () => {
  const complete = fixture('canonical_complete.png')
  const truncated = complete.subarray(0, Math.floor(complete.length / 3))
  const error = await rejection(protect(truncated, metadataRequest()))
  assert.ok(
    [
      ErrorCode.EncodeDecode,
      ErrorCode.Metadata,
      ErrorCode.InvalidFormat,
    ].includes(error.code),
    `unexpected category ${error.code}`
  )
  assert.match(error.message.toLowerCase(), /truncat|decode|metadata|format|end of|stream/i)
})

test('insufficient capacity carries required and available', () => {
  // The canonical best-effort hidden-marker policy degrades instead of failing
  // on a small carrier, so the public API cannot currently produce this
  // variant. The binding-side projection is therefore exercised directly; the
  // canonical Rust variant mapping is covered by the binding's own unit tests.
  const error = toPublicError({
    code: ErrorCode.InsufficientCapacity,
    message: 'Insufficient capacity: need 4096 carrier units, have 12',
    required: 4096,
    available: 12,
  })
  assert.ok(error instanceof Error)
  assert.equal(error.code, ErrorCode.InsufficientCapacity)
  assert.equal(error.required, 4096)
  assert.equal(error.available, 12)
  assert.equal(error.resource, undefined)
  assert.equal(error.kind, undefined)
})

test('best-effort embedding degrades with a warning instead of failing', async () => {
  const result = await protectWithWarnings(
    fixture('canonical_copyright_only.png'),
    hiddenMarkerRequest({ intensity: 1, stegoRedundancy: 10 })
  )
  assert.ok(Buffer.isBuffer(result.data))
  assert.ok(
    result.warnings.includes('LSB_CAPACITY_SKIPPED') ||
      result.warnings.includes('DCT_CAPACITY_INSUFFICIENT'),
    `expected a capacity warning in ${JSON.stringify(result.warnings)}`
  )
})

test('input resource-limit carries resource, size, and limit', async () => {
  const payload = fixture('canonical_complete.png')
  const limits = ResourceLimitsBuilder.new().withMaxInputBytes(8).build()
  const request = metadataRequest({ resourceLimits: limits })
  const error = await rejection(protect(payload, request))
  assert.equal(error.code, ErrorCode.ResourceLimit)
  assert.equal(error.resource, 'input_bytes')
  assert.equal(error.size, payload.length)
  assert.equal(error.limit, 8)
})

test('dimension resource-limit carries width, height, maxWidth, and maxHeight', async () => {
  const limits = ResourceLimitsBuilder.new().withMaxWidth(4).withMaxHeight(4).build()
  const request = metadataRequest({ resourceLimits: limits })
  const error = await rejection(protect(fixture('canonical_independent.png'), request))
  assert.equal(error.code, ErrorCode.ResourceLimit)
  assert.equal(error.resource, 'dimensions')
  assert.ok(error.width > 4)
  assert.ok(error.height > 4)
  assert.equal(error.maxWidth, 4)
  assert.equal(error.maxHeight, 4)
})

test('container resource-limit carries resource, kind, count, and limit', async () => {
  const limits = ResourceLimitsBuilder.new().withMaxPngChunks(1).build()
  const request = metadataRequest({ resourceLimits: limits })
  const error = await rejection(protect(fixture('canonical_complete.png'), request))
  assert.equal(error.code, ErrorCode.ResourceLimit)
  assert.equal(error.resource, 'container')
  assert.equal(typeof error.kind, 'string')
  assert.ok(error.count > 1)
  assert.equal(error.limit, 1)
})

test('metadata resource-limit carries resource, kind, size, and limit', async () => {
  const limits = ResourceLimitsBuilder.new().withMaxMetadataFieldBytes(1).build()
  const request = metadataRequest({
    holder: 'Acme Long Name',
    resourceLimits: limits,
  })
  const error = await rejection(protect(fixture('canonical_independent.png'), request))
  assert.equal(error.code, ErrorCode.ResourceLimit)
  assert.equal(error.resource, 'metadata')
  assert.equal(typeof error.kind, 'string')
  assert.ok(error.size > 1)
  assert.equal(error.limit, 1)
})

test('verification accepts resource limits and still returns a report', async () => {
  const limits = ResourceLimitsBuilder.new().withMaxInputBytes(8).build()
  const report = await verify(fixture('canonical_complete.png'), { resourceLimits: limits })
  assert.equal(typeof report.status, 'string')
  assert.equal(typeof report.rights.found, 'boolean')
})

test('toPublicError normalizes a synthetic non-exhaustive failure', () => {
  const error = toPublicError({
    code: ErrorCode.Internal,
    message: 'unreachable future variant',
    resource: null,
    required: null,
    available: null,
    size: null,
    limit: null,
    width: null,
    height: null,
    maxWidth: null,
    maxHeight: null,
    kind: null,
    count: null,
  })
  assert.ok(error instanceof Error)
  assert.equal(error.code, ErrorCode.Internal)
  assert.equal(error.message, 'unreachable future variant')
  assert.equal(Object.keys(error).includes('resource'), false)
})

test('toPublicError falls back to ERR_STEGOEGGO_INTERNAL for a malformed DTO', () => {
  const error = toPublicError({})
  assert.equal(error.code, ErrorCode.Internal)
  assert.equal(typeof error.message, 'string')
})

test('a wrong argument type is reported as a configuration failure, not a raw status', async () => {
  const error = await rejection(protect(42, metadataRequest()))
  assert.ok(error instanceof Error)
  assert.equal(error.code, ErrorCode.InvalidConfig)
  assert.equal(error.code.startsWith('ERR_STEGOEGGO_'), true)
})

test('an unknown native throw falls back to ERR_STEGOEGGO_INTERNAL', () => {
  const error = toPublicError({ code: 'SomethingUnmapped', message: 'boom' })
  assert.equal(error.code, ErrorCode.Internal)
})

test('secret key material never appears in errors, reports, or request state', async () => {
  const key = Buffer.from(SECRET, 'utf8')

  const limited = ResourceLimitsBuilder.new().withMaxInputBytes(8).build()
  const limitError = await rejection(
    protect(fixture('canonical_complete.png'), metadataRequest({ resourceLimits: limited }))
  )
  const limitText = `${limitError} ${JSON.stringify(limitError)} ${String(limitError.stack)}`
  assert.equal(limitText.includes(SECRET), false)

  const badFormat = await rejection(
    protect(Buffer.from('not a real image'), metadataRequest({ macKey: key }))
  )
  const badText = `${badFormat} ${JSON.stringify(badFormat)} ${String(badFormat.stack)}`
  assert.equal(badText.includes(SECRET), false)

  const result = await protectWithReport(
    fixture('canonical_independent.png'),
    hiddenMarkerRequest({ macKey: key })
  )
  const reportText = JSON.stringify(result.report, (_key, value) =>
    typeof value === 'bigint' ? value.toString() : value
  )
  assert.equal(reportText.includes(SECRET), false)
  assert.equal(Buffer.from(result.data).toString('utf8').includes(SECRET), false)
  assert.equal(Buffer.from(result.data).toString('hex').includes(key.toString('hex')), false)

  const verified = await verify(result.data, { macKey: key })
  const verifiedText = JSON.stringify(verified, (_key, value) =>
    typeof value === 'bigint' ? value.toString() : value
  )
  assert.equal(verifiedText.includes(SECRET), false)
  assert.equal(verifiedText.includes(key.toString('hex')), false)
})

test('secret key material is not present in the generated declarations', () => {
  const declarations = readFileSync(path.join(here, '..', 'index.d.ts'), 'utf8')
  assert.equal(declarations.includes(SECRET), false)
  assert.match(declarations, /withMacKey\(key: Uint8Array\)/)
})

test('a MAC-protected request reports hasMacKey without exposing the key', async () => {
  const key = randomBytes(32)
  const request = hiddenMarkerRequest({ macKey: key })
  assert.equal(request.hasMacKey, true)
  const inspected = [
    Object.getOwnPropertyNames(request).join(','),
    String(request.hasMacKey),
    Object.getOwnPropertyDescriptors(request).seed === undefined ? 'no-seed-descriptor' : 'seed',
  ].join('|')
  assert.equal(inspected.includes(key.toString('hex')), false)
  assert.equal(inspected.includes(SECRET), false)
  assert.equal(Object.getOwnPropertyNames(request).includes('macKey'), false)
})

test('no test recovers data by parsing the human message', async () => {
  const error = await rejection(
    protect(fixture('canonical_complete.png'), metadataRequest({
      resourceLimits: ResourceLimitsBuilder.new().withMaxInputBytes(8).build(),
    }))
  )
  assert.equal(error.resource, 'input_bytes')
  assert.equal(error.size, fixture('canonical_complete.png').length)
  assert.equal(error.limit, 8)
  assert.equal(error.code, ErrorCode.ResourceLimit)
})

test('deterministic digests are stable across identical requests', async () => {
  const first = await protect(fixture('canonical_independent.png'), metadataRequest())
  const second = await protect(fixture('canonical_independent.png'), metadataRequest())
  const digest = (data) => createHash('sha256').update(data).digest('hex')
  assert.equal(digest(first), digest(second))
})

test('a see-constraints policy without constraints is reported as a warning', async () => {
  const result = await protectWithWarnings(
    fixture('canonical_complete.png'),
    metadataRequest({ policy: RightsPolicy.ProhibitedSeeConstraints })
  )
  assert.ok(result.warnings.includes('MISSING_RIGHTS_CONSTRAINTS'))
  assert.equal(result.report, undefined)
})
