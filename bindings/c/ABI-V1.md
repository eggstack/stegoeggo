# StegoEggo C ABI v1 — Normative Contract

> **IMPLEMENTED AND QUALIFIED — ABI v1 STABLE.** This document is the
> normative C ABI v1 contract. Language-bindings M009 implemented the leaf
> `cdylib`, generated header, tests, and exact export surface; M010 qualified
> that implementation on the documented five-target matrix and activated the
> ABI v1 stability promise. Existing v1 symbol names, signatures, numeric
> values, and ownership rules are append-only within ABI major 1; breaking
> changes require a new `stegoeggo_v2_*` namespace.

- Authority: `plans/adrs/ADR-0006-versioned-c-abi.md` (accepted; this
  document materializes it and invents no new architecture).
- Contract-design plan:
  `plans/implementation/language-bindings/007-c-abi-contract-design.md`
  (M007; design-only).
- Implementation:
  `plans/implementation/language-bindings/009-c-abi-v1-implementation-foundation.md`
  (M009; 90-symbol leaf ABI implementation).
- Cross-platform qualification:
  `plans/implementation/language-bindings/010-c-abi-v1-cross-platform-qualification.md`
  and `plans/closure/language-bindings/010-status.md` (M010; five-target
  qualification and stability activation).
- ABI major: `1`. ABI minor: `0`. Source package version is independent
  (currently `0.4.2`; query at runtime via `stegoeggo_source_version`).
- Qualified cbindgen header-generation pin: `0.29.4`.
- Language targets: C11 consumer-compatible, C++17 include-compatible.
- First qualification matrix: Linux x86_64 GNU, Linux aarch64 GNU,
  macOS x86_64, macOS arm64, Windows x86_64 MSVC (`§14`).

## 0. Conventions used in this document

- `status` means `stegoeggo_v1_status_t`. Value `0`
  (`STEGOEGGO_V1_OK`) is success; any other value is a failure category.
- `error` means `stegoeggo_v1_error_code_t`. The status return and the
  error-handle code share one numeric space (`§4`).
- `resource` means `stegoeggo_v1_resource_code_t` (`§4`).
- "Owned handle" means a Rust-owned opaque pointer the caller must release
  with the matching `*_free`. "Borrowed" means valid only for the call or
  handle lifetime stated; the caller never frees it.
- "NULL-accepting" is stated per parameter. Anything not stated
  NULL-accepting requires non-NULL; a NULL there returns
  `STEGOEGGO_V1_ERR_INVALID_ARGUMENT`.
- All C symbols in this document are normative. Implementations of ABI v1
  MUST export exactly these `stegoeggo_*` symbols and no other
  `stegoeggo_*` symbol (`§15`).
- Total normative symbol count: **90** (3 bootstrap + 87 `stegoeggo_v1_*`).
- Category totals: notice 20, request 18, resource limits 20, operations 4,
  buffer 3, error 6, execution report 10, verification report 6
  (versioned `stegoeggo_v1_*` total 87; grand total 90 with bootstrap).

## 1. Bootstrap / version symbols (3, unversioned)

```c
uint32_t stegoeggo_abi_version_major(void);
uint32_t stegoeggo_abi_version_minor(void);
const char *stegoeggo_source_version(void);
```

- `stegoeggo_abi_version_major` returns `1`.
- `stegoeggo_abi_version_minor` returns `0`.
- `stegoeggo_source_version` returns a library-owned, NUL-terminated ASCII
  string (the `stegoeggo` package version the library was built from, e.g.
  `"0.4.2"`). The pointer is valid for process lifetime and MUST NOT be
  freed by the caller. It is never NULL.
- All three are infallible, reentrant, thread-safe, and never allocate,
  never set an error, and never unwind.
- Future ABI majors retain these three bootstrap functions unchanged.

## 2. Opaque handles (7 types)

```c
typedef struct stegoeggo_v1_notice_t stegoeggo_v1_notice_t;
typedef struct stegoeggo_v1_request_t stegoeggo_v1_request_t;
typedef struct stegoeggo_v1_resource_limits_t stegoeggo_v1_resource_limits_t;
typedef struct stegoeggo_v1_buffer_t stegoeggo_v1_buffer_t;
typedef struct stegoeggo_v1_execution_report_t stegoeggo_v1_execution_report_t;
typedef struct stegoeggo_v1_verification_report_t stegoeggo_v1_verification_report_t;
typedef struct stegoeggo_v1_error_t stegoeggo_v1_error_t;
```

- The structs are incomplete in the public header. No size, field, layout,
  or alignment property is part of the contract.
- Handles are created only by their constructor functions and destroyed
  only by their matching free function. Callers never stack-allocate,
  `memcpy`, `free`, or `realloc` them.
- Every owned handle has exactly one matching free function; freeing NULL
  is a no-op; double-free and use-after-free are caller contract
  violations (undefined behavior, never a Rust panic).
- The notice and request and limits handles are mutable builders. Mutating
  a handle invalidates no borrowed views (builders expose no views), but a
  concurrent mutation/use/free of the same handle is a contract violation.

## 3. Fixed-width scalar typedefs

```c
typedef uint32_t stegoeggo_v1_status_t;
typedef uint32_t stegoeggo_v1_error_code_t;
typedef uint32_t stegoeggo_v1_resource_code_t;
typedef uint32_t stegoeggo_v1_rights_policy_t;
typedef uint32_t stegoeggo_v1_dmi_value_t;
typedef uint32_t stegoeggo_v1_image_format_t;
typedef uint32_t stegoeggo_v1_metadata_update_policy_t;
typedef uint32_t stegoeggo_v1_preset_t;
typedef uint32_t stegoeggo_v1_authentication_mode_t;
typedef uint32_t stegoeggo_v1_hidden_marker_mode_t;
typedef uint32_t stegoeggo_v1_verification_status_t;
typedef uint32_t stegoeggo_v1_evidence_strength_t;
typedef uint32_t stegoeggo_v1_warning_t;
```

- Plain `uint32_t` typedefs are used, never C enums, so ABI width never
  depends on the consumer compiler.
- Booleans across the ABI are `uint8_t` with `0` = false, `1` = true.
  Any other value where a `uint8_t` boolean is taken is invalid argument.
- Seeds are `uint64_t` full-width. Zero is a valid seed wherever a seed
  is present; presence is carried separately (`has_seed` flags).
- Buffer lengths and counts are `size_t`. Image dimensions are `uint32_t`.
- JPEG quality is `uint8_t`. Redundancy overrides are `uint32_t`
  (valid range 1..=10; validated at operation resolution).

## 4. Numeric code tables (all values frozen for ABI v1)

Values are assigned by this document. They are NOT derived from Rust enum
order or discriminants and MUST NOT change within ABI v1. Ranges not listed
are reserved for additive future constants; passing a reserved/unknown value
as input returns invalid-argument/configuration, never undefined behavior.

### 4.1 Status / error codes

`STEGOEGGO_V1_OK = 0` is the only success value. All other values are both
a call status and the code carried by the resulting error handle.

| Constant | Value | Meaning |
|---|---|---|
| `STEGOEGGO_V1_OK` | 0 | Success. |
| `STEGOEGGO_V1_ERR_INVALID_ARGUMENT` | 1 | ABI misuse detectable before canonical work: NULL where non-NULL is required, nonzero length with NULL pointer, bad length pairing, out-of-range scalar/constant, unknown constant value, failed UTF-8, wrong content-hash length, bad tile-size pairing, empty MAC key bytes. |
| `STEGOEGGO_V1_ERR_INVALID_CONFIGURATION` | 2 | Canonical `Error::Config`, including request-resolution rejections (bad quality/redundancy/intensity, missing MAC key for an HMAC preset, contradictory legal claims surface as warnings, not errors). |
| `STEGOEGGO_V1_ERR_INVALID_FORMAT` | 3 | Canonical `Error::InvalidFormat`. |
| `STEGOEGGO_V1_ERR_ENCODE_DECODE` | 4 | Canonical `Error::ImageDecode`, `ImageEncode`, `Image`, `ImageTruncated`, and `Io` arising inside image work. |
| `STEGOEGGO_V1_ERR_METADATA` | 5 | Canonical `Error::Metadata`. |
| `STEGOEGGO_V1_ERR_STEGANOGRAPHY` | 6 | Canonical `Error::Steganography`. |
| `STEGOEGGO_V1_ERR_INSUFFICIENT_CAPACITY` | 7 | Canonical `Error::InsufficientCapacity` with structured `required`/`available` preserved in details JSON. |
| `STEGOEGGO_V1_ERR_VERIFICATION` | 8 | Canonical `Error::PayloadVerification` and `Error::Crypto`. |
| `STEGOEGGO_V1_ERR_RESOURCE_LIMIT` | 9 | Any of the six canonical resource-limit errors, with the resource category below. |
| `STEGOEGGO_V1_ERR_INTERNAL` | 10 | Canonical `Error::Serialization`, `Error::Iscc` (unreachable: the C leaf enables neither path that produces them; mapped defensively), the async-only `Task` variant (unreachable in synchronous v1), any non-exhaustive future Rust variant not yet classified, and any contained Rust panic (`catch_unwind`). |

Values 11..99 are reserved for additive future error categories.
`UINT32_MAX` is never a valid error code. Unknown future Rust variants map
to `STEGOEGGO_V1_ERR_INTERNAL`; arbitrary C-side numeric values are never
transmuted into Rust enums.

### 4.2 Resource categories

| Constant | Value | Rust source |
|---|---|---|
| `STEGOEGGO_V1_RESOURCE_NONE` | 0 | No resource bound involved (every non-resource-limit error carries this). |
| `STEGOEGGO_V1_RESOURCE_INPUT_BYTES` | 1 | `Error::InputTooLarge`. |
| `STEGOEGGO_V1_RESOURCE_DIMENSIONS` | 2 | `Error::DimensionsExceeded`. |
| `STEGOEGGO_V1_RESOURCE_CONTAINER` | 3 | `Error::ContainerLimitExceeded`. |
| `STEGOEGGO_V1_RESOURCE_METADATA` | 4 | `Error::MetadataLimitExceeded`. |
| `STEGOEGGO_V1_RESOURCE_VERIFICATION_BUDGET` | 5 | `Error::VerificationBudgetExceeded`. |
| `STEGOEGGO_V1_RESOURCE_CARRIER` | 6 | `Error::ResourceLimitExceeded(String)` (carrier report; message only, no invented numerics). |

Values 7..99 reserved. The carrier case reports category `carrier` with
the canonical message and no structured numbers.

### 4.3 Rights policy

| Constant | Value | Rust `RightsPolicy` |
|---|---|---|
| `STEGOEGGO_V1_POLICY_UNSPECIFIED` | 0 | `Unspecified` |
| `STEGOEGGO_V1_POLICY_ALLOWED` | 1 | `Allowed` |
| `STEGOEGGO_V1_POLICY_PROHIBITED_AI_ML_TRAINING` | 2 | `ProhibitedAiMlTraining` |
| `STEGOEGGO_V1_POLICY_PROHIBITED_GENERATIVE_AI_TRAINING` | 3 | `ProhibitedGenerativeAiTraining` |
| `STEGOEGGO_V1_POLICY_PROHIBITED_EXCEPT_SEARCH_INDEXING` | 4 | `ProhibitedExceptSearchIndexing` |
| `STEGOEGGO_V1_POLICY_PROHIBITED_ALL_DATA_MINING` | 5 | `ProhibitedAllDataMining` |
| `STEGOEGGO_V1_POLICY_PROHIBITED_SEE_CONSTRAINTS` | 6 | `ProhibitedSeeConstraints` |

Values 7..99 reserved.

### 4.4 DMI value

| Constant | Value | Rust `DmiValue` |
|---|---|---|
| `STEGOEGGO_V1_DMI_UNSPECIFIED` | 0 | `Unspecified` |
| `STEGOEGGO_V1_DMI_ALLOWED` | 1 | `Allowed` |
| `STEGOEGGO_V1_DMI_PROHIBITED_AI_ML_TRAINING` | 2 | `ProhibitedAiMlTraining` |
| `STEGOEGGO_V1_DMI_PROHIBITED_GEN_AI_ML_TRAINING` | 3 | `ProhibitedGenAiMlTraining` |
| `STEGOEGGO_V1_DMI_PROHIBITED_EXCEPT_SEARCH_ENGINE_INDEXING` | 4 | `ProhibitedExceptSearchEngineIndexing` |
| `STEGOEGGO_V1_DMI_PROHIBITED` | 5 | `Prohibited` |
| `STEGOEGGO_V1_DMI_PROHIBITED_SEE_CONSTRAINTS` | 6 | `ProhibitedSeeConstraints` |

Values 7..99 reserved.

### 4.5 Image format

| Constant | Value | Meaning |
|---|---|---|
| `STEGOEGGO_V1_FORMAT_UNKNOWN` | 0 | Header not recognized (detect-format success with unknown input). |
| `STEGOEGGO_V1_FORMAT_PNG` | 1 | PNG (`ImageOutputFormat::Png`). |
| `STEGOEGGO_V1_FORMAT_JPEG` | 2 | JPEG (`ImageOutputFormat::Jpeg`). |
| `STEGOEGGO_V1_FORMAT_WEBP` | 3 | WebP (`ImageOutputFormat::WebP`). |

Values 4..99 reserved. There is deliberately no "same as input" code on
the wire: `set_output_format` carries presence separately (`§7`).

### 4.6 Metadata update policy

| Constant | Value | Rust `MetadataUpdatePolicy` |
|---|---|---|
| `STEGOEGGO_V1_UPDATE_REPLACE_STEGO_OWNED` | 0 | `ReplaceStegoOwned` (default) |
| `STEGOEGGO_V1_UPDATE_FAIL_ON_CONFLICT` | 1 | `FailOnConflict` |
| `STEGOEGGO_V1_UPDATE_PRESERVE_EXISTING` | 2 | `PreserveExisting` |

Values 3..99 reserved.

### 4.7 Preset

| Constant | Value | Rust `ProtectionPreset` |
|---|---|---|
| `STEGOEGGO_V1_PRESET_LEGAL_NOTICE` | 0 | `LegalNotice` |
| `STEGOEGGO_V1_PRESET_LEGAL_NOTICE_WITH_STEGO` | 1 | `LegalNoticeWithStego` |
| `STEGOEGGO_V1_PRESET_AUTHENTICATED_PROVENANCE` | 2 | `AuthenticatedProvenance` |
| `STEGOEGGO_V1_PRESET_MAXIMAL` | 3 | `Maximal` |

Values 4..99 reserved.

### 4.8 Authentication mode

| Constant | Value | Rust `AuthenticationMode` |
|---|---|---|
| `STEGOEGGO_V1_AUTH_NONE` | 0 | `None` |
| `STEGOEGGO_V1_AUTH_HMAC` | 1 | `Hmac` |

Values 2..99 reserved.

### 4.9 Hidden-marker mode

| Constant | Value | Rust `HiddenMarkerMode` |
|---|---|---|
| `STEGOEGGO_V1_MARKER_DISABLED` | 0 | `Disabled` |
| `STEGOEGGO_V1_MARKER_SEED_ONLY` | 1 | `SeedOnly` |
| `STEGOEGGO_V1_MARKER_BEST_EFFORT` | 2 | `BestEffort` |
| `STEGOEGGO_V1_MARKER_TILED` | 3 | `Tiled { tile_size }` (size in the companion parameter) |

Values 4..99 reserved.

### 4.10 Verification status (data codes, not call statuses)

| Constant | Value | Rust `VerificationStatus` |
|---|---|---|
| `STEGOEGGO_V1_VERIFY_VERIFIED` | 0 | `Verified` |
| `STEGOEGGO_V1_VERIFY_INVALID` | 1 | `Invalid` |
| `STEGOEGGO_V1_VERIFY_NOT_FOUND` | 2 | `NotFound` |

Values 3..99 reserved. These project the canonical hidden-marker status
verbatim: `NotFound` versus `Invalid` is never reinterpreted by the binding.
The summary rule (metadata-only `NotFound` upgrading in `summary_status`)
is NOT a separate code; consumers that need it read `rights_found`
alongside `status` from the same report.

### 4.11 Evidence strength

| Constant | Value | Rust `EvidenceStrength` |
|---|---|---|
| `STEGOEGGO_V1_EVIDENCE_NO_NOTICE_FOUND` | 0 | `NoNoticeFound` |
| `STEGOEGGO_V1_EVIDENCE_METADATA_NOTICE_ONLY` | 1 | `MetadataNoticeOnly` |
| `STEGOEGGO_V1_EVIDENCE_METADATA_NOTICE_AND_BEST_EFFORT_STEGO` | 2 | `MetadataNoticeAndBestEffortStego` |
| `STEGOEGGO_V1_EVIDENCE_METADATA_NOTICE_AND_AUTHENTICATED_PROVENANCE` | 3 | `MetadataNoticeAndAuthenticatedProvenance` |

Values 4..99 reserved.

### 4.12 Protection warnings

| Constant | Value | Rust `ProtectionWarning` |
|---|---|---|
| `STEGOEGGO_V1_WARNING_MISSING_MAC_KEY` | 0 | `MissingMacKey` |
| `STEGOEGGO_V1_WARNING_METADATA_INJECTION_DISABLED` | 1 | `MetadataInjectionDisabled` |
| `STEGOEGGO_V1_WARNING_PROGRESSIVE_JPEG_FALLBACK` | 2 | `ProgressiveJpegFallback` |
| `STEGOEGGO_V1_WARNING_JPEG_REENCODE_FRAGILE` | 3 | `JpegReencodeFragile` |
| `STEGOEGGO_V1_WARNING_LSB_CAPACITY_SKIPPED` | 4 | `LsbCapacitySkipped` |
| `STEGOEGGO_V1_WARNING_DCT_CAPACITY_INSUFFICIENT` | 5 | `DctCapacityInsufficient` |
| `STEGOEGGO_V1_WARNING_CONTRADICTORY_LEGAL_CLAIMS` | 6 | `ContradictoryLegalClaims` |
| `STEGOEGGO_V1_WARNING_MISSING_RIGHTS_CONSTRAINTS` | 7 | `MissingRightsConstraints` |

Values 8..99 reserved. The warning-at-index getter returns `UINT32_MAX`
for NULL reports and out-of-range indexes; `UINT32_MAX` is never a valid
warning code.

## 5. Text and byte input contract

- Pointer + length is the only variable-input form. No C input API depends
  on NUL termination; embedded NULs in text inputs are rejected as invalid
  argument (text must be valid UTF-8 with no interior NUL).
- Input memory is borrowed only for the synchronous call and is copied
  where the Rust object retains it (notice strings, MAC key, timestamp
  override, content hash, resource-limits snapshot).
- Nonzero length requires a non-NULL pointer. Zero length with NULL is
  accepted where the parameter permits empty input and means "empty";
  whether empty means "clear to None" or "invalid" is stated per function.
- Text (notice fields, timestamp override) MUST be valid UTF-8; invalid
  UTF-8 returns invalid argument and never reaches Rust.
- Sizes are checked before Rust slice construction where arithmetic can
  overflow; overflow-prone combinations return invalid argument.
- Content-hash input MUST be exactly 4 bytes when present.
- MAC-key input: NULL + 0 clears to `None`; non-NULL + nonzero copies the
  bytes; non-NULL + 0 is invalid argument (empty keys are not
  representable in C v1, narrower than Rust by design).
- Arbitrary invalid or dangling non-NULL pointers remain caller undefined
  behavior; the binding validates NULL/length/range combinations, not
  pointer validity.
- Maximum text field length: 8192 bytes (`LegalMetadata::MAX_FIELD_LEN`),
  enforced as invalid-argument at the ABI when exceeded, matching Rust
  validation. Timestamp overrides are additionally bounded to 256 bytes.

## 6. Notice lifecycle (20 symbols)

```c
stegoeggo_v1_status_t stegoeggo_v1_notice_create(
    stegoeggo_v1_notice_t **out_notice,
    stegoeggo_v1_error_t **out_error);
void stegoeggo_v1_notice_free(stegoeggo_v1_notice_t *notice);

stegoeggo_v1_status_t stegoeggo_v1_notice_set_copyright_holder(
    stegoeggo_v1_notice_t *notice, const char *text, size_t text_len,
    stegoeggo_v1_error_t **out_error);
stegoeggo_v1_status_t stegoeggo_v1_notice_set_contact_email(
    stegoeggo_v1_notice_t *notice, const char *text, size_t text_len,
    stegoeggo_v1_error_t **out_error);
stegoeggo_v1_status_t stegoeggo_v1_notice_set_license_url(
    stegoeggo_v1_notice_t *notice, const char *text, size_t text_len,
    stegoeggo_v1_error_t **out_error);
stegoeggo_v1_status_t stegoeggo_v1_notice_set_usage_terms(
    stegoeggo_v1_notice_t *notice, const char *text, size_t text_len,
    stegoeggo_v1_error_t **out_error);
stegoeggo_v1_status_t stegoeggo_v1_notice_set_usage_terms_lang(
    stegoeggo_v1_notice_t *notice, const char *text, size_t text_len,
    stegoeggo_v1_error_t **out_error);
stegoeggo_v1_status_t stegoeggo_v1_notice_set_creation_date(
    stegoeggo_v1_notice_t *notice, const char *text, size_t text_len,
    stegoeggo_v1_error_t **out_error);
stegoeggo_v1_status_t stegoeggo_v1_notice_set_ai_constraints(
    stegoeggo_v1_notice_t *notice, const char *text, size_t text_len,
    stegoeggo_v1_error_t **out_error);
stegoeggo_v1_status_t stegoeggo_v1_notice_set_web_statement_of_rights(
    stegoeggo_v1_notice_t *notice, const char *text, size_t text_len,
    stegoeggo_v1_error_t **out_error);
stegoeggo_v1_status_t stegoeggo_v1_notice_set_creator(
    stegoeggo_v1_notice_t *notice, const char *text, size_t text_len,
    stegoeggo_v1_error_t **out_error);
stegoeggo_v1_status_t stegoeggo_v1_notice_set_credit_line(
    stegoeggo_v1_notice_t *notice, const char *text, size_t text_len,
    stegoeggo_v1_error_t **out_error);
stegoeggo_v1_status_t stegoeggo_v1_notice_set_copyright_owner(
    stegoeggo_v1_notice_t *notice, const char *text, size_t text_len,
    stegoeggo_v1_error_t **out_error);
stegoeggo_v1_status_t stegoeggo_v1_notice_set_licensor_name(
    stegoeggo_v1_notice_t *notice, const char *text, size_t text_len,
    stegoeggo_v1_error_t **out_error);
stegoeggo_v1_status_t stegoeggo_v1_notice_set_licensor_email(
    stegoeggo_v1_notice_t *notice, const char *text, size_t text_len,
    stegoeggo_v1_error_t **out_error);
stegoeggo_v1_status_t stegoeggo_v1_notice_set_licensor_url(
    stegoeggo_v1_notice_t *notice, const char *text, size_t text_len,
    stegoeggo_v1_error_t **out_error);
stegoeggo_v1_status_t stegoeggo_v1_notice_set_metadata_date(
    stegoeggo_v1_notice_t *notice, const char *text, size_t text_len,
    stegoeggo_v1_error_t **out_error);
stegoeggo_v1_status_t stegoeggo_v1_notice_set_notice_applied_at(
    stegoeggo_v1_notice_t *notice, const char *text, size_t text_len,
    stegoeggo_v1_error_t **out_error);
stegoeggo_v1_status_t stegoeggo_v1_notice_set_dmi(
    stegoeggo_v1_notice_t *notice, stegoeggo_v1_dmi_value_t dmi,
    stegoeggo_v1_error_t **out_error);
stegoeggo_v1_status_t stegoeggo_v1_notice_set_seed(
    stegoeggo_v1_notice_t *notice, uint8_t has_seed, uint64_t seed,
    stegoeggo_v1_error_t **out_error);
```

- `create` allocates an empty notice (`RightsNotice::new`). `out_notice`
  is required; on entry it is set to NULL before work.
- Each text setter replaces the field. NULL + 0 clears the field to
  `None`. Unknown DMI codes and non-0/1 `has_seed` are invalid argument.
- Coverage note: the 16 text fields are the full canonical Rust set. The
  Python and Node M006 frontends expose 15 text setters (neither exposes
  `usage_terms_lang`); C v1 exposes all 16, a strict superset of the common
  contract. No secret or key material exists on the notice; there is no
  MAC-key setter here by design.
- Borrow rule: request constructors clone the notice. Mutating or freeing
  a notice after building a request does not affect the request.

## 7. Request lifecycle (18 symbols)

```c
stegoeggo_v1_status_t stegoeggo_v1_request_metadata_only(
    const stegoeggo_v1_notice_t *notice, stegoeggo_v1_rights_policy_t policy,
    stegoeggo_v1_request_t **out_request, stegoeggo_v1_error_t **out_error);
stegoeggo_v1_status_t stegoeggo_v1_request_with_hidden_marker(
    const stegoeggo_v1_notice_t *notice, stegoeggo_v1_rights_policy_t policy,
    stegoeggo_v1_request_t **out_request, stegoeggo_v1_error_t **out_error);
stegoeggo_v1_status_t stegoeggo_v1_request_from_preset(
    stegoeggo_v1_preset_t preset, const stegoeggo_v1_notice_t *notice,
    stegoeggo_v1_rights_policy_t policy,
    stegoeggo_v1_request_t **out_request, stegoeggo_v1_error_t **out_error);
void stegoeggo_v1_request_free(stegoeggo_v1_request_t *request);

stegoeggo_v1_status_t stegoeggo_v1_request_set_seed(
    stegoeggo_v1_request_t *request, uint8_t has_seed, uint64_t seed,
    stegoeggo_v1_error_t **out_error);
stegoeggo_v1_status_t stegoeggo_v1_request_set_intensity(
    stegoeggo_v1_request_t *request, float intensity,
    stegoeggo_v1_error_t **out_error);
stegoeggo_v1_status_t stegoeggo_v1_request_set_output_format(
    stegoeggo_v1_request_t *request, uint8_t has_format,
    stegoeggo_v1_image_format_t format, stegoeggo_v1_error_t **out_error);
stegoeggo_v1_status_t stegoeggo_v1_request_set_jpeg_quality(
    stegoeggo_v1_request_t *request, uint8_t quality,
    stegoeggo_v1_error_t **out_error);
stegoeggo_v1_status_t stegoeggo_v1_request_set_progressive_jpeg(
    stegoeggo_v1_request_t *request, uint8_t enabled,
    stegoeggo_v1_error_t **out_error);
stegoeggo_v1_status_t stegoeggo_v1_request_set_max_dimension(
    stegoeggo_v1_request_t *request, uint8_t has_max, uint32_t max_dimension,
    stegoeggo_v1_error_t **out_error);
stegoeggo_v1_status_t stegoeggo_v1_request_set_metadata_update_policy(
    stegoeggo_v1_request_t *request, stegoeggo_v1_metadata_update_policy_t policy,
    stegoeggo_v1_error_t **out_error);
stegoeggo_v1_status_t stegoeggo_v1_request_set_stego_redundancy(
    stegoeggo_v1_request_t *request, uint8_t has_redundancy, uint32_t redundancy,
    stegoeggo_v1_error_t **out_error);
stegoeggo_v1_status_t stegoeggo_v1_request_set_content_hash(
    stegoeggo_v1_request_t *request, uint8_t has_hash,
    const uint8_t *hash, size_t hash_len, stegoeggo_v1_error_t **out_error);
stegoeggo_v1_status_t stegoeggo_v1_request_set_timestamp_override(
    stegoeggo_v1_request_t *request, uint8_t has_ts,
    const char *text, size_t text_len, stegoeggo_v1_error_t **out_error);
stegoeggo_v1_status_t stegoeggo_v1_request_set_mac_key(
    stegoeggo_v1_request_t *request,
    const uint8_t *key, size_t key_len, stegoeggo_v1_error_t **out_error);
stegoeggo_v1_status_t stegoeggo_v1_request_set_resource_limits(
    stegoeggo_v1_request_t *request, const stegoeggo_v1_resource_limits_t *limits,
    stegoeggo_v1_error_t **out_error);
stegoeggo_v1_status_t stegoeggo_v1_request_set_hidden_marker_mode(
    stegoeggo_v1_request_t *request, stegoeggo_v1_hidden_marker_mode_t mode,
    uint32_t tile_size, stegoeggo_v1_error_t **out_error);
stegoeggo_v1_status_t stegoeggo_v1_request_set_authentication_mode(
    stegoeggo_v1_request_t *request, stegoeggo_v1_authentication_mode_t mode,
    stegoeggo_v1_error_t **out_error);
```

- Constructors require a non-NULL notice and a known policy/preset code;
  the notice is cloned. Defaults match Rust: intensity `0.5`, JPEG quality
  `90`, non-progressive, no output-format override, `ReplaceStegoOwned`,
  no redundancy/content-hash/timestamp/seed/MAC/limits override.
- `set_seed`: `has_seed` 0 clears to `None`; 1 sets `Some(seed)` with the
  full 64 bits (`0` is valid). Other flag values are invalid argument.
- `set_intensity`: finite `0.0..=1.0` accepted and stored; non-finite or
  out-of-range is invalid argument at the ABI (narrower than Rust's
  clamp-and-reject-later; C callers get the failure at set time).
- `set_output_format`: `has_format` 0 clears to `None` (same as input)
  and ignores `format`; 1 requires a known non-UNKNOWN format code
  (UNKNOWN with presence is invalid argument).
- `set_jpeg_quality`: any `uint8_t` is stored; `0` is invalid argument
  immediately (quality range is 1..=100; 101..=255 surfaces as invalid
  configuration at protect time via canonical resolution, matching Rust).
- `set_progressive_jpeg`: `enabled` is 0/1; other values invalid argument.
- `set_max_dimension`: `has_max` 0 clears; 1 requires nonzero dimension
  (`0` with presence is invalid argument).
- `set_stego_redundancy`: `has_redundancy` 0 clears; 1 requires 1..=10
  (outside is invalid argument immediately).
- `set_content_hash`: `has_hash` 0 clears (pointer/length ignored, NULL
  accepted); 1 requires non-NULL with exactly 4 bytes.
- `set_timestamp_override`: `has_ts` 0 clears; 1 requires valid UTF-8
  text as in `§5`.
- `set_mac_key`: NULL + 0 clears to `None`; otherwise copies the bytes.
  There is deliberately NO getter: MAC key bytes are write-only and are
  zeroized when the request handle is destroyed. Key bytes never appear in
  errors, reports, getters, or JSON.
- `set_resource_limits`: NULL clears to `None` (canonical defaults apply
  at operation time); non-NULL copies the limits snapshot into the
  request (later mutation of the limits handle has no effect).
- `set_hidden_marker_mode`: unknown mode codes are invalid argument;
  `TILED` requires `tile_size` in 32..=1024; non-tiled modes require
  `tile_size == 0` (nonzero there is invalid argument, catching
  tile-size/mode confusion at the boundary).
- `set_authentication_mode`: unknown codes are invalid argument. Selecting
  HMAC without a MAC key fails at protect time with invalid configuration
  (canonical resolution), not at set time.
- No deprecated `ProtectionLevel`, `ProtectionContext`, or
  `EvidenceProfile` appears anywhere in this surface.

## 8. Resource-limits lifecycle (20 symbols)

```c
stegoeggo_v1_status_t stegoeggo_v1_resource_limits_create(
    stegoeggo_v1_resource_limits_t **out_limits,
    stegoeggo_v1_error_t **out_error);
void stegoeggo_v1_resource_limits_free(stegoeggo_v1_resource_limits_t *limits);

stegoeggo_v1_status_t stegoeggo_v1_resource_limits_set_max_input_bytes(
    stegoeggo_v1_resource_limits_t *limits, size_t value,
    stegoeggo_v1_error_t **out_error);
stegoeggo_v1_status_t stegoeggo_v1_resource_limits_set_max_width(
    stegoeggo_v1_resource_limits_t *limits, uint32_t value,
    stegoeggo_v1_error_t **out_error);
stegoeggo_v1_status_t stegoeggo_v1_resource_limits_set_max_height(
    stegoeggo_v1_resource_limits_t *limits, uint32_t value,
    stegoeggo_v1_error_t **out_error);
stegoeggo_v1_status_t stegoeggo_v1_resource_limits_set_max_png_chunks(
    stegoeggo_v1_resource_limits_t *limits, size_t value,
    stegoeggo_v1_error_t **out_error);
stegoeggo_v1_status_t stegoeggo_v1_resource_limits_set_max_png_chunk_bytes(
    stegoeggo_v1_resource_limits_t *limits, size_t value,
    stegoeggo_v1_error_t **out_error);
stegoeggo_v1_status_t stegoeggo_v1_resource_limits_set_max_jpeg_segments(
    stegoeggo_v1_resource_limits_t *limits, size_t value,
    stegoeggo_v1_error_t **out_error);
stegoeggo_v1_status_t stegoeggo_v1_resource_limits_set_max_jpeg_segment_bytes(
    stegoeggo_v1_resource_limits_t *limits, size_t value,
    stegoeggo_v1_error_t **out_error);
stegoeggo_v1_status_t stegoeggo_v1_resource_limits_set_max_webp_riff_chunks(
    stegoeggo_v1_resource_limits_t *limits, size_t value,
    stegoeggo_v1_error_t **out_error);
stegoeggo_v1_status_t stegoeggo_v1_resource_limits_set_max_webp_riff_bytes(
    stegoeggo_v1_resource_limits_t *limits, size_t value,
    stegoeggo_v1_error_t **out_error);
stegoeggo_v1_status_t stegoeggo_v1_resource_limits_set_max_xmp_bytes(
    stegoeggo_v1_resource_limits_t *limits, size_t value,
    stegoeggo_v1_error_t **out_error);
stegoeggo_v1_status_t stegoeggo_v1_resource_limits_set_max_xml_depth(
    stegoeggo_v1_resource_limits_t *limits, size_t value,
    stegoeggo_v1_error_t **out_error);
stegoeggo_v1_status_t stegoeggo_v1_resource_limits_set_max_xml_properties(
    stegoeggo_v1_resource_limits_t *limits, size_t value,
    stegoeggo_v1_error_t **out_error);
stegoeggo_v1_status_t stegoeggo_v1_resource_limits_set_max_metadata_fields(
    stegoeggo_v1_resource_limits_t *limits, size_t value,
    stegoeggo_v1_error_t **out_error);
stegoeggo_v1_status_t stegoeggo_v1_resource_limits_set_max_metadata_field_bytes(
    stegoeggo_v1_resource_limits_t *limits, size_t value,
    stegoeggo_v1_error_t **out_error);
stegoeggo_v1_status_t stegoeggo_v1_resource_limits_set_max_payload_bytes(
    stegoeggo_v1_resource_limits_t *limits, size_t value,
    stegoeggo_v1_error_t **out_error);
stegoeggo_v1_status_t stegoeggo_v1_resource_limits_set_max_detached_manifest_bytes(
    stegoeggo_v1_resource_limits_t *limits, size_t value,
    stegoeggo_v1_error_t **out_error);
stegoeggo_v1_status_t stegoeggo_v1_resource_limits_set_max_tile_extraction_origins(
    stegoeggo_v1_resource_limits_t *limits, size_t value,
    stegoeggo_v1_error_t **out_error);
stegoeggo_v1_status_t stegoeggo_v1_resource_limits_set_max_verification_seeds(
    stegoeggo_v1_resource_limits_t *limits, size_t value,
    stegoeggo_v1_error_t **out_error);
```

- `create` returns the canonical defaults (`ResourceLimits::default`).
  There is no separate "defaults" symbol; create IS defaults.
- All 18 current `ResourceLimits` fields are representable (Node M006
  parity; the Python M006 builder exposes 12 of these, so C v1 is a strict
  superset of the common Python/Node contract).
- Setters store the value verbatim; no setter fails on a non-NULL handle.
  A NULL handle is invalid argument. `size_t` carries the Rust `usize`
  fields; `uint32_t` carries width/height.

## 9. Core operations (4 symbols, synchronous)

```c
stegoeggo_v1_status_t stegoeggo_v1_detect_format(
    const uint8_t *data, size_t data_len,
    stegoeggo_v1_image_format_t *out_format,
    stegoeggo_v1_error_t **out_error);

stegoeggo_v1_status_t stegoeggo_v1_protect(
    const uint8_t *data, size_t data_len,
    const stegoeggo_v1_request_t *request,
    stegoeggo_v1_buffer_t **out_data,
    stegoeggo_v1_error_t **out_error);

stegoeggo_v1_status_t stegoeggo_v1_protect_with_report(
    const uint8_t *data, size_t data_len,
    const stegoeggo_v1_request_t *request,
    stegoeggo_v1_buffer_t **out_data,
    stegoeggo_v1_execution_report_t **out_report,
    stegoeggo_v1_error_t **out_error);

stegoeggo_v1_status_t stegoeggo_v1_verify(
    const uint8_t *data, size_t data_len,
    const uint8_t *mac_key, size_t mac_key_len,
    const stegoeggo_v1_resource_limits_t *limits,
    stegoeggo_v1_verification_report_t **out_report,
    stegoeggo_v1_error_t **out_error);
```

- Semantics are frozen as stated; argument order/naming above is normative
  (no ABI reason to deviate from the plan proposal was found).
- `detect_format` maps `ImageOutputFormat::from_magic_bytes`: PNG/JPEG/WebP
  headers yield the matching code; anything else (including empty input)
  succeeds with `STEGOEGGO_V1_FORMAT_UNKNOWN`. `out_format` is required.
  Unknown input is data, never a panic, never an error.
- `protect` delegates to canonical `process_request_bytes`;
  `protect_with_report` delegates to `process_request_bytes_with_report`.
  There is deliberately NO separate `protect_with_warnings` operation:
  warning count/value access on the execution-report handle covers it, and
  redundant ABI surface is rejected.
- `verify` delegates to `verify_image_bytes_report` (NULL/empty key) or
  `verify_image_bytes_report_with_limits` semantics with the caller limits
  when `limits != NULL` (NULL limits mean canonical defaults). The MAC key
  follows `§5`: NULL + 0 means absent; an empty non-NULL key is invalid
  argument. Verification evidence (`NotFound` vs `Invalid`) is projected
  verbatim from the canonical report; the binding performs no
  reinterpretation.
- Encoded bytes are the boundary in both directions (ADR-0003). Pixel
  handles, file paths, batch calls, callbacks, async, and cancellation are
  out of v1 by design.
- All four operations are synchronous and reentrant. The request/limits
  inputs are borrowed for the call only.

## 10. Output buffer ownership (3 symbols)

```c
const uint8_t *stegoeggo_v1_buffer_data(const stegoeggo_v1_buffer_t *buffer);
size_t stegoeggo_v1_buffer_len(const stegoeggo_v1_buffer_t *buffer);
void stegoeggo_v1_buffer_free(stegoeggo_v1_buffer_t *buffer);
```

- The data view is valid until buffer free and MUST NOT be freed,
  reallocated, or written by C. Rust allocation capacity is never exposed.
- NULL buffer yields NULL data and length 0. Trivial accessors never
  allocate an error object and never unwind (`§13`).

## 11. Error contract (6 symbols)

```c
stegoeggo_v1_error_code_t stegoeggo_v1_error_code(
    const stegoeggo_v1_error_t *error);
stegoeggo_v1_resource_code_t stegoeggo_v1_error_resource(
    const stegoeggo_v1_error_t *error);
const char *stegoeggo_v1_error_message_data(
    const stegoeggo_v1_error_t *error);
size_t stegoeggo_v1_error_message_len(const stegoeggo_v1_error_t *error);
stegoeggo_v1_buffer_t *stegoeggo_v1_error_details_json(
    const stegoeggo_v1_error_t *error);
void stegoeggo_v1_error_free(stegoeggo_v1_error_t *error);
```

- Per-call explicit errors; there is NO last-error / thread-local state.
- Every fallible call (`status` return + `out_error`): on entry all owned
  out-handles are initialized to NULL; on success status is 0 and (when
  `out_error != NULL`) `*out_error` is NULL; on failure status is nonzero
  and (when `out_error != NULL`) `*out_error` is an owned error handle.
  `out_error` itself may be NULL: the caller then receives only the numeric
  status and no handle. An error out-pointer is never required to receive
  the numeric return code.
- `error_code` on NULL returns `STEGOEGGO_V1_ERR_INVALID_ARGUMENT`;
  `error_resource` on NULL returns `STEGOEGGO_V1_RESOURCE_NONE`;
  message accessors on NULL return NULL / 0.
- The caller owns every handle written through `out_error`. The library never
  frees a previously returned handle, because it cannot tell whether the caller
  already released it. Reusing one `error_t *` slot across calls therefore leaks
  unless the caller frees the previous handle first and resets its own variable
  to NULL. The free function takes the handle by value and cannot clear the
  caller's variable, so that reset is the caller's responsibility: treat a slot
  as owned until it has been freed *and* nulled.
- The message is UTF-8, borrowed from the error handle (valid until free),
  and never contains MAC key bytes or other secret material.
- Strategy decision (frozen): stable category/resource/message accessors
  PLUS a versioned details-JSON buffer (plan option 2). No per-field typed
  nullable getter surface is added in v1.

### 11.1 Details-JSON schema (`schema_version = 1`, additive-only)

`error_details_json` returns a Rust-owned buffer handle, or NULL on NULL
input (or allocation failure). The buffer holds a JSON object:

```json
{
  "schema_version": 1,
  "code": 7,
  "resource": 0,
  "message": "Insufficient capacity: need 100 carrier units, have 10",
  "required": 100,
  "available": 10
}
```

- Always present: `schema_version` (`1`), `code`, `resource`, `message`.
- Conditionally present, exactly when the Rust variant provides them:
  `required`/`available` (capacity); `size`/`limit` (input-too-large,
  metadata-limit); `width`/`height`/`max_width`/`max_height` (dimensions);
  `kind` (string, container/metadata/verification-budget detail);
  `count` (container/verification-budget detail).
- The schema MUST never invent unavailable values: the carrier case
  (`resource = 6`) carries only `code`/`resource`/`message`. Unknown future
  Rust variants carry only the base four fields with `code = 10`.
- Within ABI v1, only additive field additions with the same
  `schema_version` are allowed. A breaking shape change requires a new
  schema version exposed through additive API, never a silent redefinition.

## 12. Report contract

### 12.1 Execution report (10 symbols)

```c
stegoeggo_v1_rights_policy_t stegoeggo_v1_execution_report_effective_policy(
    const stegoeggo_v1_execution_report_t *report);
uint8_t stegoeggo_v1_execution_report_has_dmi(
    const stegoeggo_v1_execution_report_t *report);
stegoeggo_v1_dmi_value_t stegoeggo_v1_execution_report_effective_dmi(
    const stegoeggo_v1_execution_report_t *report);
uint8_t stegoeggo_v1_execution_report_metadata_injected(
    const stegoeggo_v1_execution_report_t *report);
uint8_t stegoeggo_v1_execution_report_stego_attempted(
    const stegoeggo_v1_execution_report_t *report);
uint8_t stegoeggo_v1_execution_report_stego_succeeded(
    const stegoeggo_v1_execution_report_t *report);
uint8_t stegoeggo_v1_execution_report_format_transcoded(
    const stegoeggo_v1_execution_report_t *report);
size_t stegoeggo_v1_execution_report_warning_count(
    const stegoeggo_v1_execution_report_t *report);
stegoeggo_v1_warning_t stegoeggo_v1_execution_report_warning_at(
    const stegoeggo_v1_execution_report_t *report, size_t index);
void stegoeggo_v1_execution_report_free(
    stegoeggo_v1_execution_report_t *report);
```

- Scalar getters return `0`/false-equivalent on NULL report, except
  `warning_at`, which returns `UINT32_MAX` on NULL report or out-of-range
  index (never a valid warning code).
- `effective_dmi` is meaningful only when `has_dmi` is 1; otherwise it
  returns `STEGOEGGO_V1_DMI_UNSPECIFIED`.
- Decision (frozen): complete execution-report JSON is NOT in v1. All
  operational facts have direct getters; `resource_usage` and
  `embed_summary` depth stays behind Rust until a future additive v1 getter
  or versioned JSON is justified. This keeps v1 small and avoids freezing
  a second schema prematurely.

### 12.2 Verification report (6 symbols)

```c
stegoeggo_v1_verification_status_t stegoeggo_v1_verification_report_status(
    const stegoeggo_v1_verification_report_t *report);
stegoeggo_v1_evidence_strength_t stegoeggo_v1_verification_report_evidence_strength(
    const stegoeggo_v1_verification_report_t *report);
uint8_t stegoeggo_v1_verification_report_rights_found(
    const stegoeggo_v1_verification_report_t *report);
uint8_t stegoeggo_v1_verification_report_authenticated(
    const stegoeggo_v1_verification_report_t *report);
stegoeggo_v1_buffer_t *stegoeggo_v1_verification_report_json(
    const stegoeggo_v1_verification_report_t *report);
void stegoeggo_v1_verification_report_free(
    stegoeggo_v1_verification_report_t *report);
```

- `status` is the canonical hidden-marker status verbatim
  (`Verified`/`Invalid`/`NotFound`); `rights_found` is the canonical
  `rights.found`; `authenticated` requires attempted + HMAC-verified +
  key-matched, exactly as `compute_evidence_strength` defines it.
- Scalar getters on NULL return `NOT_FOUND` / `NO_NOTICE_FOUND` / 0.
- Complete JSON uses the already-stable `VerificationReport` JSON schema
  (`STABILITY.md` "Machine-Readable Schemas"): the binding serializes the
  canonical Rust report unchanged. Returned as a Rust-owned buffer handle;
  NULL on NULL input (or allocation failure). No second verification JSON
  schema is invented.

## 13. Panic / FFI entry contract

- Every exported fallible function uses one common internal wrapper that,
  in order: (1) validates required out-pointers; (2) initializes all owned
  outputs to NULL/zero; (3) validates NULL/length/range/constant rules
  before any unsafe conversion; (4) runs the Rust work inside
  `catch_unwind(AssertUnwindSafe(..))`; (5) maps canonical `Error` to the
  frozen code/resource/details of `§4`/`§11`; (6) maps a caught panic to
  `STEGOEGGO_V1_ERR_INTERNAL` with resource `NONE` and a fixed message
  that carries no panic payload text; (7) never unwinds across `extern "C"`
  (all exports are the non-unwinding C ABI).
- Trivial accessors (`buffer_data/len`, scalar getters) and destructors
  MUST NOT unwind either, but they need not allocate an error object; their
  NULL-input behavior is defined per function above.
- OOM (allocation failure) and process-abort conditions that do not unwind
  are explicitly outside the `catch_unwind` guarantee; resource limits are
  the primary defense against hostile oversized inputs.
- All `unsafe` required by this boundary lives in the leaf `bindings/c`
  crate only. Root and carrier crates keep `#![forbid(unsafe_code)]`.

## 14. Thread / reentrancy contract

- No global mutable request/error state exists; there is no last-error TLS.
- Independent handles may be used concurrently from multiple threads, and
  independent operations may run concurrently (callers own thread
  scheduling).
- A single mutable handle (notice/request/limits under construction) MUST
  NOT be mutated, freed, or used concurrently from multiple threads.
- Same-handle concurrent read-only use (e.g. sharing one request across
  threads for parallel `protect`) is explicitly UNSUPPORTED in v1. M010 did
  not qualify that behavior, so Rust `Sync` implementation details do not
  become part of the C contract.
- v1 operations are synchronous. No callbacks into C, no async runtime, no
  worker threads, no cancellation.

## 15. Header-generation contract (ABI v1)

- The qualified header-generation tool is cbindgen `0.29.4` (it declares
  Rust 1.74, below the project's Rust 1.89 MSRV). A future generator upgrade
  requires contract/header-drift review before it can replace this qualified
  baseline.
- cbindgen runs as a standalone development/qualification tool, never as a
  `build.rs` dependency and never as a prerequisite of ordinary root Rust
  builds/checks.
- The generated `bindings/c/include/stegoeggo.h` is committed to the
  repository. Regeneration drift in CI/qualification is a failure.
- Generation configuration (normative): C language output with C++
  compatibility guards; explicit include guard; `usize_is_size_t = true`;
  opaque-handle emission (no struct bodies); only the 90 documented symbols
  plus the scalar typedefs/constants; `stdint.h`/`stddef.h` includes.
- The header MUST compile as C11 (`-std=c11 -Wall -Wextra -Werror`) and as
  C++17 (`-std=c++17 -Wall -Wextra -Werror`); the C++17 compile is a smoke
  requirement on every matrix target, full C11 load/link/run on all five.
- Staticlib distribution is explicitly deferred (unresolved transitive
  symbol/link contract); the first artifact contract is `cdylib` only.

## 16. Symbol manifest and platform qualification contract

- The normative symbol inventory is the 90 symbols named in
  `§1`/`§6`–`§12` of this document and materialized in the checked
  `bindings/c/abi-v1-symbols.txt` (one symbol per line, sorted).
  Qualification fails on any missing expected symbol or any unexpected
  exported `stegoeggo_*` symbol, inspected with platform tooling
  (`nm -D --defined-only` on Linux, `nm -gU` on macOS,
  `dumpbin /EXPORTS` on Windows).
- The qualified M010 matrix (same five native distribution architectures as
  the Python/Node qualification):

| Target | Native smoke |
|---|---|
| Linux x86_64 GNU | GCC/Clang C11 load/link/run |
| Linux aarch64 GNU | native ARM64 C11 load/link/run |
| macOS x86_64 | clang C11 load/link/run |
| macOS arm64 | clang C11 load/link/run |
| Windows x86_64 MSVC | cl.exe C compile/link/run |

- Every target also includes the header as C++; Linux x86_64 performs the
  full C++17 compatibility compile. Rust-internal symbols accidentally
  exported from the `cdylib` are a release blocker.

## 17. Design-review fixtures (non-shipping contract sketches)

The five sketches below are normative for ownership flow (every allocated
handle has exactly one destructor on every path; no `free()` touches Rust
memory; error paths do not leak earlier handles). They are pseudocode, not
shipped tests.

### A. Metadata-only protect lifecycle

```c
stegoeggo_v1_notice_t *notice = NULL;
stegoeggo_v1_request_t *req = NULL;
stegoeggo_v1_buffer_t *out = NULL;
stegoeggo_v1_error_t *err = NULL;
stegoeggo_v1_notice_t *n = NULL;

/* ... create notice, set fields, create request, protect, use
   stegoeggo_v1_buffer_data/len, then free out, req, notice.
   Any nonzero status jumps to fail, where only non-NULL handles
   created so far are freed, plus err. */
```

### B. Hidden-marker + HMAC protect lifecycle

Create notice, `request_with_hidden_marker`, `set_mac_key` (copied;
caller keeps owning its key bytes), optional `set_seed` with full
`uint64_t`, `protect`, free in reverse. The MAC key is never readable
back through any getter, error, or report.

### C. Verification + JSON extraction

`verify` with key (or NULL key) and optional limits handle; read
`status`/`evidence_strength`/`rights_found`/`authenticated`; extract the
complete stable-schema JSON buffer and free it separately from the report.

### D. Resource-limit error handling

Build limits via `create` + setters, attach with `set_resource_limits`
(snapshot copy) or pass directly to `verify`; on `RESOURCE_LIMIT` status
read `error_resource` for branching (`input_bytes`, `dimensions`,
`container`, `metadata`, `verification_budget`, `carrier`) and
`error_details_json` for numbers where the Rust variant provides them.

### E. Cleanup on mid-operation failure

Every fallible call NULL-initializes outputs first, so a failure after a
successful earlier step frees only the earlier non-NULL handles; the
failed call's own outputs are already NULL and the single error handle is
freed once. No path frees the same handle twice or frees Rust memory
with C `free()`.

## 18. What v1 deliberately excludes

File-path APIs, batch APIs, callback/async/cancellation APIs, detached
manifest/signature experimental features, generic carrier/stego-only APIs,
C++ wrapper classes, WASM/Swift/JNI/.NET wrappers, staticlib distribution,
and registry publication. Python and Node remain direct Rust bindings and
MUST NOT be moved onto C.
