//! Public pixel-domain LSB steganography carrier.
//!
//! Application-neutral embed/extract and capacity API on top of the
//! crate-internal mechanics in [`crate::lsb_internal`]. Low-level helpers
//! (permutations, slot mappings, byte/bit conversions) are intentionally
//! not re-exported; they are implementation details.

pub use crate::lsb_internal::{capacity, embed, extract, LsbConfig, DEFAULT_TILE_SIZE};
