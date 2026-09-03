//! Public encoded-JPEG DCT steganography carrier.
//!
//! Application-neutral embed/extract and capacity API on top of the
//! crate-internal mechanics in `crate::jpeg_transcoder`. The header
//! parser, coefficient map, Huffman state, and F5 implementation are
//! private; only operation-level public helpers are exposed here.
//!
//! # Capacity units
//!
//! Capacity is reported in **eligible non-zero AC coefficients** across
//! all components. The DC coefficient and zero-valued AC coefficients are
//! not carriers. Embedding auto-selects the largest feasible redundancy
//! (capped by the configured redundancy) via one pass; failed embeds
//! still emit a seed-only carrier via quantization-table LSBs.
//!
//! # Raw vs framed vs tiled
//!
//! - **Raw** ([`embed`], [`extract`]) — caller-supplied payload length and
//!   the `actual_redundancy` returned by the embed report. Use when the
//!   caller wants explicit control of redundancy.
//! - **Framed** ([`embed_framed`], [`extract_framed`]) — wraps the payload
//!   in a self-describing header with a CRC32 and recovers without caller
//!   knowledge of the original payload length or the actual redundancy
//!   used at embed time. Extract probes the configured redundancy down
//!   to 1.
//! - **Tiled** ([`embed_tiled`], [`extract_tiled`], [`embed_tiled_framed`],
//!   [`extract_tiled_framed`]) — spatial/crop-oriented repetition: the full
//!   payload is embedded in each tile's DCT blocks with redundancy 1 and a
//!   deterministic tile-local seed. Recovery is explicitly bounded by
//!   `max_origins` and decodes supported coefficients once per operation.
//!
//! # Supported JPEG subset
//!
//! Only **8-bit, sequential, single-scan, Huffman-coded** JPEGs with up to
//! 4 components and supported sampling factors are embeddable. Progressive,
//! arithmetic, multi-scan, restart-interval, and other unsupported inputs
//! are rejected by [`probe_support`] and return
//! [`StegoError::UnsupportedJpeg`]; only the seed hint may be embedded
//! in those inputs.
//!
//! # Container preservation
//!
//! Successful embedding uses the original-JPEG byte-preserving encode
//! path, so APP2, APP13, APP14, COM, and unknown marker segments survive
//! byte-for-byte.

use crate::error::{JpegUnsupportedReason, StegoError};
use crate::jpeg_transcoder::{DctStegoF5, JpegHeader, JpegTranscoder};

#[cfg(test)]
use std::cell::Cell;

#[cfg(test)]
thread_local! {
    static JPEG_COEFFICIENT_DECODE_COUNT: Cell<usize> = const { Cell::new(0) };
}

#[cfg(test)]
pub(crate) fn reset_decode_count() {
    JPEG_COEFFICIENT_DECODE_COUNT.with(|count| count.set(0));
}

#[cfg(test)]
pub(crate) fn decode_count() -> usize {
    JPEG_COEFFICIENT_DECODE_COUNT.with(Cell::get)
}

/// Basic JPEG dimensions returned by a bounded structural inspection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JpegInfo {
    /// Image width in pixels.
    pub width: u32,
    /// Image height in pixels.
    pub height: u32,
}

/// Inspect a JPEG header with caller-provided segment limits.
pub fn inspect(
    jpeg_bytes: &[u8],
    max_segments: usize,
    max_segment_bytes: usize,
) -> std::result::Result<JpegInfo, StegoError> {
    let limits = crate::jpeg_transcoder::header::ParseLimits {
        max_jpeg_segments: max_segments,
        max_jpeg_segment_bytes: max_segment_bytes,
    };
    let header = JpegHeader::parse_with_limits(jpeg_bytes, &limits)
        .map_err(|e| StegoError::MalformedInput(e.to_string()))?;
    Ok(JpegInfo {
        width: header.width as u32,
        height: header.height as u32,
    })
}

/// Returns whether a JPEG header declares progressive encoding.
#[must_use]
pub fn is_progressive_jpeg(jpeg_bytes: &[u8]) -> bool {
    JpegHeader::parse(jpeg_bytes)
        .map(|header| header.is_progressive)
        .unwrap_or(false)
}

fn dct_payload_capacity(coefficients: &crate::jpeg_transcoder::Coefficients) -> usize {
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

pub(crate) struct DecodedJpegCarrier {
    header: Box<crate::jpeg_transcoder::JpegHeader>,
    coefficients: crate::jpeg_transcoder::Coefficients,
    available_capacity: usize,
}

impl DecodedJpegCarrier {
    pub(crate) fn header(&self) -> &crate::jpeg_transcoder::JpegHeader {
        &self.header
    }

    pub(crate) fn coefficients(&self) -> &crate::jpeg_transcoder::Coefficients {
        &self.coefficients
    }

    pub(crate) fn available_capacity(&self) -> usize {
        self.available_capacity
    }

    pub(crate) fn extract_raw(
        &self,
        payload_len: usize,
        seed: u64,
        redundancy: usize,
    ) -> std::result::Result<Vec<u8>, StegoError> {
        extract_from_decoded(self, payload_len, seed, redundancy)
    }

    pub(crate) fn capacity_for(
        &self,
        payload_len: usize,
        redundancy: usize,
    ) -> std::result::Result<super::CapacityReport, StegoError> {
        capacity_from_decoded(self, payload_len, redundancy)
    }
}

fn checked_payload_bits(payload_len: usize) -> std::result::Result<usize, StegoError> {
    payload_len.checked_mul(8).ok_or_else(|| {
        StegoError::InvalidConfig(format!(
            "payload length {payload_len} overflows the JPEG bit-count calculation"
        ))
    })
}

fn checked_required_capacity(
    payload_len: usize,
    redundancy: usize,
) -> std::result::Result<usize, StegoError> {
    checked_payload_bits(payload_len)?.checked_mul(redundancy).ok_or_else(|| {
        StegoError::InvalidConfig(format!(
            "payload length {payload_len} and redundancy {redundancy} overflow the JPEG capacity calculation"
        ))
    })
}

pub(crate) fn decode_supported_carrier(
    jpeg_bytes: &[u8],
) -> std::result::Result<DecodedJpegCarrier, StegoError> {
    if !jpeg_bytes.starts_with(&[0xFF, 0xD8]) {
        return Err(StegoError::MalformedInput("not a valid JPEG".to_string()));
    }

    #[cfg(test)]
    JPEG_COEFFICIENT_DECODE_COUNT.with(|count| count.set(count.get() + 1));

    let decoded = JpegTranscoder::decode_coefficients_with_probe(jpeg_bytes)
        .map_err(|e| StegoError::MalformedInput(e.to_string()))?;
    let (header, coefficients) = match decoded {
        crate::jpeg_transcoder::CoefficientDecode::Supported {
            header,
            coefficients,
        } => (header, coefficients),
        crate::jpeg_transcoder::CoefficientDecode::Unsupported(reason) => {
            return Err(StegoError::UnsupportedJpeg(map_unsupported_reason(reason)));
        }
    };
    let available_capacity = dct_payload_capacity(&coefficients);

    Ok(DecodedJpegCarrier {
        header,
        available_capacity,
        coefficients,
    })
}

fn capacity_from_decoded(
    decoded: &DecodedJpegCarrier,
    payload_len: usize,
    redundancy: usize,
) -> std::result::Result<super::CapacityReport, StegoError> {
    crate::constants::validate_redundancy(redundancy)?;
    let required = checked_required_capacity(payload_len, redundancy)?;

    Ok(super::CapacityReport {
        required,
        available: decoded.available_capacity,
    })
}

fn extract_from_decoded(
    decoded: &DecodedJpegCarrier,
    payload_len: usize,
    seed: u64,
    actual_redundancy: usize,
) -> std::result::Result<Vec<u8>, StegoError> {
    crate::constants::validate_redundancy(actual_redundancy)?;
    let payload_bits = checked_payload_bits(payload_len)?;
    let extracted_bits = DctStegoF5::with_redundancy(actual_redundancy).extract_f5(
        &decoded.coefficients,
        payload_bits,
        seed,
    );

    if extracted_bits.is_empty() {
        return Err(StegoError::MalformedInput(
            "extraction returned no data".to_string(),
        ));
    }

    Ok(extracted_bits
        .chunks_exact(8)
        .map(|chunk| {
            chunk
                .iter()
                .enumerate()
                .fold(0u8, |acc, (i, &bit)| acc | (bit << i))
        })
        .collect())
}

#[derive(Default)]
struct FramedFailure {
    full_frame: Option<StegoError>,
    prefix: Option<StegoError>,
    capacity: Option<super::CapacityReport>,
}

impl FramedFailure {
    fn record_capacity(&mut self, capacity: super::CapacityReport) {
        if self.capacity.is_none() {
            self.capacity = Some(capacity);
        }
    }

    fn record_prefix(&mut self, error: StegoError) {
        if self.prefix.is_none() {
            self.prefix = Some(error);
        }
    }

    fn record_full_frame(&mut self, error: StegoError) {
        if self.full_frame.is_none() {
            self.full_frame = Some(error);
        }
    }

    fn into_error(self) -> StegoError {
        if let Some(error) = self.full_frame {
            return error;
        }
        if let Some(error) = self.prefix {
            return error;
        }
        if let Some(capacity) = self.capacity {
            return StegoError::InsufficientCapacity {
                required: capacity.required,
                available: capacity.available,
            };
        }
        StegoError::FrameNotFound
    }
}

fn embed_seed_hint_internal(
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

fn extract_seed_hint_internal(jpeg_bytes: &[u8]) -> std::result::Result<Option<u64>, StegoError> {
    if !jpeg_bytes.starts_with(&[0xFF, 0xD8]) {
        return Err(StegoError::MalformedInput("Not a valid JPEG".to_string()));
    }
    let header =
        JpegHeader::parse(jpeg_bytes).map_err(|e| StegoError::MalformedInput(e.to_string()))?;
    Ok(DctStegoF5::new().extract_seed_from_quantization_tables(&header))
}

fn reassemble_jpeg_with_qtables(
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

        if marker == 0xFF {
            pos += 1;
            continue;
        }

        if marker == 0xDA || marker == 0xD9 {
            output.extend_from_slice(&jpeg_bytes[pos..]);
            break;
        }

        if marker == 0xDB {
            let segment_len =
                u16::from_be_bytes([jpeg_bytes[pos + 2], jpeg_bytes[pos + 3]]) as usize;
            let segment_end = pos + 2 + segment_len;
            if segment_end > jpeg_bytes.len() {
                return Err(StegoError::MalformedInput(
                    "Malformed JPEG segment length exceeds buffer".into(),
                ));
            }
            pos = segment_end;

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
            pos += 2;
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
/// Use [`JpegConfig::new`] for compile-time-valid values and
/// [`JpegConfig::try_new`] / [`JpegConfig::try_with_redundancy`] when the
/// redundancy comes from untrusted input (configuration files, CLI parsing,
/// network requests, etc.).
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

    /// Fallible constructor that validates the redundancy up front.
    ///
    /// Use this when the redundancy value comes from untrusted input. Returns
    /// [`StegoError::InvalidConfig`] if `redundancy` is outside `1..=10`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use stegoeggo_stego::jpeg::JpegConfig;
    ///
    /// let config = JpegConfig::try_new(42, 3)?;
    /// assert_eq!(config.redundancy(), 3);
    /// # Ok::<_, stegoeggo_stego::StegoError>(())
    /// ```
    pub fn try_new(seed: u64, redundancy: usize) -> Result<Self, StegoError> {
        crate::constants::validate_redundancy(redundancy)?;
        Ok(Self { seed, redundancy })
    }

    /// Set the redundancy level (1–10). Higher redundancy increases
    /// robustness at the cost of reduced capacity.
    ///
    /// In debug builds, panics if `redundancy` is 0 or greater than 10. In
    /// release builds with `panic=abort`, an out-of-range value is clamped to
    /// `1..=10` to avoid aborting the process; prefer
    /// [`JpegConfig::try_with_redundancy`](Self::try_with_redundancy) when
    /// the value is not statically known to be in `1..=10` (for example
    /// values from configuration files, CLI flags, or network payloads,
    /// which must not abort the process on invalid input).
    #[must_use]
    pub fn with_redundancy(mut self, redundancy: usize) -> Self {
        debug_assert!(
            (1..=10).contains(&redundancy),
            "redundancy must be 1..=10, got {redundancy}"
        );
        if !(1..=10).contains(&redundancy) {
            self.redundancy = redundancy.clamp(1, 10);
            return self;
        }
        self.redundancy = redundancy;
        self
    }

    /// Fallible variant of [`with_redundancy`](Self::with_redundancy).
    ///
    /// Returns [`StegoError::InvalidConfig`] if `redundancy` is outside
    /// `1..=10`. Prefer this over [`with_redundancy`](Self::with_redundancy)
    /// when the value is derived from runtime configuration.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use stegoeggo_stego::jpeg::JpegConfig;
    ///
    /// let user_redundancy: usize = 99;
    /// let result = JpegConfig::new(42).try_with_redundancy(user_redundancy);
    /// assert!(result.is_err());
    /// # Ok::<_, stegoeggo_stego::StegoError>(())
    /// ```
    pub fn try_with_redundancy(mut self, redundancy: usize) -> Result<Self, StegoError> {
        crate::constants::validate_redundancy(redundancy)?;
        self.redundancy = redundancy;
        Ok(self)
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
    crate::constants::validate_redundancy(config.redundancy())?;
    let required = checked_required_capacity(payload_len, config.redundancy())?;
    let decoded = decode_supported_carrier(jpeg_bytes)?;
    Ok(super::CapacityReport {
        required,
        available: decoded.available_capacity,
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
    crate::constants::validate_redundancy(config.redundancy())?;
    let payload_bits = checked_payload_bits(payload.len())?;
    let required = checked_required_capacity(payload.len(), config.redundancy())?;

    if !jpeg_bytes.starts_with(&[0xFF, 0xD8]) {
        return Err(StegoError::MalformedInput("not a valid JPEG".to_string()));
    }

    let decoded = JpegTranscoder::decode_coefficients_with_probe(jpeg_bytes)
        .map_err(|e| StegoError::MalformedInput(e.to_string()))?;
    let (header, coefficients) = match decoded {
        crate::jpeg_transcoder::CoefficientDecode::Supported {
            header,
            coefficients,
        } => (*header, coefficients),
        crate::jpeg_transcoder::CoefficientDecode::Unsupported(reason) => {
            return Err(StegoError::UnsupportedJpeg(map_unsupported_reason(reason)));
        }
    };

    let available = dct_payload_capacity(&coefficients);

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
        actual_redundancy: 0,
    })
}

/// Embed a self-describing framed payload into a supported JPEG.
///
/// The frame is encoded with [`crate::frame::encode`] and then embedded using
/// the raw JPEG DCT carrier. The returned report's `payload_bytes` includes
/// the frame header and CRC overhead because those bytes are placed in the
/// carrier.
///
/// # Examples
///
/// ```rust,no_run
/// let jpeg_bytes = std::fs::read("photo.jpg").unwrap();
/// let config = stegoeggo_stego::jpeg::JpegConfig::new(42);
/// let report = stegoeggo_stego::jpeg::embed_framed(&jpeg_bytes, b"payload", &config).unwrap();
/// std::fs::write("output.jpg", &report.output).unwrap();
/// ```
pub fn embed_framed(
    jpeg_bytes: &[u8],
    payload: &[u8],
    config: &JpegConfig,
) -> std::result::Result<super::EmbedReport, StegoError> {
    let framed = crate::frame::encode(payload)?;
    embed(jpeg_bytes, &framed, config)
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
///   or the `actual_redundancy` from the [`EmbedReport`](super::EmbedReport) if embedding
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
    crate::constants::validate_redundancy(actual_redundancy)?;
    checked_payload_bits(payload_len)?;

    if !jpeg_bytes.starts_with(&[0xFF, 0xD8]) {
        return Err(StegoError::MalformedInput("not a valid JPEG".to_string()));
    }

    let decoded = decode_supported_carrier(jpeg_bytes)?;
    extract_from_decoded(&decoded, payload_len, config.seed(), actual_redundancy)
}

/// Extract and validate a self-describing framed payload from a supported
/// JPEG without retaining the original payload length or embedding report.
///
/// The configured redundancy is tried first, followed by each lower valid
/// redundancy. Every candidate must produce a complete frame with a valid
/// CRC32 before it is accepted. The search is bounded by the configured
/// redundancy range (1 through 10) and never uses JPEG seed hints or
/// application metadata.
///
/// # Examples
///
/// ```rust,no_run
/// let jpeg_bytes = std::fs::read("output.jpg").unwrap();
/// let config = stegoeggo_stego::jpeg::JpegConfig::new(42);
/// let payload = stegoeggo_stego::jpeg::extract_framed(&jpeg_bytes, &config).unwrap();
/// println!("{} bytes", payload.len());
/// ```
pub fn extract_framed(
    jpeg_bytes: &[u8],
    config: &JpegConfig,
) -> std::result::Result<Vec<u8>, StegoError> {
    crate::constants::validate_redundancy(config.redundancy())?;
    let decoded = decode_supported_carrier(jpeg_bytes)?;
    let mut failures = FramedFailure::default();

    for redundancy in (1..=config.redundancy()).rev() {
        let prefix_capacity =
            capacity_from_decoded(&decoded, crate::frame::FRAME_HEADER_SIZE, redundancy)?;
        if !prefix_capacity.is_sufficient() {
            failures.record_capacity(prefix_capacity);
            continue;
        }

        let prefix = match extract_from_decoded(
            &decoded,
            crate::frame::FRAME_HEADER_SIZE,
            config.seed(),
            redundancy,
        ) {
            Ok(prefix) => prefix,
            Err(error) => {
                failures.record_prefix(error);
                continue;
            }
        };

        let (_, total_len) = match crate::frame::decode_prefix(&prefix) {
            Ok(prefix) => prefix,
            Err(error) => {
                failures.record_prefix(error);
                continue;
            }
        };

        let frame_capacity = capacity_from_decoded(&decoded, total_len, redundancy)?;
        if !frame_capacity.is_sufficient() {
            failures.record_capacity(frame_capacity);
            continue;
        }

        let framed = match extract_from_decoded(&decoded, total_len, config.seed(), redundancy) {
            Ok(framed) => framed,
            Err(error) => {
                failures.record_full_frame(error);
                continue;
            }
        };

        match crate::frame::decode(&framed) {
            Ok((_, payload)) => return Ok(payload),
            Err(error) => failures.record_full_frame(error),
        }
    }

    Err(failures.into_error())
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

pub use crate::types::TileConfig;

/// Validate a JPEG tiled tile size.
///
/// Requires `tile_size >= 8` and a multiple of 8 so the size maps
/// deterministically to DCT blocks. Rejects rather than silently truncating
/// non-multiples.
fn validate_jpeg_tile_size(tile_size: u32) -> std::result::Result<(), StegoError> {
    if tile_size < 8 {
        return Err(StegoError::InvalidConfig(format!(
            "JPEG tile size must be at least 8, got {tile_size}"
        )));
    }
    if !tile_size.is_multiple_of(8) {
        return Err(StegoError::InvalidConfig(format!(
            "JPEG tile size must be a multiple of 8, got {tile_size}"
        )));
    }
    Ok(())
}

fn jpeg_tile_geometry(header: &JpegHeader, tile_size: u32) -> Option<(u32, u32)> {
    let max_h = header
        .components
        .iter()
        .map(|c| c.h_sampling as u32)
        .max()
        .unwrap_or(1);
    let max_v = header
        .components
        .iter()
        .map(|c| c.v_sampling as u32)
        .max()
        .unwrap_or(1);
    let luma_blocks_x = (header.width as u32).div_ceil(max_h * 8) * max_h;
    let luma_blocks_y = (header.height as u32).div_ceil(max_v * 8) * max_v;
    let blocks_per_tile = tile_size / 8;
    (blocks_per_tile > 0).then(|| {
        (
            luma_blocks_x / blocks_per_tile,
            luma_blocks_y / blocks_per_tile,
        )
    })
}

fn jpeg_tiled_payload_matches_decoded(
    header: &JpegHeader,
    coefficients: &crate::jpeg_transcoder::Coefficients,
    payload: &[u8],
    tile_size: u32,
    tile_x: u32,
    tile_y: u32,
    local_seed: u64,
) -> bool {
    let tile_blocks = DctStegoF5::tile_block_set(header, coefficients, tile_x, tile_y, tile_size);
    let payload_bits = payload.len().saturating_mul(8);
    let bits = DctStegoF5::with_redundancy(1).extract_f5_from_blocks(
        coefficients,
        payload_bits,
        local_seed,
        &tile_blocks,
    );
    bits.len() >= payload_bits && crate::lsb_internal::bits_to_bytes(&bits) == payload
}

/// Embed arbitrary bytes once per tile for crop resistance.
///
/// Each tile's DCT blocks embed the full payload with redundancy 1 using a
/// deterministic tile-local seed (`tile_seed(master, tx, ty)`); the tile
/// grid itself is the redundancy. Uses the same supported-JPEG subset and
/// container-preserving encode as [`embed`]. The production self-check
/// extracts the first successful tile from the already-mutated in-memory
/// coefficients (one decode + one encode, no re-decode of the output).
///
/// # Errors
///
/// Returns [`StegoError::InvalidConfig`] for `tile_size < 8` or a
/// non-multiple of 8. Returns [`StegoError::UnsupportedJpeg`] for
/// unsupported structures (no silent conversion, no application seed
/// fallback).
///
/// ```rust,no_run
/// use stegoeggo_stego::jpeg::{self, TileConfig};
///
/// let jpeg_bytes = std::fs::read("photo.jpg").unwrap();
/// let config = TileConfig::try_new(42, 64).unwrap();
/// if jpeg::probe_support(&jpeg_bytes).unwrap() == jpeg::JpegSupport::Supported {
///     let report = jpeg::embed_tiled(&jpeg_bytes, b"payload", &config).unwrap();
///     assert!(report.embedded);
/// }
/// ```
pub fn embed_tiled(
    jpeg_bytes: &[u8],
    payload: &[u8],
    config: &TileConfig,
) -> std::result::Result<super::EmbedReport, StegoError> {
    validate_jpeg_tile_size(config.tile_size())?;
    if !jpeg_bytes.starts_with(&[0xFF, 0xD8]) {
        return Err(StegoError::MalformedInput("not a valid JPEG".to_string()));
    }
    let tile_size = config.tile_size();
    let seed = config.seed();
    let payload_bits = payload.len().saturating_mul(8);

    let decoded = JpegTranscoder::decode_coefficients_with_probe(jpeg_bytes)
        .map_err(|e| StegoError::MalformedInput(e.to_string()))?;
    let (mut header, mut coefficients) = match decoded {
        crate::jpeg_transcoder::CoefficientDecode::Supported {
            header,
            coefficients,
        } => (*header, coefficients),
        crate::jpeg_transcoder::CoefficientDecode::Unsupported(reason) => {
            return Err(StegoError::UnsupportedJpeg(map_unsupported_reason(reason)));
        }
    };
    DctStegoF5::new()
        .embed_seed_in_quantization_tables(&mut header, seed)
        .map_err(|e| StegoError::MalformedInput(e.to_string()))?;

    let max_h = header
        .components
        .iter()
        .map(|c| c.h_sampling as u32)
        .max()
        .unwrap_or(1);
    let max_v = header
        .components
        .iter()
        .map(|c| c.v_sampling as u32)
        .max()
        .unwrap_or(1);
    let luma_blocks_x = (header.width as u32).div_ceil(max_h * 8) * max_h;
    let luma_blocks_y = (header.height as u32).div_ceil(max_v * 8) * max_v;
    let blocks_per_tile = tile_size / 8;
    let tiles_x = luma_blocks_x / blocks_per_tile;
    let tiles_y = luma_blocks_y / blocks_per_tile;
    let mut first_embedded: Option<(u32, u32, u64)> = None;

    for ty in 0..tiles_y {
        for tx in 0..tiles_x {
            let tile_blocks = DctStegoF5::tile_block_set(&header, &coefficients, tx, ty, tile_size);
            if tile_blocks.is_empty() {
                continue;
            }
            if DctStegoF5::with_redundancy(1)
                .embed_f5_in_blocks(
                    &mut coefficients,
                    payload,
                    crate::lsb_internal::tile_seed(seed, tx, ty),
                    &tile_blocks,
                )
                .is_ok()
            {
                first_embedded.get_or_insert((
                    tx,
                    ty,
                    crate::lsb_internal::tile_seed(seed, tx, ty),
                ));
            }
        }
    }

    let output = JpegTranscoder::encode_coefficients(&header, &coefficients, Some(jpeg_bytes))
        .map_err(|e| StegoError::MalformedInput(e.to_string()))?;
    let Some((tile_x, tile_y, local_seed)) = first_embedded else {
        return Ok(super::EmbedReport {
            embedded: false,
            output,
            payload_bytes: payload.len(),
            required_capacity: payload_bits,
            available_capacity: 0,
            actual_redundancy: 0,
        });
    };

    if !jpeg_tiled_payload_matches_decoded(
        &header,
        &coefficients,
        payload,
        tile_size,
        tile_x,
        tile_y,
        local_seed,
    ) {
        return Ok(super::EmbedReport {
            embedded: false,
            output,
            payload_bytes: payload.len(),
            required_capacity: payload_bits,
            available_capacity: 0,
            actual_redundancy: 0,
        });
    }
    Ok(super::EmbedReport {
        embedded: true,
        output,
        payload_bytes: payload.len(),
        required_capacity: payload_bits,
        available_capacity: payload_bits,
        actual_redundancy: 1,
    })
}

/// Extract tiled payload bytes when the payload length is known.
///
/// Decodes supported coefficients once per operation and searches at most
/// `max_origins` tile origins with the tile-grid neighbourhood (`0..=2`)
/// used by the current tiled path. Returns the first candidate in
/// deterministic scan order with redundancy 1.
///
/// Raw mode cannot authenticate correctness; prefer [`extract_tiled_framed`]
/// for self-validating crop recovery.
///
/// # Errors
///
/// Returns [`StegoError::InvalidConfig`] for an invalid tile size or an
/// out-of-range `max_origins`. Returns [`StegoError::UnsupportedJpeg`] for
/// unsupported structures.
pub fn extract_tiled(
    jpeg_bytes: &[u8],
    payload_len: usize,
    config: &TileConfig,
    max_origins: u32,
) -> std::result::Result<Vec<u8>, StegoError> {
    validate_jpeg_tile_size(config.tile_size())?;
    crate::types::validate_max_origins(max_origins)?;
    checked_payload_bits(payload_len)?;
    let tile_size = config.tile_size();
    let decoded = decode_supported_carrier(jpeg_bytes)?;
    let Some((tiles_x, tiles_y)) = jpeg_tile_geometry(&decoded.header, tile_size) else {
        return Err(StegoError::InvalidConfig(format!(
            "JPEG tile size {tile_size} does not map to DCT blocks"
        )));
    };
    if tiles_x == 0 || tiles_y == 0 {
        return Err(StegoError::InsufficientCapacity {
            required: payload_len.saturating_mul(8),
            available: 0,
        });
    }
    let payload_bits = payload_len.saturating_mul(8);
    let mut origins_tried = 0u32;
    for ty in 0..tiles_y {
        for tx in 0..tiles_x {
            if origins_tried >= max_origins {
                return Err(StegoError::FrameNotFound);
            }
            origins_tried += 1;
            let tile_blocks = DctStegoF5::tile_block_set(
                &decoded.header,
                &decoded.coefficients,
                tx,
                ty,
                tile_size,
            );
            if tile_blocks.is_empty() {
                continue;
            }
            for dy in 0..=2u32 {
                if ty.saturating_add(dy) >= 16 {
                    break;
                }
                for dx in 0..=2u32 {
                    if tx.saturating_add(dx) >= 16 {
                        break;
                    }
                    let local_seed =
                        crate::lsb_internal::tile_seed(config.seed(), tx + dx, ty + dy);
                    let bits = DctStegoF5::with_redundancy(1).extract_f5_from_blocks(
                        &decoded.coefficients,
                        payload_bits,
                        local_seed,
                        &tile_blocks,
                    );
                    if bits.len() >= payload_bits {
                        return Ok(crate::lsb_internal::bits_to_bytes(&bits));
                    }
                }
            }
        }
    }
    Err(StegoError::FrameNotFound)
}

/// Embed a self-describing framed payload once per tile.
///
/// The frame is encoded with [`crate::frame::encode`] and then embedded with
/// [`embed_tiled`]. The report's `payload_bytes` includes frame overhead.
pub fn embed_tiled_framed(
    jpeg_bytes: &[u8],
    payload: &[u8],
    config: &TileConfig,
) -> std::result::Result<super::EmbedReport, StegoError> {
    let framed = crate::frame::encode(payload)?;
    embed_tiled(jpeg_bytes, &framed, config)
}

/// Extract and validate a framed tiled payload without caller-known length.
///
/// Retains one decoded coefficient state across prefix/full candidate
/// validation. A candidate's prefix, header, and full extraction stay tied
/// to the same tile origin, seed-offset identity, and redundancy 1; equal
/// prefix bytes from distinct candidates do not collapse identity. The
/// declared frame length is validated against frame bounds and per-tile
/// capacity before full extraction.
///
/// # Errors
///
/// Returns [`StegoError::InvalidConfig`] for an invalid tile size or an
/// out-of-range `max_origins`. Returns [`StegoError::UnsupportedJpeg`] for
/// unsupported structures. Returns [`StegoError::FrameNotFound`] when the
/// bounded search finds no valid frame.
pub fn extract_tiled_framed(
    jpeg_bytes: &[u8],
    config: &TileConfig,
    max_origins: u32,
) -> std::result::Result<Vec<u8>, StegoError> {
    validate_jpeg_tile_size(config.tile_size())?;
    crate::types::validate_max_origins(max_origins)?;
    let tile_size = config.tile_size();
    let decoded = decode_supported_carrier(jpeg_bytes)?;
    let Some((tiles_x, tiles_y)) = jpeg_tile_geometry(&decoded.header, tile_size) else {
        return Err(StegoError::InvalidConfig(format!(
            "JPEG tile size {tile_size} does not map to DCT blocks"
        )));
    };
    if tiles_x == 0 || tiles_y == 0 {
        let header_bits = crate::frame::FRAME_HEADER_SIZE.saturating_mul(8);
        return Err(StegoError::InsufficientCapacity {
            required: header_bits,
            available: 0,
        });
    }
    let prefix_bits = crate::frame::FRAME_HEADER_SIZE.saturating_mul(8);
    let mut origins_tried = 0u32;
    for ty in 0..tiles_y {
        for tx in 0..tiles_x {
            if origins_tried >= max_origins {
                return Err(StegoError::FrameNotFound);
            }
            origins_tried += 1;
            let tile_blocks = DctStegoF5::tile_block_set(
                &decoded.header,
                &decoded.coefficients,
                tx,
                ty,
                tile_size,
            );
            if tile_blocks.is_empty() {
                continue;
            }
            for dy in 0..=2u32 {
                if ty.saturating_add(dy) >= 16 {
                    break;
                }
                for dx in 0..=2u32 {
                    if tx.saturating_add(dx) >= 16 {
                        break;
                    }
                    let local_seed =
                        crate::lsb_internal::tile_seed(config.seed(), tx + dx, ty + dy);
                    let prefix_bits_vec = DctStegoF5::with_redundancy(1).extract_f5_from_blocks(
                        &decoded.coefficients,
                        prefix_bits,
                        local_seed,
                        &tile_blocks,
                    );
                    if prefix_bits_vec.len() < prefix_bits {
                        continue;
                    }
                    let prefix = crate::lsb_internal::bits_to_bytes(&prefix_bits_vec);
                    let Ok((_, total_len)) = crate::frame::decode_prefix(&prefix) else {
                        continue;
                    };
                    let Some(total_bits) = total_len.checked_mul(8) else {
                        continue;
                    };
                    let full_bits_vec = DctStegoF5::with_redundancy(1).extract_f5_from_blocks(
                        &decoded.coefficients,
                        total_bits,
                        local_seed,
                        &tile_blocks,
                    );
                    if full_bits_vec.len() < total_bits {
                        continue;
                    }
                    let framed = crate::lsb_internal::bits_to_bytes(&full_bits_vec);
                    if let Ok((_, payload)) = crate::frame::decode(&framed) {
                        return Ok(payload);
                    }
                }
            }
        }
    }
    Err(StegoError::FrameNotFound)
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

    #[test]
    fn jpeg_config_try_new_accepts_valid_redundancy() {
        for r in 1..=10usize {
            let config = JpegConfig::try_new(42, r).unwrap();
            assert_eq!(config.redundancy(), r);
            assert_eq!(config.seed(), 42);
        }
    }

    #[test]
    fn jpeg_config_try_new_rejects_out_of_range_redundancy() {
        assert!(matches!(
            JpegConfig::try_new(42, 0),
            Err(StegoError::InvalidConfig(_))
        ));
        assert!(matches!(
            JpegConfig::try_new(42, 11),
            Err(StegoError::InvalidConfig(_))
        ));
        assert!(matches!(
            JpegConfig::try_new(42, usize::MAX),
            Err(StegoError::InvalidConfig(_))
        ));
    }

    #[test]
    fn jpeg_config_try_with_redundancy_accepts_valid_values() {
        let config = JpegConfig::new(42).try_with_redundancy(7).unwrap();
        assert_eq!(config.redundancy(), 7);
    }

    #[test]
    fn jpeg_config_try_with_redundancy_rejects_out_of_range() {
        assert!(JpegConfig::new(42).try_with_redundancy(0).is_err());
        assert!(JpegConfig::new(42).try_with_redundancy(11).is_err());
        assert!(JpegConfig::new(42).try_with_redundancy(usize::MAX).is_err());
    }

    #[test]
    fn jpeg_config_fallible_does_not_panic() {
        for r in [0usize, 11, usize::MAX, 100, 1_000_000] {
            let result = std::panic::catch_unwind(|| JpegConfig::try_new(42, r));
            assert!(result.is_ok(), "try_new panicked for redundancy {r}");
            assert!(
                result.unwrap().is_err(),
                "expected error for redundancy {r}"
            );

            let result = std::panic::catch_unwind(|| JpegConfig::new(42).try_with_redundancy(r));
            assert!(
                result.is_ok(),
                "try_with_redundancy panicked for redundancy {r}"
            );
            assert!(
                result.unwrap().is_err(),
                "expected error for redundancy {r}"
            );
        }
    }

    #[test]
    fn jpeg_public_lengths_and_redundancy_are_checked() {
        let jpeg_bytes = make_test_jpeg(256, 256);
        let config = JpegConfig::new(42);

        let capacity_result =
            std::panic::catch_unwind(|| capacity(&jpeg_bytes, usize::MAX, &config));
        assert!(capacity_result.is_ok());
        assert!(matches!(
            capacity_result.unwrap(),
            Err(StegoError::InvalidConfig(_))
        ));

        let extract_result =
            std::panic::catch_unwind(|| extract(&jpeg_bytes, usize::MAX, &config, 1));
        assert!(extract_result.is_ok());
        assert!(matches!(
            extract_result.unwrap(),
            Err(StegoError::InvalidConfig(_))
        ));

        for redundancy in [0, 11, usize::MAX] {
            let result = extract(&jpeg_bytes, 1, &config, redundancy);
            assert!(matches!(result, Err(StegoError::InvalidConfig(_))));
        }

        for redundancy in [1, 10] {
            let config = JpegConfig::new(42).with_redundancy(redundancy);
            let report = embed(&jpeg_bytes, b"raw", &config).unwrap();
            assert!(report.embedded);
            assert_eq!(report.actual_redundancy, redundancy);
            let recovered = extract(&report.output, 3, &config, report.actual_redundancy).unwrap();
            assert_eq!(&recovered, b"raw");
        }
    }

    #[test]
    fn framed_extraction_decodes_coefficients_once_per_operation() {
        let jpeg_bytes = make_test_jpeg(256, 256);
        let first = embed_framed(&jpeg_bytes, b"one", &JpegConfig::new(42).with_redundancy(1))
            .unwrap()
            .output;
        let second = embed_framed(&jpeg_bytes, b"three", &JpegConfig::new(42))
            .unwrap()
            .output;

        for (output, config) in [
            (first.clone(), JpegConfig::new(42).with_redundancy(1)),
            (second.clone(), JpegConfig::new(42)),
        ] {
            JPEG_COEFFICIENT_DECODE_COUNT.with(|count| count.set(0));
            assert!(extract_framed(&output, &config).is_ok());
            assert_eq!(JPEG_COEFFICIENT_DECODE_COUNT.with(Cell::get), 1);
        }

        let requested = JpegConfig::new(42);
        let available = capacity(&jpeg_bytes, 1, &requested).unwrap().available;
        let framed_len = available / 16;
        assert!(framed_len > crate::frame::FRAME_HEADER_SIZE);
        let payload = vec![0xA5; framed_len - crate::frame::FRAME_HEADER_SIZE];
        let downgraded = embed_framed(&jpeg_bytes, &payload, &requested).unwrap();
        assert!(downgraded.actual_redundancy < requested.redundancy());

        JPEG_COEFFICIENT_DECODE_COUNT.with(|count| count.set(0));
        assert_eq!(
            extract_framed(&downgraded.output, &requested).unwrap(),
            payload
        );
        assert_eq!(JPEG_COEFFICIENT_DECODE_COUNT.with(Cell::get), 1);

        let wrong_seed = JpegConfig::new(43).with_redundancy(10);
        JPEG_COEFFICIENT_DECODE_COUNT.with(|count| count.set(0));
        assert!(extract_framed(&second, &wrong_seed).is_err());
        assert_eq!(JPEG_COEFFICIENT_DECODE_COUNT.with(Cell::get), 1);
    }

    #[test]
    fn one_shot_extract_decodes_once_per_call() {
        let jpeg_bytes = make_test_jpeg(256, 256);
        let config = JpegConfig::new(42);
        let report = embed(&jpeg_bytes, b"raw", &config).unwrap();
        assert!(report.embedded);
        JPEG_COEFFICIENT_DECODE_COUNT.with(|count| count.set(0));
        for redundancy in 1..=10 {
            let _ = extract(&report.output, 3, &config, redundancy);
        }
        assert_eq!(JPEG_COEFFICIENT_DECODE_COUNT.with(Cell::get), 10);
    }

    #[test]
    fn framed_failure_precedence_is_deterministic() {
        let mut failures = FramedFailure::default();
        failures.record_capacity(crate::CapacityReport {
            required: 20,
            available: 10,
        });
        failures.record_prefix(StegoError::MalformedFrame("bad prefix".to_string()));
        failures.record_full_frame(StegoError::FrameChecksumMismatch);
        failures.record_prefix(StegoError::FrameNotFound);
        assert!(matches!(
            failures.into_error(),
            StegoError::FrameChecksumMismatch
        ));

        let mut all_capacity = FramedFailure::default();
        all_capacity.record_capacity(crate::CapacityReport {
            required: 20,
            available: 10,
        });
        assert!(matches!(
            all_capacity.into_error(),
            StegoError::InsufficientCapacity {
                required: 20,
                available: 10
            }
        ));

        let mut prefix_over_capacity = FramedFailure::default();
        prefix_over_capacity.record_capacity(crate::CapacityReport {
            required: 20,
            available: 10,
        });
        prefix_over_capacity.record_prefix(StegoError::FrameNotFound);
        assert!(matches!(
            prefix_over_capacity.into_error(),
            StegoError::FrameNotFound
        ));
    }

    #[test]
    fn tiled_config_jpeg_rejects_small_and_non_multiple() {
        assert!(TileConfig::try_new(42, 0).is_err());
        assert!(embed_tiled(
            &make_test_jpeg(256, 256),
            b"x",
            &TileConfig::try_new(42, 7).unwrap()
        )
        .is_err());
        assert!(embed_tiled(
            &make_test_jpeg(256, 256),
            b"x",
            &TileConfig::try_new(42, 65).unwrap()
        )
        .is_err());
        assert!(TileConfig::try_new(42, 64).is_ok());
    }

    #[test]
    fn tiled_raw_roundtrip() {
        let jpeg_bytes = make_test_jpeg(256, 256);
        let payload = vec![0xA5; 36];
        let config = TileConfig::try_new(42, 64).unwrap();
        let report = embed_tiled(&jpeg_bytes, &payload, &config).unwrap();
        assert!(report.embedded);
        assert_eq!(report.actual_redundancy, 1);
        let recovered = extract_tiled(&report.output, payload.len(), &config, 64).unwrap();
        assert_eq!(recovered, payload);
    }

    #[test]
    fn tiled_framed_recovers_without_length() {
        let jpeg_bytes = make_test_jpeg(256, 256);
        let payload = b"framed tiled jpeg";
        let config = TileConfig::try_new(42, 64).unwrap();
        let report = embed_tiled_framed(&jpeg_bytes, payload, &config).unwrap();
        assert!(report.embedded);
        let recovered = extract_tiled_framed(&report.output, &config, 64).unwrap();
        assert_eq!(&recovered, payload);
    }

    #[test]
    fn tiled_extraction_rejects_zero_origins() {
        let jpeg_bytes = make_test_jpeg(256, 256);
        let config = TileConfig::try_new(42, 64).unwrap();
        assert!(extract_tiled(&jpeg_bytes, 4, &config, 0).is_err());
        assert!(extract_tiled_framed(&jpeg_bytes, &config, 0).is_err());
    }

    #[test]
    fn tiled_unsupported_jpeg_returns_structured_error() {
        use jpeg_encoder::Encoder as JpegEnc;
        let img = image::DynamicImage::new_rgb8(64, 64);
        let rgb = img.to_rgb8();
        let mut progressive_buf = Vec::new();
        {
            let mut enc = JpegEnc::new(&mut progressive_buf, 90);
            enc.set_progressive(true);
            enc.encode(rgb.as_raw(), 64, 64, jpeg_encoder::ColorType::Rgb)
                .unwrap();
        }
        let config = TileConfig::try_new(42, 64).unwrap();
        let result = embed_tiled(&progressive_buf, b"payload", &config);
        assert!(matches!(
            result,
            Err(StegoError::UnsupportedJpeg(_)) | Err(StegoError::MalformedInput(_))
        ));
    }

    #[test]
    fn tiled_framed_decodes_once_per_operation() {
        let jpeg_bytes = make_test_jpeg(256, 256);
        let config = TileConfig::try_new(42, 64).unwrap();
        let report = embed_tiled_framed(&jpeg_bytes, b"one", &config).unwrap();
        reset_decode_count();
        assert!(extract_tiled_framed(&report.output, &config, 64).is_ok());
        assert_eq!(decode_count(), 1);
    }
}
