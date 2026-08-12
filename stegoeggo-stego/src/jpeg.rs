use crate::error::{JpegUnsupportedReason, StegoError};
use crate::jpeg_transcoder::{DctStegoF5, JpegHeader, JpegTranscoder};

pub fn dct_payload_capacity(coefficients: &crate::jpeg_transcoder::Coefficients) -> usize {
    coefficients
        .values()
        .flat_map(|blocks| blocks.iter())
        .map(|block| {
            block
                .iter()
                .skip(1)
                .filter(|&&coef| coef.abs() >= 2)
                .count()
        })
        .sum()
}

pub fn embed_seed_hint_internal(
    jpeg_bytes: &[u8],
    seed: u64,
) -> std::result::Result<Vec<u8>, StegoError> {
    if !jpeg_bytes.starts_with(&[0xFF, 0xD8]) {
        return Err(StegoError::MalformedInput("Not a valid JPEG".to_string()));
    }
    let mut header =
        JpegHeader::parse(jpeg_bytes).map_err(|e| StegoError::MalformedInput(e.to_string()))?;
    DctStegoF5::new()
        .embed_seed_in_quantization_tables(&mut header, seed)
        .map_err(|e| StegoError::MalformedInput(e.to_string()))?;
    reassemble_jpeg_with_qtables(jpeg_bytes, &header)
}

pub fn extract_seed_hint_internal(
    jpeg_bytes: &[u8],
) -> std::result::Result<Option<u64>, StegoError> {
    if !jpeg_bytes.starts_with(&[0xFF, 0xD8]) {
        return Err(StegoError::MalformedInput("Not a valid JPEG".to_string()));
    }
    let header =
        JpegHeader::parse(jpeg_bytes).map_err(|e| StegoError::MalformedInput(e.to_string()))?;
    Ok(DctStegoF5::new().extract_seed_from_quantization_tables(&header))
}

pub fn reassemble_jpeg_with_qtables(
    jpeg_bytes: &[u8],
    header: &JpegHeader,
) -> std::result::Result<Vec<u8>, StegoError> {
    let mut output = Vec::with_capacity(jpeg_bytes.len() + 256);
    output.extend_from_slice(&jpeg_bytes[0..2]);

    let mut pos = 2;
    let mut wrote_tables = false;

    while pos + 4 <= jpeg_bytes.len() {
        if jpeg_bytes[pos] != 0xFF {
            pos += 1;
            continue;
        }

        let marker = jpeg_bytes[pos + 1];

        if marker == 0xDA || marker == 0xD9 {
            output.extend_from_slice(&jpeg_bytes[pos..]);
            break;
        }

        if marker == 0xDB {
            let segment_len =
                u16::from_be_bytes([jpeg_bytes[pos + 2], jpeg_bytes[pos + 3]]) as usize;
            pos += 2 + segment_len;

            if !wrote_tables {
                for table in header.quantization_tables.iter().flatten() {
                    output.push(0xFF);
                    output.push(0xDB);
                    let table_data_len = if table.precision == 16 { 129 } else { 65 };
                    let total_len = table_data_len + 2;
                    output.extend_from_slice(&(total_len as u16).to_be_bytes());
                    let precision_bit = if table.precision == 16 { 1 } else { 0 };
                    output.push((precision_bit << 4) | table.table_id);
                    if table.precision == 8 {
                        for &val in &table.values {
                            output.push(val as u8);
                        }
                    } else {
                        for &val in &table.values {
                            output.extend_from_slice(&val.to_be_bytes());
                        }
                    }
                }
                wrote_tables = true;
            }
            continue;
        }

        if marker == 0x00 {
            pos += 1;
            continue;
        }

        let segment_len = u16::from_be_bytes([jpeg_bytes[pos + 2], jpeg_bytes[pos + 3]]) as usize;
        if pos + 2 + segment_len > jpeg_bytes.len() {
            return Err(StegoError::MalformedInput(
                "Malformed JPEG segment length exceeds buffer".into(),
            ));
        }
        output.extend_from_slice(&jpeg_bytes[pos..pos + 2 + segment_len]);
        pos += 2 + segment_len;
    }

    Ok(output)
}

/// Configuration for JPEG DCT carrier operations.
///
/// Controls the seed and redundancy for F5-style DCT coefficient
/// embedding and extraction.
///
/// # Examples
///
/// ```rust
/// use stegoeggo_stego::jpeg::JpegConfig;
///
/// let config = JpegConfig::new(42);
/// assert_eq!(config.seed(), 42);
/// assert_eq!(config.redundancy(), 3);
/// ```
#[derive(Debug, Clone)]
pub struct JpegConfig {
    seed: u64,
    redundancy: usize,
}

impl JpegConfig {
    /// Create a new configuration with the given seed and default redundancy (3).
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self {
            seed,
            redundancy: 3,
        }
    }

    /// Set the redundancy level (1–10). Higher redundancy increases
    /// robustness at the cost of reduced capacity.
    ///
    /// # Panics
    ///
    /// Panics if `redundancy` is 0 or greater than 10.
    #[must_use]
    pub fn with_redundancy(mut self, redundancy: usize) -> Self {
        assert!(
            (1..=10).contains(&redundancy),
            "redundancy must be 1..=10, got {redundancy}"
        );
        self.redundancy = redundancy;
        self
    }

    /// The seed used for DCT coefficient selection.
    #[must_use]
    pub fn seed(&self) -> u64 {
        self.seed
    }

    /// The redundancy level.
    #[must_use]
    pub fn redundancy(&self) -> usize {
        self.redundancy
    }
}

/// Classification of JPEG DCT support for a given JPEG input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JpegSupport {
    /// DCT embedding is supported.
    Supported,
    /// DCT embedding is not supported for the given reason.
    Unsupported(JpegUnsupportedReason),
}

/// Probe whether a JPEG is supported for DCT embedding.
///
/// Checks header properties, scan structure, and entropy coding to
/// determine if F5-style DCT coefficient embedding is possible.
///
/// # Errors
///
/// Returns [`StegoError::MalformedInput`] if the JPEG header cannot be parsed.
///
/// # Examples
///
/// ```rust,no_run
/// let jpeg_bytes = std::fs::read("photo.jpg").unwrap();
/// let support = stegoeggo_stego::jpeg::probe_support(&jpeg_bytes).unwrap();
/// match support {
///     stegoeggo_stego::jpeg::JpegSupport::Supported => println!("DCT embedding available"),
///     stegoeggo_stego::jpeg::JpegSupport::Unsupported(reason) => {
///         println!("DCT not supported: {reason}")
///     }
/// }
/// ```
pub fn probe_support(jpeg_bytes: &[u8]) -> std::result::Result<JpegSupport, StegoError> {
    if !jpeg_bytes.starts_with(&[0xFF, 0xD8]) {
        return Err(StegoError::MalformedInput("not a valid JPEG".to_string()));
    }

    let header =
        JpegHeader::parse(jpeg_bytes).map_err(|e| StegoError::MalformedInput(e.to_string()))?;

    let support = crate::jpeg_transcoder::probe_dct_support_full(&header, jpeg_bytes);

    match support {
        crate::jpeg_transcoder::DctSupport::Supported => Ok(JpegSupport::Supported),
        crate::jpeg_transcoder::DctSupport::Unsupported(reason) => {
            Ok(JpegSupport::Unsupported(map_unsupported_reason(reason)))
        }
    }
}

fn map_unsupported_reason(
    r: crate::jpeg_transcoder::DctUnsupportedReason,
) -> JpegUnsupportedReason {
    match r {
        crate::jpeg_transcoder::DctUnsupportedReason::Progressive => {
            JpegUnsupportedReason::Progressive
        }
        crate::jpeg_transcoder::DctUnsupportedReason::MultipleScans => {
            JpegUnsupportedReason::MultipleScans
        }
        crate::jpeg_transcoder::DctUnsupportedReason::ArithmeticCoding => {
            JpegUnsupportedReason::ArithmeticCoding
        }
        crate::jpeg_transcoder::DctUnsupportedReason::LosslessCoding => {
            JpegUnsupportedReason::LosslessCoding
        }
        crate::jpeg_transcoder::DctUnsupportedReason::UnsupportedPrecision => {
            JpegUnsupportedReason::UnsupportedPrecision
        }
        crate::jpeg_transcoder::DctUnsupportedReason::RestartIntervals => {
            JpegUnsupportedReason::RestartIntervals
        }
        crate::jpeg_transcoder::DctUnsupportedReason::UnsupportedColorProcess => {
            JpegUnsupportedReason::UnsupportedColorProcess
        }
        crate::jpeg_transcoder::DctUnsupportedReason::MissingTables => {
            JpegUnsupportedReason::MissingTables
        }
        crate::jpeg_transcoder::DctUnsupportedReason::MalformedHeader => {
            JpegUnsupportedReason::MalformedHeader
        }
        crate::jpeg_transcoder::DctUnsupportedReason::TrailingSegmentsAfterScan => {
            JpegUnsupportedReason::TrailingSegmentsAfterScan
        }
    }
}

/// Query the available DCT capacity for a JPEG.
///
/// Returns a [`CapacityReport`](super::CapacityReport) indicating how many
/// payload bytes can be embedded with the given configuration. Capacity is
/// measured in non-zero AC coefficients (available) and the required number
/// for the given payload at the configured redundancy (required).
///
/// # Arguments
///
/// * `jpeg_bytes` — Encoded JPEG data.
/// * `payload_len` — Desired payload length in bytes.
/// * `config` — JPEG DCT carrier configuration.
///
/// # Errors
///
/// Returns [`StegoError::MalformedInput`] if the JPEG cannot be parsed.
/// Returns [`StegoError::UnsupportedJpeg`] if DCT embedding is not possible.
///
/// # Examples
///
/// ```rust,no_run
/// let jpeg_bytes = std::fs::read("photo.jpg").unwrap();
/// let config = stegoeggo_stego::jpeg::JpegConfig::new(42);
/// let report = stegoeggo_stego::jpeg::capacity(&jpeg_bytes, 100, &config).unwrap();
/// assert!(report.is_sufficient());
/// ```
pub fn capacity(
    jpeg_bytes: &[u8],
    payload_len: usize,
    config: &JpegConfig,
) -> std::result::Result<super::CapacityReport, StegoError> {
    let support = probe_support(jpeg_bytes)?;
    if let JpegSupport::Unsupported(reason) = support {
        return Err(StegoError::UnsupportedJpeg(reason));
    }

    let (_, coefficients) = JpegTranscoder::decode_coefficients(jpeg_bytes)
        .map_err(|e| StegoError::MalformedInput(e.to_string()))?;

    let available = dct_payload_capacity(&coefficients);
    let payload_bits = payload_len * 8;
    let required = payload_bits * config.redundancy();

    Ok(super::CapacityReport {
        required,
        available,
    })
}

/// Embed arbitrary bytes into a JPEG using F5-style DCT coefficient
/// modification.
///
/// Returns an [`EmbedReport`](super::EmbedReport) with the output JPEG
/// bytes and capacity information. Uses the container-preserving encoding
/// path that maintains APP2, APP13, APP14, COM, and other unrelated
/// segments byte-for-byte.
///
/// # Arguments
///
/// * `jpeg_bytes` — Encoded JPEG data (must be a supported structure).
/// * `payload` — Arbitrary bytes to embed.
/// * `config` — JPEG DCT carrier configuration.
///
/// # Errors
///
/// Returns [`StegoError::MalformedInput`] if the JPEG cannot be parsed.
/// Returns [`StegoError::UnsupportedJpeg`] if DCT embedding is not possible.
///
/// # Examples
///
/// ```rust,no_run
/// let jpeg_bytes = std::fs::read("photo.jpg").unwrap();
/// let config = stegoeggo_stego::jpeg::JpegConfig::new(42);
/// let report = stegoeggo_stego::jpeg::embed(&jpeg_bytes, b"secret", &config).unwrap();
/// assert!(report.embedded);
/// std::fs::write("output.jpg", &report.output).unwrap();
/// ```
pub fn embed(
    jpeg_bytes: &[u8],
    payload: &[u8],
    config: &JpegConfig,
) -> std::result::Result<super::EmbedReport, StegoError> {
    if !jpeg_bytes.starts_with(&[0xFF, 0xD8]) {
        return Err(StegoError::MalformedInput("not a valid JPEG".to_string()));
    }

    let support = probe_support(jpeg_bytes)?;
    if let JpegSupport::Unsupported(reason) = support {
        return Err(StegoError::UnsupportedJpeg(reason));
    }

    let (header, coefficients) = JpegTranscoder::decode_coefficients(jpeg_bytes)
        .map_err(|e| StegoError::MalformedInput(e.to_string()))?;

    let available = dct_payload_capacity(&coefficients);
    let payload_bits = payload.len() * 8;
    let required = payload_bits * config.redundancy();

    let max_feasible = available.checked_div(payload_bits).unwrap_or(0);
    let selected_redundancy = config.redundancy().min(max_feasible).max(1);

    if max_feasible >= 1 && !payload.is_empty() {
        let mut embedded_coefficients = coefficients.clone();
        if DctStegoF5::with_redundancy(selected_redundancy)
            .embed_f5(&mut embedded_coefficients, payload, config.seed())
            .is_ok()
        {
            let mut header = header;
            DctStegoF5::new()
                .embed_seed_in_quantization_tables(&mut header, config.seed())
                .map_err(|e| StegoError::MalformedInput(e.to_string()))?;

            let output = JpegTranscoder::encode_coefficients(
                &header,
                &embedded_coefficients,
                Some(jpeg_bytes),
            )
            .map_err(|e| StegoError::MalformedInput(e.to_string()))?;

            return Ok(super::EmbedReport {
                embedded: true,
                output,
                payload_bytes: payload.len(),
                required_capacity: required,
                available_capacity: available,
                actual_redundancy: selected_redundancy,
            });
        }
    }

    let mut header = header;
    DctStegoF5::new()
        .embed_seed_in_quantization_tables(&mut header, config.seed())
        .map_err(|e| StegoError::MalformedInput(e.to_string()))?;

    let output = JpegTranscoder::encode_coefficients(&header, &coefficients, Some(jpeg_bytes))
        .map_err(|e| StegoError::MalformedInput(e.to_string()))?;

    Ok(super::EmbedReport {
        embedded: false,
        output,
        payload_bytes: payload.len(),
        required_capacity: required,
        available_capacity: available,
        actual_redundancy: config.redundancy(),
    })
}

/// Extract arbitrary bytes from a JPEG using F5-style DCT coefficient
/// extraction.
///
/// # Arguments
///
/// * `jpeg_bytes` — Encoded JPEG data.
/// * `payload_len` — Expected payload length in bytes.
/// * `config` — JPEG DCT carrier configuration.
/// * `actual_redundancy` — The actual redundancy level used during embedding.
///   Pass `config.redundancy()` if embedding used the requested redundancy,
///   or the `actual_redundancy` from the [`EmbedReport`] if embedding
///   auto-downgraded redundancy due to capacity constraints.
///
/// # Errors
///
/// Returns [`StegoError::MalformedInput`] if the JPEG cannot be parsed or
/// extraction fails.
/// Returns [`StegoError::UnsupportedJpeg`] if DCT extraction is not possible.
///
/// # Examples
///
/// ```rust,no_run
/// let jpeg_bytes = std::fs::read("output.jpg").unwrap();
/// let config = stegoeggo_stego::jpeg::JpegConfig::new(42);
/// let recovered = stegoeggo_stego::jpeg::extract(&jpeg_bytes, 6, &config, config.redundancy()).unwrap();
/// assert_eq!(&recovered, b"secret");
/// ```
pub fn extract(
    jpeg_bytes: &[u8],
    payload_len: usize,
    config: &JpegConfig,
    actual_redundancy: usize,
) -> std::result::Result<Vec<u8>, StegoError> {
    if !jpeg_bytes.starts_with(&[0xFF, 0xD8]) {
        return Err(StegoError::MalformedInput("not a valid JPEG".to_string()));
    }

    let support = probe_support(jpeg_bytes)?;
    if let JpegSupport::Unsupported(reason) = support {
        return Err(StegoError::UnsupportedJpeg(reason));
    }

    let (_, coefficients) = JpegTranscoder::decode_coefficients(jpeg_bytes)
        .map_err(|e| StegoError::MalformedInput(e.to_string()))?;
    let payload_bits = payload_len * 8;
    let extracted_bits = DctStegoF5::with_redundancy(actual_redundancy).extract_f5(
        &coefficients,
        payload_bits,
        config.seed(),
    );

    if extracted_bits.is_empty() {
        return Err(StegoError::MalformedInput(
            "extraction returned no data".to_string(),
        ));
    }

    let bits_to_bytes: Vec<u8> = extracted_bits
        .chunks_exact(8)
        .map(|chunk| {
            chunk
                .iter()
                .enumerate()
                .fold(0u8, |acc, (i, &bit)| acc | (bit << i))
        })
        .collect();

    Ok(bits_to_bytes)
}

/// Embed a seed hint in JPEG quantization tables.
///
/// This stores a seed-like value in the quantization table LSBs. It is
/// fragile under requantization/re-encoding and does not prove a payload
/// exists or authenticate the image.
///
/// # Errors
///
/// Returns an error if the JPEG is malformed or embedding fails.
///
/// # Examples
///
/// ```rust,no_run
/// let jpeg_bytes = std::fs::read("photo.jpg").unwrap();
/// let output = stegoeggo_stego::jpeg::embed_seed_hint(&jpeg_bytes, 42).unwrap();
/// ```
pub fn embed_seed_hint(jpeg_bytes: &[u8], seed: u64) -> std::result::Result<Vec<u8>, StegoError> {
    embed_seed_hint_internal(jpeg_bytes, seed)
}

/// Extract a seed hint from JPEG quantization tables.
///
/// Returns `Ok(Some(seed))` if a seed was found, `Ok(None)` if no seed
/// is present. The seed hint is fragile under requantization/re-encoding.
///
/// # Errors
///
/// Returns an error if the JPEG is malformed.
///
/// # Examples
///
/// ```rust,no_run
/// let jpeg_bytes = std::fs::read("photo.jpg").unwrap();
/// let seed = stegoeggo_stego::jpeg::extract_seed_hint(&jpeg_bytes).unwrap();
/// ```
pub fn extract_seed_hint(jpeg_bytes: &[u8]) -> std::result::Result<Option<u64>, StegoError> {
    extract_seed_hint_internal(jpeg_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jpeg_transcoder::JpegTranscoder;

    fn make_test_jpeg(w: u32, h: u32) -> Vec<u8> {
        let mut img = image::RgbImage::new(w, h);
        for y in 0..h {
            for x in 0..w {
                let r = ((x * 7 + y * 13) % 256) as u8;
                let g = ((x * 11 + y * 3) % 256) as u8;
                let b = ((x * 5 + y * 17) % 256) as u8;
                img.put_pixel(x, y, image::Rgb([r, g, b]));
            }
        }
        let mut buf = Vec::new();
        let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, 90);
        image::DynamicImage::ImageRgb8(img)
            .write_with_encoder(encoder)
            .unwrap();
        buf
    }

    #[test]
    fn coefficient_roundtrip_is_lossless() {
        let jpeg_bytes = make_test_jpeg(256, 256);
        let (header, coefficients) = JpegTranscoder::decode_coefficients(&jpeg_bytes).unwrap();

        let output =
            JpegTranscoder::encode_coefficients(&header, &coefficients, Some(&jpeg_bytes)).unwrap();
        let (_, decoded) = JpegTranscoder::decode_coefficients(&output).unwrap();

        let mut total = 0;
        let mut diffs = 0;
        for (comp_id, blocks) in coefficients.iter() {
            for (block_idx, block) in blocks.iter().enumerate() {
                for (pos, &orig) in block.iter().enumerate() {
                    total += 1;
                    if let Some(decoded_block) = decoded.get(comp_id).and_then(|b| b.get(block_idx))
                    {
                        if orig != decoded_block[pos] {
                            diffs += 1;
                        }
                    } else {
                        diffs += 1;
                    }
                }
            }
        }
        assert_eq!(
            diffs, 0,
            "Coefficient roundtrip has {diffs} diffs out of {total}"
        );
    }

    #[test]
    fn dct_stego_raw_roundtrip() {
        let jpeg_bytes = make_test_jpeg(256, 256);
        let (header, coefficients) = JpegTranscoder::decode_coefficients(&jpeg_bytes).unwrap();

        let payload = b"test payload for jpeg dct";
        let mut embedded = coefficients.clone();
        crate::jpeg_transcoder::DctStegoF5::with_redundancy(3)
            .embed_f5(&mut embedded, payload, 42)
            .unwrap();

        let output =
            JpegTranscoder::encode_coefficients(&header, &embedded, Some(&jpeg_bytes)).unwrap();
        let (_, decoded) = JpegTranscoder::decode_coefficients(&output).unwrap();

        let extracted_bits = crate::jpeg_transcoder::DctStegoF5::with_redundancy(3).extract_f5(
            &decoded,
            payload.len() * 8,
            42,
        );
        let extracted: Vec<u8> = extracted_bits
            .chunks_exact(8)
            .map(|chunk| {
                chunk
                    .iter()
                    .enumerate()
                    .fold(0u8, |acc, (i, &bit)| acc | (bit << i))
            })
            .collect();
        assert_eq!(&extracted, payload);
    }

    #[test]
    fn public_embed_extract_roundtrip() {
        let jpeg_bytes = make_test_jpeg(256, 256);
        let payload = b"hello jpeg stego";
        let config = JpegConfig::new(42);

        let report = embed(&jpeg_bytes, payload, &config).unwrap();
        assert!(report.embedded);

        let recovered = extract(
            &report.output,
            payload.len(),
            &config,
            report.actual_redundancy,
        )
        .unwrap();
        assert_eq!(&recovered, payload);
    }
}
