# Request Resolution

**Source:** `src/protected/resolve.rs`

Resolves a `ProtectionRequest` into an immutable `ResolvedProtectionPlan`. This is the single validation point before any image processing occurs.

## Flow

```
ProtectionRequest (user constructs)
        │
        ▼
resolve_request(request, input_format) → ResolvedProtectionPlan
        │
        ▼
process_request_bytes(img_bytes, &plan) → Vec<u8>
```

## `resolve_request()`

```rust
pub fn resolve_request(
    request: &ProtectionRequest,
    input_format: ImageOutputFormat,
) -> Result<ResolvedProtectionPlan>
```

### Validation Steps

1. **Channel validation** — `validate_channels()` checks that:
   - HMAC authentication requires a MAC key
   - `ProhibitedSeeConstraints` policy requires `ai_constraints` or `web_statement_of_rights`
   - Non-Unspecified rights policy requires `rights_metadata` to be enabled

2. **DMI resolution** — Maps `RightsPolicy` to `DmiValue`:
   - `Unspecified` → `None`
   - All others → `Some(DmiValue::from(policy))`

3. **Seed resolution** — Uses explicit seed from request, or generates a random one

4. **Output format** — Uses explicit format from request, or matches input format

5. **Warning collection** — Collects warnings during resolution:
   - `MissingMacKey` — HMAC requested but no MAC key provided
   - `MetadataInjectionDisabled` — `rights_metadata` is false

### Returns

`ResolvedProtectionPlan` containing:
- Effective policy and DMI value
- Normalized rights notice
- Protection channels and processing options
- Seed and intensity
- Input/output formats
- Legal metadata and MAC key (if provided)
- Pre-computed warnings
- Resource limits

## Why Resolution Runs Once

Separating resolution from execution ensures:
- **Single validation point** — All inputs validated before processing starts
- **Immutable plan** — No mid-flight mutations during pipeline execution
- **Clean separation** — Request construction is decoupled from image processing
- **Testable** — Resolution can be tested independently of image processing

## Entry Points Using Resolution

- `process_request_bytes()` — Resolves then processes
- `process_request_bytes_with_warnings()` — Resolves, processes, and collects runtime warnings
- `process_request_bytes_with_report()` — Full execution report with resource usage
