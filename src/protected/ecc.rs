/// Error correction for steganographic payloads using 3x repetition with majority voting.
///
/// Each byte of the original payload is replicated 3 times. During decoding, for each
/// byte position, the most frequent value across the 3 copies is selected. This allows
/// correction of up to 1 corrupted copy per byte position (2 out of 3 must agree).
///
/// This is not true Reed-Solomon coding — it provides simpler error correction that is
/// sufficient for the steganographic use case where individual bits may be flipped by
/// image processing noise. The 3x expansion trades capacity for robustness.
const DATA_LEN: usize = 24;
/// Number of times each data byte is replicated in the ECC-encoded output.
pub(crate) const REPLICATION_FACTOR: usize = 3;
/// Total length of the ECC-encoded data (DATA_LEN × REPLICATION_FACTOR).
pub(crate) const TOTAL_ECC_LEN: usize = DATA_LEN * REPLICATION_FACTOR;

/// Encode data using 3x repetition.
///
/// Each byte is written 3 times consecutively. The output length is
/// `data.len() * REPLICATION_FACTOR`.
///
/// Kept for backward-compatible extraction of v1/v2 payloads. No longer
/// used for writing since v3 uses built-in CRC32/HMAC instead of ECC.
#[allow(dead_code)]
pub(crate) fn ecc_encode(data: &[u8]) -> Vec<u8> {
    let data_len = data.len();
    let mut encoded = Vec::with_capacity(data_len * REPLICATION_FACTOR);
    for _ in 0..REPLICATION_FACTOR {
        encoded.extend_from_slice(data);
    }
    encoded
}

/// Decode ECC-encoded data using majority voting.
///
/// For each byte position, counts votes across the 3 copies and returns the most
/// frequent value. Returns `None` if the encoded data is shorter than expected.
///
/// Corrects exactly 1 corrupted copy per byte position. If 2 or more copies are
/// corrupted to the same wrong value, the wrong value wins the vote.
pub(crate) fn ecc_decode(encoded: &[u8], data_len: usize) -> Option<Vec<u8>> {
    let expected_len = data_len * REPLICATION_FACTOR;
    if encoded.len() < expected_len {
        return None;
    }

    let mut decoded = Vec::with_capacity(data_len);
    for i in 0..data_len {
        let mut votes = [0u8; 256];
        for r in 0..REPLICATION_FACTOR {
            votes[encoded[i + r * data_len] as usize] += 1;
        }
        let best = votes.iter().enumerate().max_by_key(|(_, &count)| count)?.0;
        decoded.push(best as u8);
    }
    Some(decoded)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_produces_correct_length() {
        let data = [0x42u8; DATA_LEN];
        let encoded = ecc_encode(&data);
        assert_eq!(encoded.len(), TOTAL_ECC_LEN);
    }

    #[test]
    fn encode_decode_roundtrip_no_errors() {
        let data: Vec<u8> = (0..DATA_LEN as u8).collect();
        let encoded = ecc_encode(&data);
        let decoded = ecc_decode(&encoded, DATA_LEN).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn decode_corrects_one_error_per_byte_group() {
        let data: Vec<u8> = (0..DATA_LEN as u8).collect();
        let mut encoded = ecc_encode(&data);
        for byte in encoded.iter_mut().take(DATA_LEN) {
            *byte ^= 0xFF;
        }
        let decoded = ecc_decode(&encoded, DATA_LEN).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn decode_fails_on_too_short_input() {
        let result = ecc_decode(&[0u8; 10], DATA_LEN);
        assert!(result.is_none());
    }

    #[test]
    fn encode_output_is_triple_length() {
        let data = [1u8; DATA_LEN];
        let encoded = ecc_encode(&data);
        assert_eq!(encoded.len(), DATA_LEN * 3);
        assert_eq!(&encoded[..DATA_LEN], &data);
        assert_eq!(&encoded[DATA_LEN..DATA_LEN * 2], &data);
        assert_eq!(&encoded[DATA_LEN * 2..], &data);
    }

    #[test]
    fn different_data_different_encoded() {
        let data_a = [0x11u8; DATA_LEN];
        let data_b = [0x22u8; DATA_LEN];
        let enc_a = ecc_encode(&data_a);
        let enc_b = ecc_encode(&data_b);
        assert_ne!(enc_a, enc_b);
    }

    #[test]
    fn decode_recovers_from_single_bit_flip() {
        let data: Vec<u8> = (0..DATA_LEN as u8).map(|i| i.wrapping_mul(7)).collect();
        let mut encoded = ecc_encode(&data);
        encoded[15] ^= 0x01;
        let decoded = ecc_decode(&encoded, DATA_LEN).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn decode_recovers_from_all_parity_corrupted() {
        let data: Vec<u8> = (0..DATA_LEN as u8).collect();
        let mut encoded = ecc_encode(&data);
        let data_end = DATA_LEN;
        let parity_end = DATA_LEN * 2;
        for item in encoded.iter_mut().take(parity_end).skip(data_end) {
            *item ^= 0xFF;
        }
        let decoded = ecc_decode(&encoded, DATA_LEN).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn decode_recovers_from_mixed_errors() {
        let data: Vec<u8> = (0..DATA_LEN as u8).collect();
        let mut encoded = ecc_encode(&data);
        encoded[2] ^= 0x11;
        encoded[DATA_LEN + 5] ^= 0x22;
        encoded[DATA_LEN * 2 + 1] ^= 0x33;
        let decoded = ecc_decode(&encoded, DATA_LEN).unwrap();
        assert_eq!(decoded, data);
    }
}
