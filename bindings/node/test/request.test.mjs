import test from 'node:test'
import assert from 'node:assert/strict'
import { binding } from './helpers.mjs'

const {
  AuthenticationMode,
  DmiValue,
  EvidenceChannel,
  EvidenceStrength,
  FieldSource,
  HiddenMarkerKind,
  HiddenMarkerMode,
  ImageOutputFormat,
  MetadataUpdatePolicy,
  ProcessingOptions,
  ProtectionPreset,
  ProtectionRequest,
  ProtectionWarning,
  ResourceLimits,
  ResourceLimitsBuilder,
  RightsNotice,
  RightsPolicy,
  VerificationStatus,
  DiagnosticLevel,
  ErrorCode,
} = binding()

/**
 * Reads every member of a napi-rs string-enum object.
 *
 * The members are non-enumerable accessors, so `Object.values` sees nothing.
 */
function values (enumObject) {
  return Object.getOwnPropertyNames(enumObject).map((key) => enumObject[key])
}

test('enum value sets are stable at runtime', () => {
  assert.deepEqual(values(RightsPolicy), [
    'UNSPECIFIED',
    'ALLOWED',
    'PROHIBITED_AI_ML_TRAINING',
    'PROHIBITED_GENERATIVE_AI_TRAINING',
    'PROHIBITED_EXCEPT_SEARCH_INDEXING',
    'PROHIBITED_ALL_DATA_MINING',
    'PROHIBITED_SEE_CONSTRAINTS',
  ])
  assert.deepEqual(values(DmiValue), [
    'UNSPECIFIED',
    'ALLOWED',
    'PROHIBITED_AI_ML_TRAINING',
    'PROHIBITED_GEN_AI_ML_TRAINING',
    'PROHIBITED_EXCEPT_SEARCH_ENGINE_INDEXING',
    'PROHIBITED',
    'PROHIBITED_SEE_CONSTRAINTS',
  ])
  assert.deepEqual(values(ImageOutputFormat), ['PNG', 'JPEG', 'WEBP'])
  assert.deepEqual(values(MetadataUpdatePolicy), [
    'REPLACE_STEGO_OWNED',
    'FAIL_ON_CONFLICT',
    'PRESERVE_EXISTING',
  ])
  assert.deepEqual(values(ProtectionPreset), [
    'LEGAL_NOTICE',
    'LEGAL_NOTICE_WITH_STEGO',
    'AUTHENTICATED_PROVENANCE',
    'MAXIMAL',
  ])
  assert.deepEqual(values(AuthenticationMode), ['NONE', 'HMAC'])
  assert.deepEqual(values(HiddenMarkerKind), ['disabled', 'seedOnly', 'bestEffort', 'tiled'])
  assert.deepEqual(values(ProtectionWarning), [
    'MISSING_MAC_KEY',
    'METADATA_INJECTION_DISABLED',
    'PROGRESSIVE_JPEG_FALLBACK',
    'JPEG_REENCODE_FRAGILE',
    'LSB_CAPACITY_SKIPPED',
    'DCT_CAPACITY_INSUFFICIENT',
    'CONTRADICTORY_LEGAL_CLAIMS',
    'MISSING_RIGHTS_CONSTRAINTS',
  ])
  assert.deepEqual(values(VerificationStatus), ['VERIFIED', 'INVALID', 'NOT_FOUND'])
  assert.deepEqual(values(EvidenceStrength), [
    'NO_NOTICE_FOUND',
    'METADATA_NOTICE_ONLY',
    'METADATA_NOTICE_AND_BEST_EFFORT_STEGO',
    'METADATA_NOTICE_AND_AUTHENTICATED_PROVENANCE',
  ])
  assert.deepEqual(values(FieldSource), [
    'xmp',
    'legacy',
    'embeddedPayloadV1',
    'embeddedPayloadV2',
    'embeddedPayloadV3',
    'detachedManifest',
    'qTableSeed',
  ])
  assert.deepEqual(values(EvidenceChannel), [
    'pngText',
    'pngXmp',
    'jpegComment',
    'jpegXmp',
    'jpegIptc',
    'webpXmp',
    'webpExif',
    'lsbPayload',
    'dctPayload',
    'qTableSeed',
  ])
  assert.deepEqual(values(DiagnosticLevel), ['info', 'warning', 'error'])
  assert.deepEqual(values(ErrorCode), [
    'ERR_STEGOEGGO_INVALID_CONFIG',
    'ERR_STEGOEGGO_INVALID_FORMAT',
    'ERR_STEGOEGGO_ENCODE_DECODE',
    'ERR_STEGOEGGO_METADATA',
    'ERR_STEGOEGGO_STEGANOGRAPHY',
    'ERR_STEGOEGGO_INSUFFICIENT_CAPACITY',
    'ERR_STEGOEGGO_VERIFICATION',
    'ERR_STEGOEGGO_RESOURCE_LIMIT',
    'ERR_STEGOEGGO_INTERNAL',
  ])
})

test('RightsNotice builders and getters round-trip', () => {
  const base = new RightsNotice()
  assert.equal(base.copyrightHolder, null)
  assert.equal(base.dmi, null)
  assert.equal(base.seed, null)
  assert.equal(base.hasLegalContent, false)

  const notice = base
    .withCopyrightHolder('Acme Corp')
    .withCopyrightOwner('Acme Holdings')
    .withContactEmail('legal@acme.example')
    .withLicenseUrl('https://acme.example/license')
    .withUsageTerms('All rights reserved')
    .withCreationDate('2026-01-01')
    .withAiConstraints('No AI training')
    .withWebStatementOfRights('https://acme.example/rights')
    .withCreator('A. Creator')
    .withCreditLine('Credit Line')
    .withLicensorName('Licensor')
    .withLicensorEmail('licensor@acme.example')
    .withLicensorUrl('https://acme.example/licensor')
    .withMetadataDate('2026-01-02')
    .withNoticeAppliedAt('2026-01-03T00:00:00Z')
    .withDmi(DmiValue.Prohibited)

  assert.equal(notice.copyrightHolder, 'Acme Corp')
  assert.equal(notice.copyrightOwner, 'Acme Holdings')
  assert.equal(notice.contactEmail, 'legal@acme.example')
  assert.equal(notice.licenseUrl, 'https://acme.example/license')
  assert.equal(notice.usageTerms, 'All rights reserved')
  assert.equal(notice.creationDate, '2026-01-01')
  assert.equal(notice.aiConstraints, 'No AI training')
  assert.equal(notice.webStatementOfRights, 'https://acme.example/rights')
  assert.equal(notice.creator, 'A. Creator')
  assert.equal(notice.creditLine, 'Credit Line')
  assert.equal(notice.licensorName, 'Licensor')
  assert.equal(notice.licensorEmail, 'licensor@acme.example')
  assert.equal(notice.licensorUrl, 'https://acme.example/licensor')
  assert.equal(notice.metadataDate, '2026-01-02')
  assert.equal(notice.noticeAppliedAt, '2026-01-03T00:00:00Z')
  assert.equal(notice.dmi, DmiValue.Prohibited)
  assert.equal(base.copyrightHolder, null, 'builders are immutable')
})

test('ProtectionRequest metadata-only and hidden-marker factories', () => {
  const notice = new RightsNotice().withCopyrightHolder('Acme')

  const metadata = ProtectionRequest.metadataOnly(notice, RightsPolicy.ProhibitedAiMlTraining)
  assert.ok(metadata instanceof ProtectionRequest)
  assert.equal(metadata.policy, RightsPolicy.ProhibitedAiMlTraining)
  assert.equal(metadata.rightsMetadataEnabled, true)
  assert.equal(metadata.hiddenMarkerMode.kind, HiddenMarkerKind.Disabled)
  assert.equal(metadata.authenticationMode, AuthenticationMode.None)
  assert.equal(metadata.seed, null)
  assert.equal(metadata.intensity, 0.5)
  assert.equal(metadata.hasMacKey, false)

  const marker = ProtectionRequest.withHiddenMarker(notice, RightsPolicy.ProhibitedAiMlTraining)
  assert.equal(marker.hiddenMarkerMode.kind, HiddenMarkerKind.BestEffort)
  assert.equal(marker.hiddenMarkerMode.tileSize, null)
})

test('ProtectionRequest preset construction', () => {
  const notice = new RightsNotice().withCopyrightHolder('Acme')
  const legal = ProtectionRequest.fromPreset(ProtectionPreset.LegalNotice, notice, RightsPolicy.Allowed)
  assert.equal(legal.hiddenMarkerMode.kind, HiddenMarkerKind.Disabled)
  assert.equal(legal.authenticationMode, AuthenticationMode.None)

  const authenticated = ProtectionRequest.fromPreset(
    ProtectionPreset.AuthenticatedProvenance,
    notice,
    RightsPolicy.ProhibitedSeeConstraints
  )
  assert.equal(authenticated.hiddenMarkerMode.kind, HiddenMarkerKind.BestEffort)
  assert.equal(authenticated.authenticationMode, AuthenticationMode.Hmac)
})

test('ProtectionRequest hidden-marker mode replacement keeps other channels', () => {
  const notice = new RightsNotice().withCopyrightHolder('Acme')
  const base = ProtectionRequest.withHiddenMarker(notice, RightsPolicy.ProhibitedAiMlTraining)
    .withSeed(11n)
    .withTimestampOverride('2026-01-01T00:00:00Z')
  const tiled = base.withHiddenMarkerMode(HiddenMarkerMode.tiled(128))
  assert.equal(tiled.hiddenMarkerMode.kind, HiddenMarkerKind.Tiled)
  assert.equal(tiled.hiddenMarkerMode.tileSize, 128)
  assert.equal(tiled.seed, 11n)
  assert.equal(tiled.timestampOverride, '2026-01-01T00:00:00Z')
  assert.equal(base.hiddenMarkerMode.kind, HiddenMarkerKind.BestEffort)

  const hmac = base.withAuthentication(AuthenticationMode.Hmac)
  assert.equal(hmac.authenticationMode, AuthenticationMode.Hmac)
  assert.equal(hmac.hiddenMarkerMode.kind, HiddenMarkerKind.BestEffort)
  assert.equal(base.authenticationMode, AuthenticationMode.None)
})

test('ProcessingOptions projection', () => {
  const defaults = new ProcessingOptions()
  assert.equal(defaults.outputFormat, null)
  assert.equal(defaults.jpegQuality, 90)
  assert.equal(defaults.progressiveJpeg, false)
  assert.equal(defaults.maxDimension, null)
  assert.equal(defaults.metadataUpdatePolicy, MetadataUpdatePolicy.ReplaceStegoOwned)
  assert.equal(defaults.stegoRedundancy, null)
  assert.equal(defaults.contentHash, null)
  assert.equal(defaults.timestampOverride, null)

  const configured = defaults
    .withOutputFormat(ImageOutputFormat.Jpeg)
    .withJpegQuality(80)
    .withProgressiveJpeg()
    .withMaxDimension(1024)
    .withMetadataUpdatePolicy(MetadataUpdatePolicy.PreserveExisting)
    .withStegoRedundancy(3)
    .withContentHash(new Uint8Array([1, 2, 3, 4]))
    .withTimestampOverride('2026-01-01T00:00:00Z')

  assert.equal(configured.outputFormat, ImageOutputFormat.Jpeg)
  assert.equal(configured.jpegQuality, 80)
  assert.equal(configured.progressiveJpeg, true)
  assert.equal(configured.maxDimension, 1024)
  assert.equal(configured.metadataUpdatePolicy, MetadataUpdatePolicy.PreserveExisting)
  assert.equal(configured.stegoRedundancy, 3)
  assert.deepEqual(Array.from(configured.contentHash), [1, 2, 3, 4])
  assert.equal(configured.timestampOverride, '2026-01-01T00:00:00Z')
  assert.equal(defaults.outputFormat, null, 'builders are immutable')
})

test('ResourceLimits defaults and builder', () => {
  const defaults = ResourceLimits.defaults()
  assert.ok(defaults instanceof ResourceLimits)
  assert.ok(defaults.maxInputBytes > 0)
  assert.ok(defaults.maxWidth > 0)
  assert.ok(defaults.maxHeight > 0)

  const limits = ResourceLimitsBuilder.new()
    .withMaxInputBytes(8)
    .withMaxWidth(4)
    .withMaxHeight(4)
    .withMaxPngChunks(1)
    .build()
  assert.equal(limits.maxInputBytes, 8)
  assert.equal(limits.maxWidth, 4)
  assert.equal(limits.maxHeight, 4)
  assert.equal(limits.maxPngChunks, 1)
  assert.equal(limits.maxJpegSegments, defaults.maxJpegSegments, 'unset limits keep defaults')
})

test('bigint seed at 0, ordinary, u64::MAX, and above MAX_SAFE_INTEGER', () => {
  const notice = new RightsNotice().withCopyrightHolder('Acme')
  const base = ProtectionRequest.metadataOnly(notice, RightsPolicy.Allowed)

  assert.equal(base.withSeed(0n).seed, 0n)
  assert.equal(base.withSeed(42n).seed, 42n)

  const max = 18446744073709551615n
  assert.equal(base.withSeed(max).seed, max)
  assert.equal(typeof base.withSeed(max).seed, 'bigint')

  const aboveSafe = 9007199254740993n
  assert.ok(aboveSafe > BigInt(Number.MAX_SAFE_INTEGER))
  const seeded = base.withSeed(aboveSafe)
  assert.equal(seeded.seed, aboveSafe)
  assert.equal(seeded.seed.toString(), '9007199254740993')

  const noticeSeed = new RightsNotice().withSeed(aboveSafe)
  assert.equal(noticeSeed.seed, aboveSafe)
})

test('invalid numeric inputs are rejected before narrowing', () => {
  const notice = new RightsNotice().withCopyrightHolder('Acme')
  const base = ProtectionRequest.metadataOnly(notice, RightsPolicy.Allowed)
  const options = new ProcessingOptions()

  for (const bad of [Number.NaN, Number.POSITIVE_INFINITY, Number.NEGATIVE_INFINITY, 1.5, -1, Number.MAX_SAFE_INTEGER + 2]) {
    assert.throws(() => base.withMaxDimension(bad), (error) => {
      assert.equal(error.code, ErrorCode.InvalidConfig)
      assert.match(error.message, /maxDimension/)
      return true
    }, `withMaxDimension(${String(bad)}) must be rejected`)
  }

  assert.throws(() => base.withJpegQuality(300), /jpegQuality/)
  assert.throws(() => base.withStegoRedundancy(-3), /stegoRedundancy/)
  assert.throws(() => base.withIntensity(1.5), /intensity/)
  assert.throws(() => base.withIntensity(Number.NaN), /intensity/)
  assert.throws(() => options.withMaxDimension(Number.NaN), /maxDimension/)
  assert.throws(() => ResourceLimitsBuilder.new().withMaxInputBytes(-1), /maxInputBytes/)
  assert.throws(() => base.withContentHash(new Uint8Array([1, 2, 3])), /contentHash/)
})

test('bigint seeds outside u64 are rejected', () => {
  const notice = new RightsNotice().withCopyrightHolder('Acme')
  const base = ProtectionRequest.metadataOnly(notice, RightsPolicy.Allowed)
  assert.throws(() => base.withSeed(-1n), (error) => {
    assert.equal(error.code, ErrorCode.InvalidConfig)
    return true
  })
  assert.throws(() => base.withSeed(18446744073709551616n), /seed/)
  assert.throws(() => base.withSeed(42), (error) => {
    assert.ok(error instanceof Error)
    return true
  }, 'a JavaScript number is not a valid seed')
})

test('hidden-marker tile size validation', () => {
  assert.throws(() => HiddenMarkerMode.tiled(31), /32\.\.=1024/)
  assert.throws(() => HiddenMarkerMode.tiled(1025), /32\.\.=1024/)
  assert.throws(() => HiddenMarkerMode.tiled(64.5), /tileSize/)
  assert.throws(() => HiddenMarkerMode.tiled(Number.NaN), /tileSize/)
  const tiled = HiddenMarkerMode.tiled(64)
  assert.equal(tiled.kind, HiddenMarkerKind.Tiled)
  assert.equal(tiled.tileSize, 64)
  assert.equal(HiddenMarkerMode.bestEffort().tileSize, null)
})

test('deprecated Rust adapters are not projected into the Node surface', () => {
  const api = binding()
  const forbidden = [
    'ProtectionLevel',
    'ProtectionContext',
    'EvidenceProfile',
    'processImage',
    'processImageBytes',
    'verifyLegalNotice',
  ]
  for (const name of forbidden) {
    assert.equal(api[name], undefined, `${name} must not be exported`)
  }
  const declarations = api.__napiBindingTarget !== undefined
  assert.equal(declarations, true)
})
