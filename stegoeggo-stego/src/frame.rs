/// Magic bytes identifying a framed generic payload: `[0x53, 0x47]` ("SG").
pub const FRAMED_MAGIC: [u8; 2] = [0x53, 0x47];

/// Current frame version.
pub const FRAME_VERSION: u8 = 1;

/// Maximum payload size in bytes (16 MiB).
pub const MAX_FRAME_PAYLOAD: usize = 16 * 1024 * 1024;

/// Overhead of the frame header in bytes (magic + version + length + CRC32).
pub const FRAME_HEADER_SIZE: usize = 11;

/// A decoded frame header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameHeader {
    /// Frame version.
    pub version: u8,
    /// Payload byte length.
    pub payload_len: usize,
}

/// Encode arbitrary payload bytes into a framed byte sequence.
///
/// The frame format is:
///
/// ```text
/// Offset  Size  Field
/// 0       2     Magic [0x53, 0x47]
/// 2       1     Version (=1)
/// 3       4     Payload length (u32 LE)
/// 7       4     CRC32 of payload bytes
/// 11..    N     Payload bytes
/// ```
///
/// # Errors
///
/// Returns [`StegoError::InvalidConfig`] if the payload exceeds [`MAX_FRAME_PAYLOAD`].
pub fn encode(payload: &[u8]) -> Result<Vec<u8>, super::StegoError> {
    if payload.len() > MAX_FRAME_PAYLOAD {
        return Err(super::StegoError::InvalidConfig(format!(
            "payload size {} exceeds maximum {}",
            payload.len(),
            MAX_FRAME_PAYLOAD
        )));
    }

    let mut buf = Vec::with_capacity(FRAME_HEADER_SIZE + payload.len());
    buf.extend_from_slice(&FRAMED_MAGIC);
    buf.push(FRAME_VERSION);
    buf.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    buf.extend_from_slice(&crc32fast::hash(payload).to_le_bytes());
    buf.extend_from_slice(payload);
    Ok(buf)
}

/// Decode a framed byte sequence, verifying the CRC32 checksum.
///
/// Returns the payload bytes and the parsed frame header on success.
///
/// # Errors
///
/// Returns [`StegoError::FrameNotFound`] if the data is too short for a header.
/// Returns [`StegoError::MalformedFrame`] if the magic bytes don't match, the
/// version is unknown, the declared payload length exceeds bounds, or there
/// are trailing bytes after the complete frame.
/// Returns [`StegoError::FrameChecksumMismatch`] if the CRC32 doesn't match.
/// Returns [`StegoError::MalformedFrame`] if the data is shorter than header +
/// declared payload length.
pub fn decode(data: &[u8]) -> Result<(FrameHeader, Vec<u8>), super::StegoError> {
    if data.len() < FRAME_HEADER_SIZE {
        return Err(super::StegoError::FrameNotFound);
    }

    if data[0..2] != FRAMED_MAGIC {
        return Err(super::StegoError::MalformedFrame(format!(
            "expected magic {:02x?}, got {:02x?}",
            FRAMED_MAGIC,
            &data[0..2]
        )));
    }

    if data[2] != FRAME_VERSION {
        return Err(super::StegoError::MalformedFrame(format!(
            "unsupported frame version {}",
            data[2]
        )));
    }

    let payload_len = u32::from_le_bytes([data[3], data[4], data[5], data[6]]) as usize;

    if payload_len > MAX_FRAME_PAYLOAD {
        return Err(super::StegoError::MalformedFrame(format!(
            "declared payload length {payload_len} exceeds maximum {MAX_FRAME_PAYLOAD}"
        )));
    }

    let expected_len = FRAME_HEADER_SIZE + payload_len;
    if data.len() < expected_len {
        return Err(super::StegoError::MalformedFrame(format!(
            "data length {} is shorter than header ({FRAME_HEADER_SIZE}) + declared payload ({payload_len})",
            data.len()
        )));
    }

    if data.len() > expected_len {
        return Err(super::StegoError::MalformedFrame(format!(
            "data length {} exceeds expected frame size {} ({} trailing bytes)",
            data.len(),
            expected_len,
            data.len() - expected_len
        )));
    }

    let expected_crc = u32::from_le_bytes([data[7], data[8], data[9], data[10]]);
    let payload = &data[FRAME_HEADER_SIZE..expected_len];
    let actual_crc = crc32fast::hash(payload);

    if expected_crc != actual_crc {
        return Err(super::StegoError::FrameChecksumMismatch);
    }

    Ok((
        FrameHeader {
            version: FRAME_VERSION,
            payload_len,
        },
        payload.to_vec(),
    ))
}

/// Try to decode just enough of a frame header to determine the total frame
/// length, without verifying the CRC or allocating the full payload.
///
/// Returns `(frame_header, total_frame_length)` on success.
/// The `total_frame_length` is `FRAME_HEADER_SIZE + payload_len`.
///
/// This is useful for extracting the frame prefix from carrier data to
/// determine how many additional bytes to extract.
///
/// # Errors
///
/// Same as [`decode`] for header-level failures.
pub fn decode_prefix(data: &[u8]) -> Result<(FrameHeader, usize), super::StegoError> {
    if data.len() < FRAME_HEADER_SIZE {
        return Err(super::StegoError::FrameNotFound);
    }

    if data[0..2] != FRAMED_MAGIC {
        return Err(super::StegoError::MalformedFrame(format!(
            "expected magic {:02x?}, got {:02x?}",
            FRAMED_MAGIC,
            &data[0..2]
        )));
    }

    if data[2] != FRAME_VERSION {
        return Err(super::StegoError::MalformedFrame(format!(
            "unsupported frame version {}",
            data[2]
        )));
    }

    let payload_len = u32::from_le_bytes([data[3], data[4], data[5], data[6]]) as usize;

    if payload_len > MAX_FRAME_PAYLOAD {
        return Err(super::StegoError::MalformedFrame(format!(
            "declared payload length {payload_len} exceeds maximum {MAX_FRAME_PAYLOAD}"
        )));
    }

    let total = FRAME_HEADER_SIZE + payload_len;
    Ok((
        FrameHeader {
            version: FRAME_VERSION,
            payload_len,
        },
        total,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StegoError;

    #[test]
    fn encode_decode_roundtrip() {
        let payload = b"hello stegoeggo";
        let framed = encode(payload).unwrap();
        let (header, decoded) = decode(&framed).unwrap();
        assert_eq!(header.version, FRAME_VERSION);
        assert_eq!(header.payload_len, payload.len());
        assert_eq!(&decoded, payload);
    }

    #[test]
    fn encode_decode_empty_payload() {
        let framed = encode(b"").unwrap();
        let (header, decoded) = decode(&framed).unwrap();
        assert_eq!(header.payload_len, 0);
        assert!(decoded.is_empty());
    }

    #[test]
    fn decode_rejects_short_data() {
        assert!(matches!(decode(&[0x53]), Err(StegoError::FrameNotFound)));
    }

    #[test]
    fn decode_rejects_bad_magic() {
        let mut data = vec![0u8; FRAME_HEADER_SIZE + 10];
        data[0] = 0xAA;
        data[1] = 0xBB;
        assert!(matches!(decode(&data), Err(StegoError::MalformedFrame(_))));
    }

    #[test]
    fn decode_rejects_bad_version() {
        let mut data = vec![0u8; FRAME_HEADER_SIZE + 10];
        data[0..2].copy_from_slice(&FRAMED_MAGIC);
        data[2] = 99;
        assert!(matches!(decode(&data), Err(StegoError::MalformedFrame(_))));
    }

    #[test]
    fn decode_rejects_checksum_mismatch() {
        let mut framed = encode(b"test data").unwrap();
        framed[7] ^= 0xFF;
        assert!(matches!(
            decode(&framed),
            Err(StegoError::FrameChecksumMismatch)
        ));
    }

    #[test]
    fn decode_rejects_truncated_payload() {
        let mut data = vec![0u8; FRAME_HEADER_SIZE + 5];
        data[0..2].copy_from_slice(&FRAMED_MAGIC);
        data[2] = FRAME_VERSION;
        data[3..7].copy_from_slice(&(100u32).to_le_bytes());
        assert!(matches!(decode(&data), Err(StegoError::MalformedFrame(_))));
    }

    #[test]
    fn decode_prefix_returns_total_length() {
        let payload = vec![42u8; 100];
        let framed = encode(&payload).unwrap();
        let (header, total) = decode_prefix(&framed).unwrap();
        assert_eq!(header.payload_len, 100);
        assert_eq!(total, FRAME_HEADER_SIZE + 100);
    }

    #[test]
    fn max_payload_boundary() {
        let payload = vec![0u8; MAX_FRAME_PAYLOAD];
        let framed = encode(&payload).unwrap();
        let (_, decoded) = decode(&framed).unwrap();
        assert_eq!(decoded.len(), MAX_FRAME_PAYLOAD);
    }

    #[test]
    fn reject_over_max_payload() {
        let payload = vec![0u8; MAX_FRAME_PAYLOAD + 1];
        assert!(matches!(
            encode(&payload),
            Err(StegoError::InvalidConfig(_))
        ));
    }
}
