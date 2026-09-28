import { createRequire } from 'node:module'

/**
 * ESM entry point for the StegoEggo napi-rs binding.
 *
 * Re-exports the CommonJS surface (including the generated napi loader and the
 * Promise-based public operations) so `import` and `require` observe exactly
 * the same runtime values.
 */
const require = createRequire(import.meta.url)
const binding = require('./stegoeggo.js')

export const {
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
  detectFormat,
  protect,
  protectWithReport,
  protectWithWarnings,
  stegoeggoVersion,
  toPublicError,
  verify,
} = binding

export default binding
