use serde::{Deserialize, Serialize};

/// Magic bytes identifying a V3 payload (`"SE"`).
pub const V3_MAGIC: [u8; 2] = [0x53, 0x45];

/// Current payload format version.
pub const V3_PAYLOAD_VERSION: u8 = 3;

/// Maximum total embedded payload size in bytes.
pub const V3_MAX_EMBEDDED_SIZE: usize = 256;

/// Maximum size of the extension section in bytes.
pub const V3_MAX_EXTENSION_SIZE: usize = 128;

/// Maximum key identifier length in bytes.
pub const V3_MAX_KEY_ID_LEN: usize = 32;

/// Domain separation string for v3 payloads.
pub const V3_DOMAIN_STRING: &[u8] = b"StegoEggo-v3";

/// Core header size in bytes (excluding key ID and extensions).
pub const V3_CORE_SIZE: usize = 32;

/// Maximum number of extension entries allowed.
pub const V3_MAX_EXTENSION_COUNT: usize = 32;

/// Sentinel value marking the end of the extension section.
pub const END_OF_EXTENSIONS: u16 = 0xFFFF;

/// Authentication algorithm identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum AuthAlgorithm {
    /// No authentication.
    None = 0,
    /// CRC32 checksum.
    Crc32 = 1,
    /// Truncated HMAC-SHA256.
    HmacSha256Truncated = 2,
    /// Ed25519 signature.
    Ed25519 = 3,
}

impl AuthAlgorithm {
    /// Parse an algorithm from its byte discriminant.
    #[must_use]
    pub fn from_byte(b: u8) -> Option<Self> {
        match b {
            0 => Some(Self::None),
            1 => Some(Self::Crc32),
            2 => Some(Self::HmacSha256Truncated),
            3 => Some(Self::Ed25519),
            _ => None,
        }
    }

    /// Returns the authentication tag length in bytes, if applicable.
    #[must_use]
    pub fn tag_length(self) -> Option<usize> {
        match self {
            Self::None => Some(0),
            Self::Crc32 => Some(4),
            Self::HmacSha256Truncated => Some(8),
            Self::Ed25519 => Some(64),
        }
    }
}

/// Protection channel bitfield for the V3 header.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtectionChannels {
    /// Rights metadata channel enabled.
    pub rights_metadata: bool,
    /// Hidden steganographic marker channel enabled.
    pub hidden_marker: bool,
    /// Authentication channel enabled.
    pub authentication: bool,
}

impl ProtectionChannels {
    /// Encode channels as a 16-bit bitfield.
    #[must_use]
    pub fn to_bits(self) -> u16 {
        let mut bits = 0u16;
        if self.rights_metadata {
            bits |= 1;
        }
        if self.hidden_marker {
            bits |= 1 << 1;
        }
        if self.authentication {
            bits |= 1 << 2;
        }
        bits
    }

    /// Decode channels from a 16-bit bitfield. Returns `None` if reserved bits are set.
    #[must_use]
    pub fn from_bits(bits: u16) -> Option<Self> {
        if bits & !0x0007 != 0 {
            return None;
        }
        Some(Self {
            rights_metadata: bits & 1 != 0,
            hidden_marker: bits & (1 << 1) != 0,
            authentication: bits & (1 << 2) != 0,
        })
    }
}

/// A single TLV extension entry in a V3 payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtensionEntry {
    /// Extension type discriminant.
    pub extension_type: u16,
    /// Whether this extension is critical (must be understood by parser).
    pub critical: bool,
    /// Raw extension data.
    pub data: Vec<u8>,
}

/// V3 payload header flags bitfield.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PayloadFlags {
    /// Whether extensions are present.
    pub has_extensions: bool,
    /// Whether a key ID is present.
    pub has_key_id: bool,
    /// Whether tiled steganography is used.
    pub tiled: bool,
    /// Whether the source image is a progressive JPEG.
    pub progressive_jpeg: bool,
    /// Whether a critical extension is present.
    pub critical_extension: bool,
    /// Whether the payload is signed.
    pub signed: bool,
    /// Reserved bits (must be zero for forward compatibility).
    pub reserved: u16,
}

impl PayloadFlags {
    /// Bitmask for the `has_extensions` flag.
    pub const HAS_EXTENSIONS: u16 = 0x0001;
    /// Bitmask for the `has_key_id` flag.
    pub const HAS_KEY_ID: u16 = 0x0002;
    /// Bitmask for the `tiled` flag.
    pub const TILED: u16 = 0x0004;
    /// Bitmask for the `progressive_jpeg` flag.
    pub const PROGRESSIVE_JPEG: u16 = 0x0008;
    /// Bitmask for the `critical_extension` flag.
    pub const CRITICAL_EXTENSION: u16 = 0x0100;
    /// Bitmask for the `signed` flag.
    pub const SIGNED: u16 = 0x0200;

    /// Encode flags as a 16-bit bitfield.
    #[must_use]
    pub fn to_bits(self) -> u16 {
        let mut bits = self.reserved & 0xF0F0;
        if self.has_extensions {
            bits |= Self::HAS_EXTENSIONS;
        }
        if self.has_key_id {
            bits |= Self::HAS_KEY_ID;
        }
        if self.tiled {
            bits |= Self::TILED;
        }
        if self.progressive_jpeg {
            bits |= Self::PROGRESSIVE_JPEG;
        }
        if self.critical_extension {
            bits |= Self::CRITICAL_EXTENSION;
        }
        if self.signed {
            bits |= Self::SIGNED;
        }
        bits
    }

    /// Decode flags from a 16-bit bitfield.
    #[must_use]
    pub fn from_bits(bits: u16) -> Self {
        Self {
            has_extensions: bits & Self::HAS_EXTENSIONS != 0,
            has_key_id: bits & Self::HAS_KEY_ID != 0,
            tiled: bits & Self::TILED != 0,
            progressive_jpeg: bits & Self::PROGRESSIVE_JPEG != 0,
            critical_extension: bits & Self::CRITICAL_EXTENSION != 0,
            signed: bits & Self::SIGNED != 0,
            reserved: bits & 0xF0F0,
        }
    }
}

/// Known extension type discriminants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum ExtensionType {
    /// Timestamp of processing.
    Timestamp = 0x0001,
    /// Creator fingerprint.
    CreatorFingerprint = 0x0002,
    /// Unique instance identifier.
    InstanceId = 0x0003,
    /// Hash of the legal notice text.
    LegalNoticeHash = 0x0004,
    /// Software version string.
    VersionString = 0x0005,
    /// Processing history trail.
    ProcessingHistory = 0x0006,
    /// Ed25519 public key for detached signatures.
    Ed25519PublicKey = 0x0010,
    /// Ed25519 detached signature.
    Ed25519DetachedSig = 0x0011,
    /// Vendor-specific extension prefix.
    VendorPrefix = 0x00FF,
}

impl ExtensionType {
    /// Parse an extension type from a `u16` discriminant.
    #[must_use]
    pub fn from_u16(v: u16) -> Option<Self> {
        match v {
            0x0001 => Some(Self::Timestamp),
            0x0002 => Some(Self::CreatorFingerprint),
            0x0003 => Some(Self::InstanceId),
            0x0004 => Some(Self::LegalNoticeHash),
            0x0005 => Some(Self::VersionString),
            0x0006 => Some(Self::ProcessingHistory),
            0x0010 => Some(Self::Ed25519PublicKey),
            0x0011 => Some(Self::Ed25519DetachedSig),
            0x00FF => Some(Self::VendorPrefix),
            _ => None,
        }
    }

    /// Returns `true` if the given extension type discriminant is critical.
    #[must_use]
    pub fn is_critical(v: u16) -> bool {
        matches!(v, 0x0011)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_algorithm_roundtrip() {
        for i in 0..=3u8 {
            let algo = AuthAlgorithm::from_byte(i).unwrap();
            assert_eq!(algo as u8, i);
        }
        assert!(AuthAlgorithm::from_byte(4).is_none());
    }

    #[test]
    fn test_protection_channels_roundtrip() {
        let channels = ProtectionChannels {
            rights_metadata: true,
            hidden_marker: true,
            authentication: false,
        };
        let bits = channels.to_bits();
        let decoded = ProtectionChannels::from_bits(bits).unwrap();
        assert_eq!(channels, decoded);
    }

    #[test]
    fn test_protection_channels_invalid_bits() {
        assert!(ProtectionChannels::from_bits(0xFFF8).is_none());
    }

    #[test]
    fn test_payload_flags_roundtrip() {
        let flags = PayloadFlags {
            has_extensions: true,
            has_key_id: false,
            tiled: true,
            progressive_jpeg: false,
            critical_extension: true,
            signed: false,
            reserved: 0,
        };
        let bits = flags.to_bits();
        let decoded = PayloadFlags::from_bits(bits);
        assert_eq!(flags, decoded);
    }

    #[test]
    fn test_extension_type_criticality() {
        assert!(ExtensionType::is_critical(0x0011));
        assert!(!ExtensionType::is_critical(0x0001));
        assert!(!ExtensionType::is_critical(0x0100));
    }
}
