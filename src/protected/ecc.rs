const DATA_LEN: usize = 24;
pub(crate) const REPLICATION_FACTOR: usize = 3;
pub(crate) const TOTAL_ECC_LEN: usize = DATA_LEN * REPLICATION_FACTOR;

pub(crate) fn ecc_encode(data: &[u8]) -> Vec<u8> {
    let data_len = data.len();
    let mut encoded = Vec::with_capacity(data_len * REPLICATION_FACTOR);
    for _ in 0..REPLICATION_FACTOR {
        encoded.extend_from_slice(data);
    }
    encoded
}

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
