use crate::payload_v3::errors::PayloadV3ParseError;
#[cfg(feature = "signatures")]
use crate::payload_v3::types::ExtensionType;
use crate::payload_v3::types::{
    AuthAlgorithm, ExtensionEntry, PayloadFlags, ProtectionChannels, V3_CORE_SIZE, V3_MAGIC,
    V3_MAX_EMBEDDED_SIZE, V3_MAX_EXTENSION_COUNT, V3_MAX_EXTENSION_SIZE, V3_MAX_KEY_ID_LEN,
    V3_PAYLOAD_VERSION,
};

/// Builder for constructing a V3 payload.
///
/// # Wire Format
///
/// The payload consists of:
/// - 32-byte core header
/// - 0–32 byte key identifier
/// - 0–128 byte TLV extension section
/// - Variable-length authentication tag
///
/// Total size must not exceed [`V3_MAX_EMBEDDED_SIZE`] (256 bytes).
///
/// # Example
///
/// ```rust
/// use stegoeggo::payload_v3::writer::PayloadBuilder;
/// use stegoeggo::payload_v3::{
///     AuthAlgorithm, ExtensionEntry, PayloadFlags, ProtectionChannels,
/// };
///
/// let payload = PayloadBuilder::new()
///     .seed(42)
///     .intensity(5000)
///     .dmi_policy(2)
///     .channels(ProtectionChannels {
///         rights_metadata: true,
///         hidden_marker: true,
///         authentication: true,
///     })
///     .auth_algorithm(AuthAlgorithm::Crc32)
///     .build()
///     .unwrap();
///
/// assert!(payload.len() <= 256);
/// assert_eq!(&payload[0..2], &[0x53, 0x45]); // magic
/// assert_eq!(payload[2], 3); // version
/// ```
pub struct PayloadBuilder {
    seed: u64,
    intensity: u16,
    dmi_policy: u8,
    channels: ProtectionChannels,
    flags: PayloadFlags,
    key_id: Vec<u8>,
    auth_algorithm: AuthAlgorithm,
    auth_tag: Vec<u8>,
    content_hash: [u8; 8],
    extensions: Vec<ExtensionEntry>,
}

impl PayloadBuilder {
    /// Create a new builder with default values.
    #[must_use]
    pub fn new() -> Self {
        Self {
            seed: 0,
            intensity: 0,
            dmi_policy: 0,
            channels: ProtectionChannels {
                rights_metadata: false,
                hidden_marker: false,
                authentication: false,
            },
            flags: PayloadFlags {
                has_extensions: false,
                has_key_id: false,
                tiled: false,
                progressive_jpeg: false,
                critical_extension: false,
                signed: false,
                reserved: 0,
            },
            key_id: Vec::new(),
            auth_algorithm: AuthAlgorithm::None,
            auth_tag: Vec::new(),
            content_hash: [0u8; 8],
            extensions: Vec::new(),
        }
    }

    /// Set the PRNG seed for steganographic embedding.
    #[must_use]
    pub fn seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }

    /// Set the embedding intensity (0–10000, where 10000 = 100.0%).
    #[must_use]
    pub fn intensity(mut self, intensity: u16) -> Self {
        self.intensity = intensity;
        self
    }

    /// Set the DMI rights-policy discriminant.
    #[must_use]
    pub fn dmi_policy(mut self, dmi_policy: u8) -> Self {
        self.dmi_policy = dmi_policy;
        self
    }

    /// Set the protection channel bitmask.
    #[must_use]
    pub fn channels(mut self, channels: ProtectionChannels) -> Self {
        self.channels = channels;
        self
    }

    /// Set the key identifier.
    ///
    /// # Panics
    ///
    /// Panics if `key_id` exceeds 32 bytes.
    #[must_use]
    pub fn key_id(mut self, key_id: Vec<u8>) -> Self {
        assert!(
            key_id.len() <= V3_MAX_KEY_ID_LEN,
            "Key ID exceeds maximum length"
        );
        self.key_id = key_id;
        self
    }

    /// Set the authentication algorithm.
    #[must_use]
    pub fn auth_algorithm(mut self, auth_algorithm: AuthAlgorithm) -> Self {
        self.auth_algorithm = auth_algorithm;
        self
    }

    /// Set the authentication tag or signature bytes.
    #[must_use]
    pub fn auth_tag(mut self, auth_tag: Vec<u8>) -> Self {
        self.auth_tag = auth_tag;
        self
    }

    /// Set the truncated content hash (8 bytes).
    #[must_use]
    pub fn content_hash(mut self, content_hash: [u8; 8]) -> Self {
        self.content_hash = content_hash;
        self
    }

    /// Add a TLV extension entry.
    #[must_use]
    pub fn extension(mut self, entry: ExtensionEntry) -> Self {
        self.extensions.push(entry);
        self
    }

    /// Set multiple TLV extension entries.
    #[must_use]
    pub fn extensions(mut self, extensions: Vec<ExtensionEntry>) -> Self {
        self.extensions = extensions;
        self
    }

    /// Mark the payload as using tiled steganography.
    #[must_use]
    pub fn tiled(mut self, tiled: bool) -> Self {
        self.flags.tiled = tiled;
        self
    }

    /// Mark the payload as coming from a progressive JPEG source.
    #[must_use]
    pub fn progressive_jpeg(mut self, progressive: bool) -> Self {
        self.flags.progressive_jpeg = progressive;
        self
    }

    /// Mark the payload as having a critical extension.
    #[must_use]
    pub fn critical_extension(mut self, critical: bool) -> Self {
        self.flags.critical_extension = critical;
        self
    }

    /// Mark the payload as signed (Ed25519 signature).
    #[must_use]
    pub fn signed(mut self, signed: bool) -> Self {
        self.flags.signed = signed;
        self
    }

    /// Sign claim bytes and embed the Ed25519 signature in this payload.
    ///
    /// Adds the public key as an `Ed25519PublicKey` extension (0x0010),
    /// the signature as an `Ed25519DetachedSig` extension (0x0011),
    /// sets the auth algorithm to `Ed25519`, sets the auth tag to the
    /// 64-byte signature, and marks the payload as signed.
    ///
    /// The caller should verify capacity before calling this method.
    /// Use [`check_signature_capacity`](crate::signing::check_signature_capacity)
    /// or [`SigningConfig::check_capacity`](crate::signing::SigningConfig::check_capacity)
    /// to determine if the signature fits.
    ///
    /// # Arguments
    ///
    /// * `signing_key` — The Ed25519 signing key.
    /// * `claim_bytes` — Canonical claim bytes to sign.
    ///
    /// # Returns
    ///
    /// The builder with signature fields populated.
    #[cfg(feature = "signatures")]
    #[must_use]
    pub fn embed_signature(
        mut self,
        signing_key: &crate::signing::SigningKey,
        claim_bytes: &[u8],
    ) -> Self {
        let signature = signing_key.sign(claim_bytes);
        let public_key = signing_key.public_key_bytes();
        let key_id = signing_key.key_id().to_vec();

        self.key_id = key_id;
        self.auth_algorithm = AuthAlgorithm::Ed25519;
        self.auth_tag = signature.clone();
        self.flags.signed = true;

        self.extensions.push(ExtensionEntry {
            extension_type: ExtensionType::Ed25519PublicKey as u16,
            critical: false,
            data: public_key.to_vec(),
        });

        self.extensions.push(ExtensionEntry {
            extension_type: ExtensionType::Ed25519DetachedSig as u16,
            critical: true,
            data: signature,
        });

        self
    }

    /// Compute the serialized extension section size in bytes.
    fn extension_size(&self) -> usize {
        if self.extensions.is_empty() {
            return 0;
        }
        let mut size = 0;
        for ext in &self.extensions {
            size += 4 + ext.data.len();
        }
        size
    }

    /// Compute the header length (core + key_id + extensions).
    fn header_length(&self) -> usize {
        V3_CORE_SIZE + self.key_id.len() + self.extension_size()
    }

    /// Build the payload bytes.
    ///
    /// # Errors
    ///
    /// Returns [`PayloadV3ParseError`] if the payload exceeds size limits
    /// or contains invalid field combinations.
    pub fn build(self) -> Result<Vec<u8>, PayloadV3ParseError> {
        if self.extensions.len() > V3_MAX_EXTENSION_COUNT {
            return Err(PayloadV3ParseError::ExtensionsTooLarge);
        }

        let ext_size = self.extension_size();
        if ext_size > V3_MAX_EXTENSION_SIZE {
            return Err(PayloadV3ParseError::ExtensionsTooLarge);
        }

        let header_length = self.header_length();
        if header_length > 255 {
            return Err(PayloadV3ParseError::Oversized {
                size: header_length,
                max: 255,
            });
        }

        let expected_tag_len = self.auth_algorithm.tag_length().unwrap_or(0);
        if !self.auth_tag.is_empty() && self.auth_tag.len() != expected_tag_len {
            return Err(PayloadV3ParseError::CorruptTag);
        }

        let total_length = header_length + self.auth_tag.len();
        if total_length > V3_MAX_EMBEDDED_SIZE {
            return Err(PayloadV3ParseError::Oversized {
                size: total_length,
                max: V3_MAX_EMBEDDED_SIZE,
            });
        }

        let mut flags = self.flags;
        flags.has_key_id = !self.key_id.is_empty();
        flags.has_extensions = !self.extensions.is_empty();

        let mut buf = Vec::with_capacity(total_length);

        // Core header (32 bytes)
        buf.extend_from_slice(&V3_MAGIC);
        buf.push(V3_PAYLOAD_VERSION);
        buf.push(header_length as u8);
        buf.extend_from_slice(&(total_length as u16).to_le_bytes());
        buf.extend_from_slice(&flags.to_bits().to_le_bytes());
        buf.extend_from_slice(&self.channels.to_bits().to_le_bytes());
        buf.push(self.dmi_policy);
        buf.extend_from_slice(&self.seed.to_le_bytes());
        buf.extend_from_slice(&self.intensity.to_le_bytes());
        buf.extend_from_slice(&self.content_hash);
        buf.push(self.auth_algorithm as u8);
        buf.push(expected_tag_len as u8);
        buf.push(self.key_id.len() as u8);

        debug_assert_eq!(buf.len(), V3_CORE_SIZE);

        // Key ID
        buf.extend_from_slice(&self.key_id);

        // Extensions (TLV)
        for ext in &self.extensions {
            buf.extend_from_slice(&ext.extension_type.to_le_bytes());
            buf.extend_from_slice(&(ext.data.len() as u16).to_le_bytes());
            buf.extend_from_slice(&ext.data);
        }

        // Authentication tag
        buf.extend_from_slice(&self.auth_tag);

        debug_assert_eq!(buf.len(), total_length);

        Ok(buf)
    }
}

impl Default for PayloadBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_minimal_payload() {
        let payload = PayloadBuilder::new().build().unwrap();
        assert_eq!(payload.len(), V3_CORE_SIZE);
        assert_eq!(&payload[0..2], &V3_MAGIC);
        assert_eq!(payload[2], V3_PAYLOAD_VERSION);
        assert_eq!(payload[3], V3_CORE_SIZE as u8);
    }

    #[test]
    fn test_build_with_key_id() {
        let payload = PayloadBuilder::new()
            .key_id(vec![0xAA; 16])
            .seed(42)
            .build()
            .unwrap();
        assert_eq!(payload.len(), V3_CORE_SIZE + 16);
        assert_eq!(payload[31], 16); // key_id_len
        assert_eq!(&payload[V3_CORE_SIZE..V3_CORE_SIZE + 16], &[0xAA; 16]);
    }

    #[test]
    fn test_build_with_extension() {
        let ext = ExtensionEntry {
            extension_type: 0x0001,
            critical: false,
            data: vec![0xBB; 4],
        };
        let payload = PayloadBuilder::new().extension(ext).build().unwrap();
        assert_eq!(payload.len(), V3_CORE_SIZE + 8); // 4-byte header + 4-byte data
    }

    #[test]
    fn test_build_with_crc32() {
        let payload = PayloadBuilder::new()
            .auth_algorithm(AuthAlgorithm::Crc32)
            .auth_tag(vec![0xCC; 4])
            .build()
            .unwrap();
        assert_eq!(payload.len(), V3_CORE_SIZE + 4);
    }

    #[test]
    fn test_build_with_hmac() {
        let payload = PayloadBuilder::new()
            .auth_algorithm(AuthAlgorithm::HmacSha256Truncated)
            .auth_tag(vec![0xDD; 16])
            .build()
            .unwrap();
        assert_eq!(payload.len(), V3_CORE_SIZE + 16);
    }

    #[test]
    fn test_roundtrip_through_parser() {
        use crate::payload_v3::parser::parse_payload;

        let payload = PayloadBuilder::new()
            .seed(42)
            .intensity(5000)
            .dmi_policy(2)
            .channels(ProtectionChannels {
                rights_metadata: true,
                hidden_marker: true,
                authentication: false,
            })
            .key_id(vec![0xAA; 8])
            .build()
            .unwrap();

        let parsed = parse_payload(&payload).unwrap();
        match parsed {
            crate::payload_v3::parser::ParsedPayload::V3(v3) => {
                assert_eq!(v3.header.seed, 42);
                assert_eq!(v3.header.intensity, 5000);
                assert_eq!(v3.header.dmi_policy, 2);
                assert_eq!(v3.key_id, vec![0xAA; 8]);
            }
            _ => panic!("Expected V3"),
        }
    }

    #[test]
    fn test_build_rejects_mismatched_tag_length() {
        let result = PayloadBuilder::new()
            .auth_algorithm(AuthAlgorithm::HmacSha256Truncated)
            .auth_tag(vec![0xDD; 8]) // wrong: expects 16 bytes
            .build();
        assert!(matches!(result, Err(PayloadV3ParseError::CorruptTag)));
    }

    #[test]
    fn test_build_rejects_extension_too_large() {
        let result = PayloadBuilder::new()
            .extensions(vec![ExtensionEntry {
                extension_type: 0x0001,
                critical: false,
                data: vec![0u8; 129],
            }])
            .build();
        assert!(matches!(
            result,
            Err(PayloadV3ParseError::ExtensionsTooLarge)
        ));
    }

    #[test]
    fn test_default_flags() {
        let payload = PayloadBuilder::new().build().unwrap();
        let flags = u16::from_le_bytes([payload[6], payload[7]]);
        // No flags should be set for empty payload
        assert_eq!(flags & 0x0003, 0);
    }

    #[test]
    fn test_flags_tiled() {
        let payload = PayloadBuilder::new().tiled(true).build().unwrap();
        let flags = u16::from_le_bytes([payload[6], payload[7]]);
        assert!(flags & PayloadFlags::TILED != 0);
    }

    #[test]
    fn test_intensity_float_roundtrip() {
        let payload = PayloadBuilder::new().intensity(5000).build().unwrap();
        let parsed = crate::payload_v3::header::PayloadV3Header::from_bytes(&payload).unwrap();
        assert!((parsed.intensity_f32() - 50.0).abs() < f32::EPSILON);
    }

    #[cfg(feature = "signatures")]
    #[test]
    fn test_embed_signature_roundtrip() {
        use crate::payload_v3::parser::parse_payload;
        use crate::signing::SigningKey;

        let sk = SigningKey::from_bytes([42u8; 32], b"test-key".to_vec());
        let claim = b"test claim data for signing";

        let payload = PayloadBuilder::new()
            .seed(99)
            .intensity(7500)
            .dmi_policy(3)
            .channels(ProtectionChannels {
                rights_metadata: true,
                hidden_marker: true,
                authentication: true,
            })
            .embed_signature(&sk, claim)
            .build()
            .unwrap();

        assert!(payload.len() <= V3_MAX_EMBEDDED_SIZE);

        let parsed = parse_payload(&payload).unwrap();
        match parsed {
            crate::payload_v3::parser::ParsedPayload::V3(v3) => {
                assert_eq!(v3.header.seed, 99);
                assert_eq!(v3.header.intensity, 7500);
                assert_eq!(v3.header.auth_algorithm, AuthAlgorithm::Ed25519 as u8);
                assert_eq!(v3.header.auth_tag_len, 64);
                assert!(v3.header.flags & PayloadFlags::SIGNED != 0);
                assert_eq!(v3.key_id, b"test-key");

                let pk_ext = v3.extensions.iter().find(|e| e.extension_type == 0x0010);
                assert!(pk_ext.is_some());
                assert_eq!(pk_ext.unwrap().data.len(), 32);

                let sig_ext = v3.extensions.iter().find(|e| e.extension_type == 0x0011);
                assert!(sig_ext.is_some());
                assert_eq!(sig_ext.unwrap().data.len(), 64);
                assert!(sig_ext.unwrap().critical);
            }
            _ => panic!("Expected V3"),
        }
    }

    #[cfg(feature = "signatures")]
    #[test]
    fn test_embed_signature_verifies() {
        use crate::signing::SigningKey;

        let sk = SigningKey::from_bytes([7u8; 32], b"verify-key".to_vec());
        let vk = sk.verifying_key();
        let claim = b"claim to verify";

        let payload = PayloadBuilder::new()
            .embed_signature(&sk, claim)
            .build()
            .unwrap();

        let parsed = crate::payload_v3::parser::parse_payload(&payload).unwrap();
        match parsed {
            crate::payload_v3::parser::ParsedPayload::V3(v3) => {
                let sig_ext = v3
                    .extensions
                    .iter()
                    .find(|e| e.extension_type == 0x0011)
                    .unwrap();
                let result = vk.verify(claim, &sig_ext.data);
                assert_eq!(result, crate::signing::SignatureResult::Valid);
            }
            _ => panic!("Expected V3"),
        }
    }

    #[cfg(feature = "signatures")]
    #[test]
    fn test_embed_signature_rejects_altered_claim() {
        use crate::signing::SigningKey;

        let sk = SigningKey::from_bytes([9u8; 32], b"alter-key".to_vec());
        let vk = sk.verifying_key();

        let payload = PayloadBuilder::new()
            .embed_signature(&sk, b"original claim")
            .build()
            .unwrap();

        let parsed = crate::payload_v3::parser::parse_payload(&payload).unwrap();
        match parsed {
            crate::payload_v3::parser::ParsedPayload::V3(v3) => {
                let sig_ext = v3
                    .extensions
                    .iter()
                    .find(|e| e.extension_type == 0x0011)
                    .unwrap();
                let result = vk.verify(b"altered claim", &sig_ext.data);
                assert_eq!(result, crate::signing::SignatureResult::Invalid);
            }
            _ => panic!("Expected V3"),
        }
    }
}
