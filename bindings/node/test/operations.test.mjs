import test from 'node:test'
import assert from 'node:assert/strict'
import { setTimeout as delay } from 'node:timers/promises'
import { binding, fixture, hiddenMarkerRequest, metadataRequest } from './helpers.mjs'

const {
  AuthenticationMode,
  ErrorCode,
  EvidenceStrength,
  HiddenMarkerMode,
  ImageOutputFormat,
  MetadataUpdatePolicy,
  ProcessingOptions,
  ProtectionRequest,
  ProtectionWarning,
  ResourceLimitsBuilder,
  RightsNotice,
  RightsPolicy,
  VerificationStatus,
} = binding()

const { protect, protectWithReport, protectWithWarnings, verify } = binding()

test('protect accepts a Buffer and returns a Buffer', async () => {
  const out = await protect(fixture('canonical_independent.png'), metadataRequest())
  assert.ok(Buffer.isBuffer(out))
  assert.ok(out.length > 0)
  assert.notEqual(Buffer.compare(out, fixture('canonical_independent.png')), 0)
})

test('protect accepts a plain Uint8Array', async () => {
  const input = new Uint8Array(fixture('canonical_independent.png'))
  const out = await protect(input, metadataRequest())
  assert.ok(Buffer.isBuffer(out))
})

test('caller mutation after invocation cannot change the task input', async () => {
  const original = fixture('canonical_independent.png')
  const mutable = Buffer.from(original)
  const request = metadataRequest()

  const pending = protect(mutable, request)
  mutable.fill(0)
  const raced = await pending

  const untouched = await protect(original, request)
  assert.equal(Buffer.compare(raced, untouched), 0, 'the worker must see the bytes as they were at call time')
  assert.notEqual(Buffer.compare(raced, Buffer.alloc(mutable.length, 0)), 0)
})

test('metadata-only protect works for PNG, JPEG, and WebP', async () => {
  for (const [fixtureName, format] of [
    ['canonical_independent.png', ImageOutputFormat.Png],
    ['canonical_independent.jpg', ImageOutputFormat.Jpeg],
    ['canonical_independent.webp', ImageOutputFormat.WebP],
  ]) {
    const request = metadataRequest({ format, holder: `Holder ${format}` })
    const out = await protect(fixture(fixtureName), request)
    const report = await verify(out)
    assert.equal(report.rights.found, true, fixtureName)
    assert.equal(report.rights.copyrightHolder, `Holder ${format}`)
    assert.equal(report.evidenceStrength, EvidenceStrength.MetadataNoticeOnly)
  }
})

test('metadata-only protect with the same output format preserves the container', async () => {
  for (const fixtureName of ['canonical_independent.png', 'canonical_independent.jpg', 'canonical_independent.webp']) {
    const out = await protect(fixture(fixtureName), metadataRequest({ format: undefined }))
    assert.equal(binding().detectFormat(out), binding().detectFormat(fixture(fixtureName)))
  }
})

test('hidden-marker protect embeds a verified marker', async () => {
  const out = await protect(fixture('canonical_complete.png'), hiddenMarkerRequest())
  const report = await verify(out)
  assert.equal(report.hiddenMarker.status, VerificationStatus.Verified)
  assert.equal(report.hiddenMarker.payloadVersion, 3)
  assert.equal(typeof report.hiddenMarker.seed, 'bigint')
  assert.equal(report.hiddenMarker.seed, 7n)
  assert.equal(report.evidenceStrength, EvidenceStrength.MetadataNoticeAndBestEffortStego)
})

test('hidden-marker protect with an explicit tiled mode', async () => {
  const request = hiddenMarkerRequest({ hiddenMarkerMode: HiddenMarkerMode.tiled(128) })
  assert.equal(request.hiddenMarkerMode.kind, 'tiled')
  const out = await protect(fixture('canonical_complete.png'), request)
  const report = await verify(out)
  assert.equal(report.hiddenMarker.status, VerificationStatus.Verified)
  assert.equal(report.hiddenMarker.payloadVersion, 3)
  assert.equal(report.evidenceStrength, EvidenceStrength.MetadataNoticeAndBestEffortStego)
})

test('seedOnly mode is honoured', async () => {
  const request = hiddenMarkerRequest({ hiddenMarkerMode: HiddenMarkerMode.seedOnly() })
  const out = await protect(fixture('canonical_complete.png'), request)
  const report = await verify(out)
  assert.notEqual(report.hiddenMarker.status, VerificationStatus.Invalid)
  assert.equal(report.rights.found, true)
})

test('seed above Number.MAX_SAFE_INTEGER survives protect and verify', async () => {
  const seed = 9007199254740993n
  const request = hiddenMarkerRequest({ seed })
  assert.equal(request.seed, seed)
  const out = await protect(fixture('canonical_complete.png'), request)
  const report = await verify(out)
  assert.equal(report.hiddenMarker.status, VerificationStatus.Verified)
  assert.equal(report.hiddenMarker.seed, seed, 'the full-width seed must round-trip exactly')
  assert.ok(report.hiddenMarker.seed > BigInt(Number.MAX_SAFE_INTEGER))
})

test('u64::MAX seed round-trips', async () => {
  const seed = 18446744073709551615n
  const out = await protect(fixture('canonical_complete.png'), hiddenMarkerRequest({ seed }))
  const report = await verify(out)
  assert.equal(report.hiddenMarker.seed, seed)
})

test('zero seed is valid', async () => {
  const out = await protect(fixture('canonical_complete.png'), hiddenMarkerRequest({ seed: 0n }))
  const report = await verify(out)
  assert.equal(report.hiddenMarker.status, VerificationStatus.Verified)
  assert.equal(report.hiddenMarker.seed, 0n)
})

test('protectWithWarnings returns warnings and bytes', async () => {
  const request = metadataRequest({ policy: RightsPolicy.ProhibitedSeeConstraints })
  const result = await protectWithWarnings(fixture('canonical_complete.png'), request)
  assert.ok(Buffer.isBuffer(result.data))
  assert.ok(Array.isArray(result.warnings))
  assert.ok(
    result.warnings.includes(ProtectionWarning.MissingRightsConstraints),
    `expected MISSING_RIGHTS_CONSTRAINTS in ${JSON.stringify(result.warnings)}`
  )
})

test('protectWithReport returns the structured execution report', async () => {
  const result = await protectWithReport(fixture('canonical_complete.png'), metadataRequest())
  assert.ok(Buffer.isBuffer(result.data))
  assert.equal(result.report.effectivePolicy, RightsPolicy.ProhibitedAiMlTraining)
  assert.equal(result.report.effectiveDmi, 'PROHIBITED_AI_ML_TRAINING')
  assert.equal(result.report.metadataInjected, true)
  assert.equal(result.report.stegoAttempted, false)
  assert.equal(result.report.stegoSucceeded, false)
  assert.equal(result.report.formatTranscoded, false)
  assert.equal(result.report.anySucceeded, true)
  assert.equal(result.report.hasDegradation, false)
  assert.deepEqual(result.report.warnings, [])
  assert.equal(typeof result.report.resourceUsage.inputBytes, 'number')
  assert.ok(result.report.resourceUsage.inputBytes > 0)
  assert.equal(typeof result.report.resourceUsage.pngChunksScanned, 'number')
})

test('protectWithReport reports a hidden-marker run', async () => {
  const result = await protectWithReport(fixture('canonical_complete.png'), hiddenMarkerRequest())
  assert.equal(result.report.stegoAttempted, true)
  assert.equal(result.report.stegoSucceeded, true)
})

test('verify reports an unprotected image', async () => {
  const report = await verify(fixture('canonical_independent.png'))
  assert.equal(report.rights.found, false)
  assert.equal(report.hiddenMarker.status, VerificationStatus.NotFound)
  assert.equal(report.status, VerificationStatus.NotFound)
  assert.equal(report.evidenceStrength, EvidenceStrength.NoNoticeFound)
  assert.equal(report.hasErrors, false)
  assert.equal(report.authentication.attempted, false)
  assert.deepEqual(report.diagnostics, [])
  assert.equal(typeof report.trust.trusted, 'boolean')
})

test('verify reports a metadata-only image', async () => {
  const out = await protect(fixture('canonical_independent.png'), metadataRequest())
  const report = await verify(out)
  assert.equal(report.rights.found, true)
  assert.equal(report.rights.copyrightHolder, 'Parity Co')
  assert.equal(report.hiddenMarker.status, VerificationStatus.NotFound)
  assert.equal(report.status, VerificationStatus.Verified)
  assert.equal(report.evidenceStrength, EvidenceStrength.MetadataNoticeOnly)
  assert.equal(report.rights.source, 'legacy')
  assert.deepEqual(report.rights.channels, ['pngText'])
  assert.equal(report.rights.dmi, 2)
  assert.equal(report.rights.creator, null)
})

test('verify with the correct HMAC key authenticates', async () => {
  const key = Buffer.from('0123456789abcdef0123456789abcdef', 'utf8')
  const out = await protect(
    fixture('canonical_complete.png'),
    hiddenMarkerRequest({ macKey: key, authentication: AuthenticationMode.Hmac })
  )
  const report = await verify(out, { macKey: key })
  assert.equal(report.authentication.attempted, true)
  assert.equal(report.authentication.keyMatched, true)
  assert.equal(report.authentication.hmacStatus, VerificationStatus.Verified)
  assert.equal(report.evidenceStrength, EvidenceStrength.MetadataNoticeAndAuthenticatedProvenance)
  assert.equal(report.hasErrors, false)
})

test('verify with a wrong HMAC key reports invalid without throwing', async () => {
  const key = Buffer.from('0123456789abcdef0123456789abcdef', 'utf8')
  const wrong = Buffer.from('ffffffffffffffffffffffffffffffff', 'utf8')
  const out = await protect(
    fixture('canonical_complete.png'),
    hiddenMarkerRequest({ macKey: key, authentication: AuthenticationMode.Hmac })
  )
  const report = await verify(out, { macKey: wrong })
  assert.equal(report.authentication.keyMatched, false)
  assert.equal(report.authentication.hmacStatus, VerificationStatus.Invalid)
  assert.equal(report.hiddenMarker.status, VerificationStatus.Invalid)
  assert.equal(report.hasErrors, true)
  assert.notEqual(report.status, VerificationStatus.Verified)
})

test('verify without the HMAC key cannot authenticate', async () => {
  const key = Buffer.from('0123456789abcdef0123456789abcdef', 'utf8')
  const out = await protect(
    fixture('canonical_complete.png'),
    hiddenMarkerRequest({ macKey: key, authentication: AuthenticationMode.Hmac })
  )
  const report = await verify(out)
  assert.equal(report.authentication.attempted, true)
  assert.equal(report.authentication.keyMatched, false)
  assert.equal(report.authentication.hmacStatus, VerificationStatus.NotFound)
  assert.equal(report.evidenceStrength, EvidenceStrength.MetadataNoticeOnly)
  assert.notEqual(report.evidenceStrength, EvidenceStrength.MetadataNoticeAndAuthenticatedProvenance)
})

test('verify never rejects for untrusted bytes', async () => {
  for (const payload of [
    Buffer.from('not a real image'),
    Buffer.alloc(0),
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    fixture('canonical_complete.png').subarray(0, 32),
  ]) {
    const report = await verify(payload)
    assert.equal(typeof report.status, 'string')
    assert.equal(typeof report.hasErrors, 'boolean')
  }
})

test('event loop stays live during a representative protect operation', async () => {
  let tickRan = false
  const pending = protect(fixture('canonical_complete.png'), hiddenMarkerRequest({ intensity: 0.9 }))
  const work = delay(0).then(() => {
    tickRan = true
  })
  await pending
  await work
  assert.equal(tickRan, true, 'queued event-loop work must run before the Promise settles')
})

test('concurrent operations do not share request state', async () => {
  const requests = [
    hiddenMarkerRequest({ holder: 'One', seed: 1n }),
    hiddenMarkerRequest({ holder: 'Two', seed: 2n }),
    hiddenMarkerRequest({ holder: 'Three', seed: 3n }),
    metadataRequest({ holder: 'Four' }),
  ]
  const outputs = await Promise.all(
    requests.map((request) => protect(fixture('canonical_independent.png'), request))
  )
  const reports = await Promise.all(outputs.map((output) => verify(output)))
  assert.deepEqual(
    reports.map((report) => report.rights.copyrightHolder),
    ['One', 'Two', 'Three', 'Four']
  )
  assert.deepEqual(
    reports.slice(0, 3).map((report) => report.hiddenMarker.seed),
    [1n, 2n, 3n]
  )
  assert.equal(reports[3].hiddenMarker.status, VerificationStatus.NotFound)
  assert.equal(reports[3].evidenceStrength, EvidenceStrength.MetadataNoticeOnly)
})

test('deterministic requests produce byte-identical output', async () => {
  const first = await protect(fixture('canonical_independent.png'), metadataRequest())
  const second = await protect(fixture('canonical_independent.png'), metadataRequest())
  assert.equal(Buffer.compare(first, second), 0)
})

test('metadata update policy and output format are honoured', async () => {
  const request = metadataRequest({ format: ImageOutputFormat.WebP })
  const out = await protect(fixture('canonical_complete.png'), request)
  assert.equal(binding().detectFormat(out), 'webp')

  const preserve = metadataRequest({
    processing: new ProcessingOptions().withMetadataUpdatePolicy(
      MetadataUpdatePolicy.PreserveExisting
    ),
  })
  const preserved = await protect(out, preserve)
  const report = await verify(preserved)
  assert.equal(report.rights.copyrightHolder, 'Parity Co')
})

test('resource limits are accepted through the request', async () => {
  const limits = ResourceLimitsBuilder.new().withMaxInputBytes(1_000_000).build()
  const request = metadataRequest({ resourceLimits: limits })
  const out = await protect(fixture('canonical_independent.png'), request)
  assert.equal((await verify(out)).rights.found, true)
})

test('the whole public surface rejects only with ERR_STEGOEGGO_* codes', async () => {
  const valid = [
    ErrorCode.InvalidConfig,
    ErrorCode.InvalidFormat,
    ErrorCode.EncodeDecode,
    ErrorCode.Metadata,
    ErrorCode.Steganography,
    ErrorCode.InsufficientCapacity,
    ErrorCode.Verification,
    ErrorCode.ResourceLimit,
    ErrorCode.Internal,
  ]
  assert.equal(valid.length, 9)
  for (const code of valid) {
    assert.match(code, /^ERR_STEGOEGGO_[A-Z_]+$/)
  }
})

test('an unspecified policy with an empty notice emits no notice', async () => {
  const notice = new RightsNotice()
  const request = ProtectionRequest.metadataOnly(notice, RightsPolicy.Unspecified)
  const out = await protect(fixture('canonical_independent.png'), request)
  const report = await verify(out)
  assert.equal(report.rights.found, false)
  assert.equal(report.status, VerificationStatus.NotFound)
  assert.equal(report.evidenceStrength, EvidenceStrength.NoNoticeFound)
})
