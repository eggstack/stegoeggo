import { readFileSync } from 'node:fs'
import { createHash } from 'node:crypto'
import { createRequire } from 'node:module'
import { fileURLToPath } from 'node:url'
import path from 'node:path'

const here = path.dirname(fileURLToPath(import.meta.url))
const require = createRequire(import.meta.url)

/**
 * Repository-authoritative conformance fixtures. The Node binding reads the
 * root fixture set directly so no third copy is created under `bindings/node`.
 */
export const FIXTURE_DIR = path.resolve(here, '..', '..', '..', 'tests', 'fixtures', 'conformance', 'canonical')

/** Reads a canonical conformance fixture. */
export function fixture (name) {
  return readFileSync(path.join(FIXTURE_DIR, name))
}

/** The canonical conformance fixtures this binding's tests consume. */
export const CANONICAL_FIXTURES = [
  'canonical_independent.png',
  'canonical_independent.jpg',
  'canonical_independent.webp',
  'canonical_complete.png',
  'canonical_complete.jpg',
  'canonical_complete.webp',
  'canonical_policy_only.png',
  'canonical_unicode.png',
  'canonical_unicode.webp',
  'canonical_copyright_only.png',
  'canonical_multi_creator.png',
  'canonical_multi_creator.webp',
  'canonical_alt_prefix.png',
  'canonical_alt_prefix.jpg',
]

/** Lazily loads the public CommonJS binding entry. */
let cached = null
export function binding () {
  if (cached === null) {
    cached = require('../stegoeggo.js')
  }
  return cached
}

/** Applies optional request overrides shared by the request helpers. */
function applyRest (request, rest) {
  let next = request
  if (rest.intensity !== undefined) next = next.withIntensity(rest.intensity)
  if (rest.jpegQuality !== undefined) next = next.withJpegQuality(rest.jpegQuality)
  if (rest.progressiveJpeg) next = next.withProgressiveJpeg()
  if (rest.maxDimension !== undefined) next = next.withMaxDimension(rest.maxDimension)
  if (rest.stegoRedundancy !== undefined) next = next.withStegoRedundancy(rest.stegoRedundancy)
  if (rest.contentHash !== undefined) next = next.withContentHash(rest.contentHash)
  if (rest.resourceLimits !== undefined) next = next.withResourceLimits(rest.resourceLimits)
  if (rest.hiddenMarkerMode !== undefined) {
    next = next.withHiddenMarkerMode(rest.hiddenMarkerMode)
  }
  if (rest.authentication !== undefined) next = next.withAuthentication(rest.authentication)
  if (rest.processing !== undefined) next = next.withProcessing(rest.processing)
  return next
}

/** A metadata-only request pinned to a seed and timestamp for determinism. */
export function metadataRequest (options = {}) {
  const {
    holder = 'Parity Co',
    policy,
    format,
    seed = 4242n,
    timestamp = '2026-03-03T03:03:03Z',
    ...rest
  } = options
  const { RightsNotice, ProtectionRequest, RightsPolicy } = binding()
  const notice = new RightsNotice().withCopyrightHolder(holder)
  let request = ProtectionRequest.metadataOnly(notice, policy ?? RightsPolicy.ProhibitedAiMlTraining)
  if (format !== undefined) {
    request = request.withOutputFormat(format)
  }
  if (seed !== null) {
    request = request.withSeed(seed)
  }
  if (timestamp !== null) {
    request = request.withTimestampOverride(timestamp)
  }
  return applyRest(request, rest)
}

/** A best-effort hidden-marker request. */
export function hiddenMarkerRequest (options = {}) {
  const {
    holder = 'Marker Co',
    policy,
    seed = 7n,
    timestamp = '2026-04-04T04:04:04Z',
    macKey = null,
    ...rest
  } = options
  const { RightsNotice, ProtectionRequest, RightsPolicy } = binding()
  const notice = new RightsNotice().withCopyrightHolder(holder)
  const base = ProtectionRequest.withHiddenMarker(notice, policy ?? RightsPolicy.ProhibitedAiMlTraining)
  let request = base.withSeed(seed).withTimestampOverride(timestamp)
  if (macKey) {
    request = request.withMacKey(macKey)
  }
  return applyRest(request, rest)
}

/** SHA-256 hex digest of a byte buffer. */
export function sha256 (data) {
  return createHash('sha256').update(data).digest('hex')
}

/** Awaits a rejected Promise and returns the rejection value. */
export async function rejection (promise) {
  try {
    await promise
  } catch (error) {
    return error
  }
  throw new Error('expected the operation to reject')
}
