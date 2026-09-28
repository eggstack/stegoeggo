/**
 * Public type contract for `@eggstack/stegoeggo`.
 *
 * Request/configuration state, enum value sets, and report projections are
 * re-exported from the napi-rs generated declarations. The four public
 * Promise-based operations and the structured error contract are declared
 * here because `AsyncTask::reject` cannot carry structured properties: the
 * runtime wrapper (`stegoeggo.js`) builds a normal `Error` with `code` plus the
 * structured fields.
 *
 * The generated `index.d.ts` is committed and drift-checked; this file must
 * never be used to hide a mismatched runtime type.
 */

export {
  AuthenticationMode,
  AuthenticationVerification,
  BindingVerification,
  Diagnostic,
  DiagnosticLevel,
  DmiValue,
  ErrorCode,
  EvidenceChannel,
  EvidenceStrength,
  ExecutionReport,
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
  ResourceUsage,
  RightsNotice,
  RightsPolicy,
  RightsVerification,
  TrustEvaluation,
  VerificationReport,
  VerificationStatus,
} from './index.js'

export type { NativeError } from './index.js'

import type {
  AuthenticationMode,
  ErrorCode,
  ExecutionReport,
  NativeError,
  ProtectionRequest,
  ProtectionWarning,
  ResourceLimits,
  VerificationReport,
} from './index.js'

export declare function detectFormat(data: Uint8Array): string | null

/** The wrapped StegoEggo library version, identical to the package version. */
export declare function stegoeggoVersion(): string

/** Options accepted by {@link verify}. */
export interface VerifyOptions {
  /** HMAC/MAC key used for authenticated verification. */
  macKey?: Uint8Array | null
  /** Parser hardening limits applied to the untrusted input. */
  resourceLimits?: ResourceLimits | null
}

/** Result of {@link protectWithWarnings}. */
export interface ProtectWithWarningsResult {
  data: Buffer
  warnings: ProtectionWarning[]
}

/** Result of {@link protectWithReport}. */
export interface ProtectWithReportResult {
  data: Buffer
  report: ExecutionReport
}

/** Structured properties attached to a rejected operation error. */
export interface StegoEggoErrorDetails {
  /** Resource category: `input_bytes`, `dimensions`, `container`, `metadata`, or `verification_budget`. */
  resource?: string
  /** Required carrier units for `ERR_STEGOEGGO_INSUFFICIENT_CAPACITY`. */
  required?: number
  /** Available carrier units for `ERR_STEGOEGGO_INSUFFICIENT_CAPACITY`. */
  available?: number
  /** Observed size for `input_bytes` and `metadata` limit failures. */
  size?: number
  /** Configured limit. */
  limit?: number
  /** Observed image width for `dimensions` limit failures. */
  width?: number
  /** Observed image height for `dimensions` limit failures. */
  height?: number
  /** Configured maximum width for `dimensions` limit failures. */
  maxWidth?: number
  /** Configured maximum height for `dimensions` limit failures. */
  maxHeight?: number
  /** Container or budget kind for `container` and `verification_budget` failures. */
  kind?: string
  /** Observed count for `container` and `verification_budget` failures. */
  count?: number
}

/** The public rejection type of every Promise-based operation. */
export type StegoEggoError = Error & StegoEggoErrorDetails & { code: ErrorCode }

/**
 * Protects encoded PNG/JPEG/WebP bytes with the canonical request.
 *
 * Runs on a libuv worker thread. Rejects with a {@link StegoEggoError}.
 */
export declare function protect(
  data: Uint8Array,
  request: ProtectionRequest
): Promise<Buffer>

/** Protects encoded bytes and returns the degradation warnings. */
export declare function protectWithWarnings(
  data: Uint8Array,
  request: ProtectionRequest
): Promise<ProtectWithWarningsResult>

/** Protects encoded bytes and returns the execution report. */
export declare function protectWithReport(
  data: Uint8Array,
  request: ProtectionRequest
): Promise<ProtectWithReportResult>

/** Verifies encoded PNG/JPEG/WebP bytes and returns the canonical report. */
export declare function verify(
  data: Uint8Array,
  options?: VerifyOptions | null
): Promise<VerificationReport>

/** Builds the public `Error` for a native failure DTO. */
export declare function toPublicError(dto: NativeError): StegoEggoError
