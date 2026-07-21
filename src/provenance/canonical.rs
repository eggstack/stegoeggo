use super::claim::ProvenanceClaim;

/// Canonical JSON serialization for provenance claims.
///
/// Produces deterministic JSON with sorted keys, no whitespace,
/// and null omission. This form is used for hashing and signing.
#[must_use]
pub fn canonical_json(claim: &ProvenanceClaim) -> Vec<u8> {
    claim.canonical_bytes()
}

/// Verify that canonical bytes are stable across calls.
#[must_use]
pub fn verify_canonical_stability(claim: &ProvenanceClaim) -> bool {
    let bytes1 = canonical_json(claim);
    let bytes2 = canonical_json(claim);
    bytes1 == bytes2
}
