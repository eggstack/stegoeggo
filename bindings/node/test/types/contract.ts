import {
  AuthenticationMode,
  DmiValue,
  ErrorCode,
  EvidenceStrength,
  HiddenMarkerMode,
  ImageOutputFormat,
  ProcessingOptions,
  ProtectionPreset,
  ProtectionRequest,
  ResourceLimitsBuilder,
  RightsNotice,
  RightsPolicy,
  VerificationStatus,
  detectFormat,
  protect,
  protectWithReport,
  protectWithWarnings,
  stegoeggoVersion,
  verify,
  type ExecutionReport,
  type ProtectionWarning,
  type StegoEggoError,
  type VerificationReport,
} from '../..'

const SEED: bigint = 18446744073709551615n

function metadataRequest(): ProtectionRequest {
  const notice = new RightsNotice()
    .withCopyrightHolder('Contract Co')
    .withCreator('Contract Creator')
    .withDmi(DmiValue.ProhibitedAiMlTraining)
  return ProtectionRequest.metadataOnly(notice, RightsPolicy.ProhibitedAiMlTraining)
    .withSeed(SEED)
    .withTimestampOverride('2026-06-06T06:06:06Z')
}

function markerRequest(key: Uint8Array): ProtectionRequest {
  const notice = new RightsNotice().withCopyrightHolder('Contract Marker')
  const processing = new ProcessingOptions()
    .withOutputFormat(ImageOutputFormat.Png)
    .withJpegQuality(90)
    .withStegoRedundancy(2)
  const limits = ResourceLimitsBuilder.new().withMaxInputBytes(1 << 26).build()
  return ProtectionRequest.withHiddenMarker(notice, RightsPolicy.ProhibitedAiMlTraining)
    .withSeed(7n)
    .withTimestampOverride('2026-06-06T06:06:06Z')
    .withProcessing(processing)
    .withMacKey(key)
    .withResourceLimits(limits)
    .withHiddenMarkerMode(HiddenMarkerMode.bestEffort())
    .withAuthentication(AuthenticationMode.Hmac)
}

function presetRequest(): ProtectionRequest {
  const notice = new RightsNotice().withCopyrightHolder('Contract Preset')
  return ProtectionRequest.fromPreset(
    ProtectionPreset.AuthenticatedProvenance,
    notice,
    RightsPolicy.ProhibitedAiMlTraining
  ).withSeed(0n)
}

function narrowError(error: unknown): string {
  if (error instanceof Error && 'code' in error) {
    const coded = error as StegoEggoError
    switch (coded.code) {
      case ErrorCode.ResourceLimit:
        return `${coded.resource ?? 'unknown'} ${coded.limit ?? -1}`
      case ErrorCode.InsufficientCapacity:
        return `${coded.required ?? -1} ${coded.available ?? -1}`
      default:
        return coded.code
    }
  }
  return 'unknown'
}

export async function exercise(data: Uint8Array, key: Uint8Array): Promise<string> {
  const version: string = stegoeggoVersion()
  const format: string | null = detectFormat(data)

  const protectedBytes: Buffer = await protect(data, metadataRequest())
  const warned: { data: Buffer, warnings: ProtectionWarning[] } = await protectWithWarnings(
    data,
    markerRequest(key)
  )
  const reported: { data: Buffer, report: ExecutionReport } = await protectWithReport(
    data,
    presetRequest()
  )
  const report: VerificationReport = await verify(reported.data, { macKey: key })

  const holder: string | null = report.rights.copyrightHolder
  const seed: bigint | null = report.hiddenMarker.seed
  const strength: EvidenceStrength = report.evidenceStrength
  const status: VerificationStatus = report.status
  const warnings: ProtectionWarning[] = reported.report.warnings

  let outcome = `${version} ${format ?? 'unknown'} ${holder ?? 'none'}`
  outcome += ` ${seed === null ? 'no-seed' : seed.toString()} ${strength} ${status}`
  outcome += ` ${warned.warnings.length} ${warnings.length} ${protectedBytes.length}`
  try {
    await protect(data, metadataRequest().withMaxDimension(-1))
  } catch (error) {
    outcome += ` ${narrowError(error)}`
  }
  return outcome
}
