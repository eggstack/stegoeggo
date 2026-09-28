use crate::error::{config_error, Result};
use napi::bindgen_prelude::BigInt;

const MAX_SAFE_INTEGER: f64 = 9_007_199_254_740_991.0;

fn reject(field: &str, detail: &str) -> crate::error::BindingError {
    config_error(format!("{field} {detail}"))
}

fn check_count(field: &str, value: f64) -> Result<()> {
    if !value.is_finite() {
        return Err(reject(field, "must be a finite number"));
    }
    if value.fract() != 0.0 {
        return Err(reject(field, "must be an integer"));
    }
    if value < 0.0 {
        return Err(reject(field, "must not be negative"));
    }
    if value > MAX_SAFE_INTEGER {
        return Err(reject(field, "exceeds Number.MAX_SAFE_INTEGER"));
    }
    Ok(())
}

/// Narrows a JavaScript `number` to `u32`, rejecting NaN, infinity, fractions,
/// negatives, and values above `Number.MAX_SAFE_INTEGER` first.
pub fn to_u32(field: &str, value: f64) -> Result<u32> {
    check_count(field, value)?;
    if value > f64::from(u32::MAX) {
        return Err(reject(field, "exceeds the u32 range"));
    }
    #[allow(clippy::cast_possible_truncation)]
    Ok(value as u32)
}

/// Narrows a JavaScript `number` to `u8`, rejecting NaN, infinity, fractions,
/// negatives, and out-of-range values first.
pub fn to_u8(field: &str, value: f64) -> Result<u8> {
    check_count(field, value)?;
    if value > f64::from(u8::MAX) {
        return Err(reject(field, "exceeds the u8 range"));
    }
    #[allow(clippy::cast_possible_truncation)]
    Ok(value as u8)
}

/// Narrows a JavaScript `number` to `usize`, rejecting NaN, infinity,
/// fractions, negatives, and values above `Number.MAX_SAFE_INTEGER` first.
pub fn to_usize(field: &str, value: f64) -> Result<usize> {
    check_count(field, value)?;
    if value > usize::MAX as f64 {
        return Err(reject(field, "exceeds the usize range"));
    }
    #[allow(clippy::cast_possible_truncation)]
    Ok(value as usize)
}

/// Narrows a JavaScript `number` to `f32` inside the closed unit interval.
pub fn to_unit_f32(field: &str, value: f64) -> Result<f32> {
    if !value.is_finite() {
        return Err(reject(field, "must be a finite number"));
    }
    if !(0.0..=1.0).contains(&value) {
        return Err(reject(field, "must be between 0 and 1 inclusive"));
    }
    #[allow(clippy::cast_possible_truncation)]
    Ok(value as f32)
}

/// Extracts a full-width `u64` from a JavaScript `bigint` without ever routing
/// the value through a JavaScript `number`.
pub fn to_u64(field: &str, value: &BigInt) -> Result<u64> {
    let (sign_bit, magnitude, lossless) = value.get_u64();
    if sign_bit || !lossless {
        return Err(reject(
            field,
            "must be a non-negative BigInt that fits in an unsigned 64-bit integer",
        ));
    }
    Ok(magnitude)
}

/// Projects a bounded Rust count into a JavaScript `number`.
#[must_use]
pub fn from_usize(value: usize) -> f64 {
    value as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_integers() {
        assert_eq!(to_u32("maxDimension", 4096.0).unwrap(), 4096);
        assert_eq!(to_u8("jpegQuality", 90.0).unwrap(), 90);
        assert_eq!(to_usize("stegoRedundancy", 3.0).unwrap(), 3);
    }

    #[test]
    fn rejects_nan_and_infinity() {
        assert!(to_u32("maxDimension", f64::NAN).is_err());
        assert!(to_u32("maxDimension", f64::INFINITY).is_err());
        assert!(to_u32("maxDimension", f64::NEG_INFINITY).is_err());
    }

    #[test]
    fn rejects_fractions() {
        assert!(to_u32("maxDimension", 4.5).is_err());
        assert!(to_usize("stegoRedundancy", 1.000_001).is_err());
    }

    #[test]
    fn rejects_negatives() {
        assert!(to_u32("maxDimension", -1.0).is_err());
        assert!(to_usize("stegoRedundancy", -1.0).is_err());
    }

    #[test]
    fn rejects_unsafe_integers() {
        assert!(to_usize("maxInputBytes", MAX_SAFE_INTEGER + 2.0).is_err());
    }

    #[test]
    fn rejects_out_of_range() {
        assert!(to_u32("maxDimension", 1.0e12).is_err());
        assert!(to_u8("jpegQuality", 300.0).is_err());
    }

    #[test]
    fn intensity_must_be_in_unit_range() {
        assert!(to_unit_f32("intensity", 0.5).is_ok());
        assert!(to_unit_f32("intensity", 0.0).is_ok());
        assert!(to_unit_f32("intensity", 1.0).is_ok());
        assert!(to_unit_f32("intensity", 1.5).is_err());
        assert!(to_unit_f32("intensity", -0.1).is_err());
        assert!(to_unit_f32("intensity", f64::NAN).is_err());
    }

    #[test]
    fn bigint_seed_keeps_full_width() {
        let seed = BigInt {
            sign_bit: false,
            words: vec![u64::MAX],
        };
        assert_eq!(to_u64("seed", &seed).unwrap(), u64::MAX);
    }

    #[test]
    fn bigint_seed_rejects_negative() {
        let seed = BigInt {
            sign_bit: true,
            words: vec![1],
        };
        assert!(to_u64("seed", &seed).is_err());
    }

    #[test]
    fn bigint_seed_rejects_oversized() {
        let seed = BigInt {
            sign_bit: false,
            words: vec![0, 1],
        };
        assert!(to_u64("seed", &seed).is_err());
    }
}
