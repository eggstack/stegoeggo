/// The embedding path used for steganographic payload insertion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbedPath {
    /// Non-tiled LSB pixel embedding (PNG/WebP).
    Lsb,
    /// Tiled LSB pixel embedding for crop resistance (PNG/WebP).
    LsbTiled,
    /// Non-tiled F5 DCT coefficient embedding (JPEG).
    DctF5,
    /// Tiled F5 DCT coefficient embedding for crop resistance (JPEG).
    DctF5Tiled,
    /// Q-table seed only — progressive JPEG fallback (no payload).
    QTableSeedOnly,
}

/// Structured outcome from steganographic embedding.
///
/// Returned by embedding helpers to report whether the payload was
/// actually embedded, skipped due to capacity, or degraded to a
/// seed-only path. Propagated through the pipeline to warnings,
/// reports, and strict CLI behavior.
#[derive(Debug, Clone)]
pub enum EmbedOutcome<T> {
    /// Payload was successfully embedded.
    Embedded {
        /// The carrier image with embedded payload.
        output: T,
        /// Payload size in bytes.
        payload_bytes: usize,
        /// Required capacity (in units appropriate to the embedding path).
        required_capacity: usize,
        /// Available capacity in the carrier.
        available_capacity: usize,
        /// The embedding path used.
        path: EmbedPath,
    },
    /// Payload was skipped due to insufficient carrier capacity.
    SkippedCapacity {
        /// The carrier image with Q-table/seed metadata (no payload embedded).
        output: T,
        /// Payload size in bytes.
        payload_bytes: usize,
        /// Required capacity (in units appropriate to the embedding path).
        required_capacity: usize,
        /// Available capacity in the carrier.
        available_capacity: usize,
        /// The embedding path that was attempted.
        path: EmbedPath,
    },
    /// Progressive JPEG — fell back to Q-table seed only (no payload).
    UnsupportedProgressive {
        /// The carrier image with Q-table seed metadata (no payload embedded).
        output: T,
    },
}

impl<T> EmbedOutcome<T> {
    /// Map the inner output type.
    pub fn map<U, F: FnOnce(T) -> U>(self, f: F) -> EmbedOutcome<U> {
        match self {
            EmbedOutcome::Embedded {
                output,
                payload_bytes,
                required_capacity,
                available_capacity,
                path,
            } => EmbedOutcome::Embedded {
                output: f(output),
                payload_bytes,
                required_capacity,
                available_capacity,
                path,
            },
            EmbedOutcome::SkippedCapacity {
                output,
                payload_bytes,
                required_capacity,
                available_capacity,
                path,
            } => EmbedOutcome::SkippedCapacity {
                output: f(output),
                payload_bytes,
                required_capacity,
                available_capacity,
                path,
            },
            EmbedOutcome::UnsupportedProgressive { output } => {
                EmbedOutcome::UnsupportedProgressive { output: f(output) }
            }
        }
    }

    /// Consume the outcome and return the inner output value.
    pub fn into_inner(self) -> T {
        match self {
            EmbedOutcome::Embedded { output, .. }
            | EmbedOutcome::SkippedCapacity { output, .. }
            | EmbedOutcome::UnsupportedProgressive { output } => output,
        }
    }

    /// Returns a reference to the inner output.
    #[must_use]
    pub fn output(&self) -> &T {
        match self {
            EmbedOutcome::Embedded { output, .. }
            | EmbedOutcome::SkippedCapacity { output, .. }
            | EmbedOutcome::UnsupportedProgressive { output } => output,
        }
    }

    /// Returns true if the payload was embedded.
    #[must_use]
    pub fn is_embedded(&self) -> bool {
        matches!(self, EmbedOutcome::Embedded { .. })
    }

    /// Returns true if capacity was skipped.
    #[must_use]
    pub fn is_skipped(&self) -> bool {
        matches!(self, EmbedOutcome::SkippedCapacity { .. })
    }

    /// Returns the required capacity for this embedding attempt.
    #[must_use]
    pub fn required_capacity(&self) -> usize {
        match self {
            EmbedOutcome::Embedded {
                required_capacity, ..
            }
            | EmbedOutcome::SkippedCapacity {
                required_capacity, ..
            } => *required_capacity,
            EmbedOutcome::UnsupportedProgressive { .. } => 0,
        }
    }

    /// Returns the available capacity for this embedding attempt.
    #[must_use]
    pub fn available_capacity(&self) -> usize {
        match self {
            EmbedOutcome::Embedded {
                available_capacity, ..
            }
            | EmbedOutcome::SkippedCapacity {
                available_capacity, ..
            } => *available_capacity,
            EmbedOutcome::UnsupportedProgressive { .. } => 0,
        }
    }

    /// Decompose into the output value and an [`EmbedOutcomeSummary`].
    #[must_use]
    pub fn into_parts(self) -> (T, EmbedOutcomeSummary) {
        let summary = match &self {
            EmbedOutcome::Embedded {
                payload_bytes,
                required_capacity,
                available_capacity,
                path,
                ..
            } => EmbedOutcomeSummary {
                status: EmbedStatus::Embedded,
                path: *path,
                payload_bytes: *payload_bytes,
                required_capacity: *required_capacity,
                available_capacity: *available_capacity,
            },
            EmbedOutcome::SkippedCapacity {
                payload_bytes,
                required_capacity,
                available_capacity,
                path,
                ..
            } => EmbedOutcomeSummary {
                status: EmbedStatus::SkippedCapacity,
                path: *path,
                payload_bytes: *payload_bytes,
                required_capacity: *required_capacity,
                available_capacity: *available_capacity,
            },
            EmbedOutcome::UnsupportedProgressive { .. } => EmbedOutcomeSummary {
                status: EmbedStatus::UnsupportedProgressive,
                path: EmbedPath::QTableSeedOnly,
                payload_bytes: 0,
                required_capacity: 0,
                available_capacity: 0,
            },
        };
        (self.into_inner(), summary)
    }
}

/// Status of a steganographic embedding attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbedStatus {
    /// Payload was successfully embedded.
    Embedded,
    /// Payload was skipped due to insufficient carrier capacity.
    SkippedCapacity,
    /// Progressive JPEG — fell back to Q-table seed only.
    UnsupportedProgressive,
}

/// Summary returned by an in-place LSB embedding operation.
///
/// Capacity fields are in **RGB carrier slots** (`width * height * 3`).
/// The alpha channel is never a carrier. `payload_bytes` is the raw
/// payload size in bytes placed in the carrier; for framed operations it
/// includes the frame header and CRC32 overhead.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InPlaceEmbedReport {
    /// Whether the payload was embedded.
    pub embedded: bool,
    /// Payload size in bytes placed in the carrier.
    pub payload_bytes: usize,
    /// Required capacity in RGB carrier slots.
    pub required_capacity: usize,
    /// Available capacity in RGB carrier slots.
    pub available_capacity: usize,
    /// The redundancy level used for embedding.
    pub actual_redundancy: usize,
}

impl std::fmt::Display for EmbedStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EmbedStatus::Embedded => write!(f, "embedded"),
            EmbedStatus::SkippedCapacity => write!(f, "skipped_capacity"),
            EmbedStatus::UnsupportedProgressive => write!(f, "unsupported_progressive"),
        }
    }
}

/// Summary of a steganographic embedding attempt.
///
/// Carries the actual outcome of embedding — status, path, payload size,
/// and capacity figures — for use in reports and CLI output.
#[derive(Debug, Clone)]
pub struct EmbedOutcomeSummary {
    /// Whether the payload was embedded, skipped, or degraded.
    pub status: EmbedStatus,
    /// The embedding path used.
    pub path: EmbedPath,
    /// Payload size in bytes.
    pub payload_bytes: usize,
    /// Required capacity for the carrier (RGB slots for LSB paths, non-zero AC
    /// coefficients for DCT paths).
    pub required_capacity: usize,
    /// Available capacity in the carrier (same units as `required_capacity`).
    pub available_capacity: usize,
}

impl EmbedOutcomeSummary {
    /// Whether the payload was actually embedded.
    #[must_use]
    pub fn is_embedded(&self) -> bool {
        self.status == EmbedStatus::Embedded
    }
}

/// Hard maximum for bounded tiled recovery.
///
/// Tiled extraction enumerates at most this many tile origins per operation.
/// Callers pass an explicit `max_origins` in `1..=MAX_TILED_ORIGINS`; larger
/// values are rejected with [`crate::StegoError::InvalidConfig`].
pub const MAX_TILED_ORIGINS: u32 = 4096;

/// Shared application-neutral configuration for tiled carrier operations.
///
/// Both LSB and JPEG tiled operations use this type so callers do not need
/// separate nearly-identical structs. It carries only carrier-domain state:
/// the master seed tile coordinates are derived from, and the tile edge
/// length in pixels.
///
/// Use [`TileConfig::try_new`] when the tile size comes from untrusted input.
/// Generic validation requires `tile_size > 0` with checked geometry; JPEG
/// tiled operations additionally require `tile_size >= 8` and a multiple of
/// 8 so the size maps deterministically to DCT blocks.
///
/// # Examples
///
/// ```rust
/// use stegoeggo_stego::TileConfig;
///
/// let config = TileConfig::try_new(42, 64)?;
/// assert_eq!(config.seed(), 42);
/// assert_eq!(config.tile_size(), 64);
/// # Ok::<_, stegoeggo_stego::StegoError>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TileConfig {
    seed: u64,
    tile_size: u32,
}

impl TileConfig {
    /// Fallible constructor that validates the tile size up front.
    ///
    /// Returns [`crate::StegoError::InvalidConfig`] if `tile_size` is zero.
    pub fn try_new(seed: u64, tile_size: u32) -> Result<Self, crate::StegoError> {
        if tile_size == 0 {
            return Err(crate::StegoError::InvalidConfig(
                "tile size must be non-zero".to_string(),
            ));
        }
        Ok(Self { seed, tile_size })
    }

    /// The master seed tile coordinates are derived from.
    #[must_use]
    pub fn seed(&self) -> u64 {
        self.seed
    }

    /// The tile edge length in pixels.
    #[must_use]
    pub fn tile_size(&self) -> u32 {
        self.tile_size
    }
}

/// Validate an explicit tiled-extraction bound.
///
/// Rejects `0` and values above [`MAX_TILED_ORIGINS`] with
/// [`crate::StegoError::InvalidConfig`]. Keeps `ResourceLimits` types out of
/// the carrier API while guaranteeing every tiled recovery is bounded.
pub(crate) fn validate_max_origins(max_origins: u32) -> Result<(), crate::StegoError> {
    if max_origins == 0 || max_origins > MAX_TILED_ORIGINS {
        return Err(crate::StegoError::InvalidConfig(format!(
            "max_origins must be in 1..={MAX_TILED_ORIGINS}, got {max_origins}"
        )));
    }
    Ok(())
}
