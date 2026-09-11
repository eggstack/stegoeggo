use crate::constants::{SPLITMIX64_SEED, STEGO_SPREAD_FACTOR};
use crate::types::{EmbedOutcome, InPlaceEmbedReport};
use image::{Rgba, RgbaImage};

/// Default tile size for tiled steganographic embedding (64×64 pixels).
pub const DEFAULT_TILE_SIZE: u32 = 64;

/// Minimum tile size for tiled steganographic embedding (reserved).
#[allow(dead_code)]
pub const MIN_TILE_SIZE: u32 = 32;

/// Derive a per-tile seed from a master seed and tile grid coordinates.
///
/// Uses splitmix64 mixing to produce a deterministic, independent seed
/// for each tile position.
pub fn tile_seed(master_seed: u64, tile_x: u32, tile_y: u32) -> u64 {
    let mut z = master_seed;
    z ^= (tile_x as u64).wrapping_mul(0x9E3779B97F4A7C15);
    z ^= (tile_y as u64).wrapping_mul(0xBF58476D1CE4E5B9);
    splitmix64(z)
}

#[inline(always)]
pub fn splitmix64(x: u64) -> u64 {
    let mut z = x.wrapping_add(SPLITMIX64_SEED);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
    z ^ (z >> 31)
}

#[inline(always)]
pub fn stego_permutation(index: usize, total_pixels: usize, seed: u64) -> usize {
    if total_pixels == 0 {
        return 0;
    }
    let a = splitmix64(seed).wrapping_mul(2) | 1;
    let b = splitmix64(seed.wrapping_add(0x9e3779b97f4a7c15));
    a.wrapping_mul(index as u64).wrapping_add(b) as usize % total_pixels
}

/// Cycle-walking permutation over `0..slot_count`.
///
/// Maps `index` through a linear congruential step modulo the next power
/// of two `m`, then walks the cycle until the value falls inside
/// `0..slot_count`. The walk is bounded at 256 steps; beyond that the
/// construction falls back to `splitmix64(x) % slot_count`. The fallback
/// is not proven bijective for arbitrary `a,b,m` but guarantees termination.
/// Exhaustive injectivity is verified by tests for the documented small and
/// medium domains only; the mapping is byte-frozen (V2) for compatibility
/// and must not be assumed bijective outside the tested domains. Chi-squared
/// uniformity is checked in tests for several `(seed, slot_count)` pairs.
/// Accepted domain is exactly the carrier domain: `slot_count` always comes
/// from checked `lsb_available_slots` over `u32` dimensions (`None` on
/// overflow), and this function additionally returns `None` when
/// `checked_next_power_of_two` overflows. No larger abstract domain is
/// claimed. This construction is not NIST SP 800-38G FPE and makes no
/// cryptographic-security claim.
#[inline(always)]
pub fn stego_permutation_v2(index: usize, slot_count: usize, seed: u64) -> Option<usize> {
    permutation_v2_core(index, slot_count, seed).map(|(slot, _, _)| slot)
}

#[inline(always)]
fn permutation_v2_core(index: usize, slot_count: usize, seed: u64) -> Option<(usize, usize, bool)> {
    if slot_count <= 1 {
        if slot_count == 0 {
            return None;
        }
        if index == 0 {
            return Some((0, 0, false));
        }
        return None;
    }

    let m = slot_count.checked_next_power_of_two()?;

    let a = splitmix64(seed).wrapping_mul(2) | 1;
    let b = splitmix64(seed.wrapping_add(SPLITMIX64_SEED));

    let mut x = (a.wrapping_mul(index as u64).wrapping_add(b)) % (m as u64);
    let mut attempts = 0usize;
    while (x as usize) >= slot_count {
        x = (a.wrapping_mul(x).wrapping_add(b)) % (m as u64);
        attempts += 1;
        if attempts >= 256 {
            return Some(((splitmix64(x) % slot_count as u64) as usize, attempts, true));
        }
    }
    Some((x as usize, attempts, false))
}

#[cfg(test)]
pub(crate) fn stego_permutation_v2_debug(
    index: usize,
    slot_count: usize,
    seed: u64,
) -> Option<(usize, usize, bool)> {
    permutation_v2_core(index, slot_count, seed)
}

#[inline(always)]
pub fn carrier_v2_slot_to_pixel_channel(
    slot: usize,
    width: u32,
    height: u32,
) -> Option<(usize, usize)> {
    let total_pixels = (width as usize).checked_mul(height as usize)?;
    let pixel_index = slot / 3;
    let channel = slot % 3;
    if pixel_index >= total_pixels {
        return None;
    }
    Some((pixel_index, channel))
}

/// Returns the total number of independent pixel-channel carrier slots
/// available in an image of the given dimensions.
///
#[inline(always)]
pub fn lsb_available_slots(width: u32, height: u32) -> Option<usize> {
    (width as usize)
        .checked_mul(height as usize)
        .and_then(|p| p.checked_mul(3))
}

pub(crate) fn checked_lsb_available_slots(
    width: u32,
    height: u32,
) -> Result<usize, super::StegoError> {
    lsb_available_slots(width, height).ok_or_else(|| {
        super::StegoError::ResourceLimitExceeded("carrier dimensions overflow".into())
    })
}

/// Required carrier slots for a V2 payload under the exact capacity model.
///
/// Each payload bit consumes `STEGO_SPREAD_FACTOR * redundancy` logical
/// slots. Exactness (no inter-replica collision) is verified for the
/// documented small/medium domains; outside those domains it is an
/// operational assumption, not a proven invariant. The mapping itself is
/// byte-frozen for compatibility.
#[inline(always)]
pub fn lsb_required_capacity_v2(payload_bits: usize, redundancy: usize) -> usize {
    payload_bits
        .checked_mul(STEGO_SPREAD_FACTOR)
        .and_then(|r| r.checked_mul(redundancy))
        .unwrap_or(usize::MAX)
}

#[inline(always)]
pub fn lsb_required_slots_legacy(payload_bits: usize) -> usize {
    payload_bits.saturating_mul(STEGO_SPREAD_FACTOR)
}

#[allow(dead_code)]
pub fn lsb_capacity_for_image(
    width: u32,
    height: u32,
    payload_bits: usize,
    redundancy: usize,
) -> (usize, Option<usize>) {
    (
        lsb_required_capacity_v2(payload_bits, redundancy),
        lsb_available_slots(width, height),
    )
}

pub fn bytes_to_bits(bytes: &[u8]) -> Vec<u8> {
    let mut bits = Vec::with_capacity(bytes.len() * 8);
    for byte in bytes {
        for i in 0..8 {
            bits.push((byte >> i) & 1);
        }
    }
    bits
}

#[inline]
fn payload_bit(payload: &[u8], bit_index: usize) -> u8 {
    (payload[bit_index / 8] >> (bit_index % 8)) & 1
}

pub fn bits_to_bytes(bits: &[u8]) -> Vec<u8> {
    if !bits.len().is_multiple_of(8) {
        return Vec::new();
    }
    let mut bytes = Vec::with_capacity(bits.len() / 8);
    for chunk in bits.chunks_exact(8) {
        let mut byte = 0u8;
        for (i, &bit) in chunk.iter().enumerate() {
            byte |= bit << i;
        }
        bytes.push(byte);
    }
    bytes
}

pub fn embed_bit_in_pixel(output: &mut RgbaImage, x: u32, y: u32, channel: usize, bit: u8) {
    let pixel = output.get_pixel(x, y);
    let old_val = pixel[channel];

    if (old_val & 1) == bit {
        return;
    }

    output.get_pixel_mut(x, y)[channel] = apply_lsb_bit(old_val, x, y);
}

#[inline]
pub(crate) fn apply_lsb_bit(old_val: u8, x: u32, y: u32) -> u8 {
    if old_val == 0 {
        1
    } else if old_val == 255 {
        254
    } else {
        let direction_hash = x.wrapping_mul(31).wrapping_add(y.wrapping_mul(17));
        if direction_hash & 1 == 0 {
            old_val + 1
        } else {
            old_val - 1
        }
    }
}

/// Private carrier access for the shared LSB V2 mutation/extraction core.
///
/// Implemented by `RgbaImage`, borrowed pixel views, and tile windows.
/// Coordinates are logical carrier coordinates; implementations map them
/// to physical storage. Out-of-range coordinates return `None` and fail
/// the operation gracefully instead of panicking.
pub(crate) trait PixelCarrier {
    fn carrier_width(&self) -> u32;
    fn carrier_height(&self) -> u32;
    fn read_channel(&self, x: u32, y: u32, channel: usize) -> Option<u8>;
}

/// Private mutable carrier access for the shared LSB V2 mutation core.
pub(crate) trait PixelCarrierMut: PixelCarrier {
    fn write_channel_bit(&mut self, x: u32, y: u32, channel: usize, bit: u8) -> Option<()>;
}

impl PixelCarrier for RgbaImage {
    fn carrier_width(&self) -> u32 {
        self.width()
    }

    fn carrier_height(&self) -> u32 {
        self.height()
    }

    fn read_channel(&self, x: u32, y: u32, channel: usize) -> Option<u8> {
        if x >= self.width() || y >= self.height() || channel >= 3 {
            return None;
        }
        Some(self.get_pixel(x, y)[channel])
    }
}

impl PixelCarrierMut for RgbaImage {
    fn write_channel_bit(&mut self, x: u32, y: u32, channel: usize, bit: u8) -> Option<()> {
        if x >= self.width() || y >= self.height() || channel >= 3 {
            return None;
        }
        let old_val = self.get_pixel(x, y)[channel];
        if (old_val & 1) != bit {
            self.get_pixel_mut(x, y)[channel] = apply_lsb_bit(old_val, x, y);
        }
        Some(())
    }
}

/// Private immutable tile window over a carrier, avoiding tile-copy allocation.
pub(crate) struct TileWindow<'a, C: PixelCarrier + ?Sized> {
    inner: &'a C,
    x0: u32,
    y0: u32,
    width: u32,
    height: u32,
}

impl<'a, C: PixelCarrier + ?Sized> TileWindow<'a, C> {
    pub(crate) fn new(inner: &'a C, x0: u32, y0: u32, width: u32, height: u32) -> Self {
        Self {
            inner,
            x0,
            y0,
            width,
            height,
        }
    }
}

impl<C: PixelCarrier + ?Sized> PixelCarrier for TileWindow<'_, C> {
    fn carrier_width(&self) -> u32 {
        self.width
    }

    fn carrier_height(&self) -> u32 {
        self.height
    }

    fn read_channel(&self, x: u32, y: u32, channel: usize) -> Option<u8> {
        self.inner.read_channel(self.x0 + x, self.y0 + y, channel)
    }
}

/// Private mutable tile window over a carrier, avoiding tile-copy allocation.
pub(crate) struct TileWindowMut<'a, C: PixelCarrierMut + ?Sized> {
    inner: &'a mut C,
    x0: u32,
    y0: u32,
    width: u32,
    height: u32,
}

impl<'a, C: PixelCarrierMut + ?Sized> TileWindowMut<'a, C> {
    pub(crate) fn new(inner: &'a mut C, x0: u32, y0: u32, width: u32, height: u32) -> Self {
        Self {
            inner,
            x0,
            y0,
            width,
            height,
        }
    }
}

impl<C: PixelCarrierMut + ?Sized> PixelCarrier for TileWindowMut<'_, C> {
    fn carrier_width(&self) -> u32 {
        self.width
    }

    fn carrier_height(&self) -> u32 {
        self.height
    }

    fn read_channel(&self, x: u32, y: u32, channel: usize) -> Option<u8> {
        self.inner.read_channel(self.x0 + x, self.y0 + y, channel)
    }
}

impl<C: PixelCarrierMut + ?Sized> PixelCarrierMut for TileWindowMut<'_, C> {
    fn write_channel_bit(&mut self, x: u32, y: u32, channel: usize, bit: u8) -> Option<()> {
        self.inner
            .write_channel_bit(self.x0 + x, self.y0 + y, channel, bit)
    }
}

#[allow(dead_code)]
pub fn embed_lsb(img: &RgbaImage, payload: &[u8], seed: u64) -> EmbedOutcome<RgbaImage> {
    let (width, height) = img.dimensions();
    let mut output = img.clone();

    let payload_bits = bytes_to_bits(payload);

    let Some(total_pixels) = (width as usize).checked_mul(height as usize) else {
        return EmbedOutcome::SkippedCapacity {
            output,
            payload_bytes: payload.len(),
            required_capacity: lsb_required_slots_legacy(payload_bits.len()),
            available_capacity: 0,
            path: crate::types::EmbedPath::Lsb,
        };
    };
    let total_pixels_needed = payload_bits
        .len()
        .saturating_mul(STEGO_SPREAD_FACTOR)
        .div_ceil(3);
    let Some(available_slots) = lsb_available_slots(width, height) else {
        return EmbedOutcome::SkippedCapacity {
            output,
            payload_bytes: payload.len(),
            required_capacity: usize::MAX,
            available_capacity: 0,
            path: crate::types::EmbedPath::Lsb,
        };
    };
    let required_slots = lsb_required_slots_legacy(payload_bits.len());

    if total_pixels_needed > total_pixels {
        return EmbedOutcome::SkippedCapacity {
            output,
            payload_bytes: payload.len(),
            required_capacity: required_slots,
            available_capacity: available_slots,
            path: crate::types::EmbedPath::Lsb,
        };
    }

    for (i, &bit) in payload_bits.iter().enumerate() {
        let channel = i % 3;
        for s in 0..STEGO_SPREAD_FACTOR {
            let logical = i * STEGO_SPREAD_FACTOR + s;
            let idx = stego_permutation(logical, total_pixels, seed);

            let x = idx as u32 % width;
            let y = idx as u32 / width;

            embed_bit_in_pixel(&mut output, x, y, channel, bit);
        }
    }

    EmbedOutcome::Embedded {
        output,
        payload_bytes: payload.len(),
        required_capacity: required_slots,
        available_capacity: available_slots,
        path: crate::types::EmbedPath::Lsb,
    }
}

pub fn extract_lsb(img: &RgbaImage, expected_bits: usize, seed: u64) -> Option<Vec<u8>> {
    let (width, height) = img.dimensions();
    let total_pixels = (width as usize).checked_mul(height as usize)?;
    let available = lsb_available_slots(width, height)?;

    if expected_bits.checked_mul(STEGO_SPREAD_FACTOR)? > available {
        return None;
    }

    let mut bits = Vec::with_capacity(expected_bits);
    let threshold = (STEGO_SPREAD_FACTOR / 2) as u32;

    for i in 0..expected_bits {
        let channel = i % 3;
        let mut ones = 0u32;

        for s in 0..STEGO_SPREAD_FACTOR {
            let logical = i * STEGO_SPREAD_FACTOR + s;
            let idx = stego_permutation(logical, total_pixels, seed);

            let x = idx as u32 % width;
            let y = idx as u32 / width;
            let pixel = img.get_pixel(x, y);

            let bit = match channel {
                0 => pixel[0] & 1,
                1 => pixel[1] & 1,
                _ => pixel[2] & 1,
            };
            ones += bit as u32;
        }

        bits.push(if ones > threshold { 1 } else { 0 });
    }

    Some(bits_to_bytes(&bits))
}

#[allow(dead_code)]
pub fn extract_lsb_range(
    img: &RgbaImage,
    expected_bits: usize,
    offset: usize,
    count: usize,
    seed: u64,
) -> Option<Vec<u8>> {
    let (width, height) = img.dimensions();
    let total_pixels = (width as usize).checked_mul(height as usize)?;
    let available = lsb_available_slots(width, height)?;

    if expected_bits.checked_mul(STEGO_SPREAD_FACTOR)? > available {
        return None;
    }
    if offset + count > expected_bits {
        return None;
    }

    let mut bits = Vec::with_capacity(count);
    let threshold = (STEGO_SPREAD_FACTOR / 2) as u32;

    for i in offset..offset + count {
        let channel = i % 3;
        let mut ones = 0u32;

        for s in 0..STEGO_SPREAD_FACTOR {
            let logical = i * STEGO_SPREAD_FACTOR + s;
            let idx = stego_permutation(logical, total_pixels, seed);

            let x = idx as u32 % width;
            let y = idx as u32 / width;
            let pixel = img.get_pixel(x, y);

            let bit = match channel {
                0 => pixel[0] & 1,
                1 => pixel[1] & 1,
                _ => pixel[2] & 1,
            };
            ones += bit as u32;
        }

        bits.push(if ones > threshold { 1 } else { 0 });
    }

    Some(bits_to_bytes(&bits))
}

pub fn embed_lsb_v2_in_place(
    image: &mut RgbaImage,
    payload: &[u8],
    seed: u64,
    redundancy: usize,
) -> InPlaceEmbedReport {
    embed_v2_in_place_carrier(image, payload, seed, redundancy)
}

pub(crate) fn embed_v2_in_place_carrier<C: PixelCarrierMut>(
    carrier: &mut C,
    payload: &[u8],
    seed: u64,
    redundancy: usize,
) -> InPlaceEmbedReport {
    if !(1..=10).contains(&redundancy) {
        let (width, height) = (carrier.carrier_width(), carrier.carrier_height());
        return InPlaceEmbedReport {
            embedded: false,
            payload_bytes: payload.len(),
            required_capacity: usize::MAX,
            available_capacity: lsb_available_slots(width, height).unwrap_or(0),
            actual_redundancy: redundancy,
        };
    }
    let (width, height) = (carrier.carrier_width(), carrier.carrier_height());
    let Some(available) = lsb_available_slots(width, height) else {
        return InPlaceEmbedReport {
            embedded: false,
            payload_bytes: payload.len(),
            required_capacity: usize::MAX,
            available_capacity: 0,
            actual_redundancy: redundancy,
        };
    };
    let Some(bit_len) = payload.len().checked_mul(8) else {
        return InPlaceEmbedReport {
            embedded: false,
            payload_bytes: payload.len(),
            required_capacity: usize::MAX,
            available_capacity: available,
            actual_redundancy: redundancy,
        };
    };
    let required = lsb_required_capacity_v2(bit_len, redundancy);

    if required > available {
        return InPlaceEmbedReport {
            embedded: false,
            payload_bytes: payload.len(),
            required_capacity: required,
            available_capacity: available,
            actual_redundancy: redundancy,
        };
    }

    let Some(replicas_per_bit) = STEGO_SPREAD_FACTOR.checked_mul(redundancy) else {
        return InPlaceEmbedReport {
            embedded: false,
            payload_bytes: payload.len(),
            required_capacity: required,
            available_capacity: available,
            actual_redundancy: redundancy,
        };
    };
    for bit_index in 0..bit_len {
        let bit = payload_bit(payload, bit_index);
        for replica in 0..replicas_per_bit {
            debug_assert!(bit_index.checked_mul(replicas_per_bit).is_some());
            debug_assert!(bit_index
                .checked_mul(replicas_per_bit)
                .and_then(|v| v.checked_add(replica))
                .is_some());
            let logical = bit_index * replicas_per_bit + replica;
            let slot = stego_permutation_v2(logical, available, seed);
            let pixel_channel =
                slot.and_then(|slot| carrier_v2_slot_to_pixel_channel(slot, width, height));
            let Some((pixel_index, slot_channel)) = pixel_channel else {
                return InPlaceEmbedReport {
                    embedded: false,
                    payload_bytes: payload.len(),
                    required_capacity: required,
                    available_capacity: available,
                    actual_redundancy: redundancy,
                };
            };
            let x = pixel_index as u32 % width;
            let y = pixel_index as u32 / width;
            if carrier.write_channel_bit(x, y, slot_channel, bit).is_none() {
                return InPlaceEmbedReport {
                    embedded: false,
                    payload_bytes: payload.len(),
                    required_capacity: required,
                    available_capacity: available,
                    actual_redundancy: redundancy,
                };
            }
        }
    }

    InPlaceEmbedReport {
        embedded: true,
        payload_bytes: payload.len(),
        required_capacity: required,
        available_capacity: available,
        actual_redundancy: redundancy,
    }
}

/// Extract LSB V2 payload with majority voting.
///
/// For even `redundancy` values (`replicas_per_bit` even), a tied vote
/// (`ones * 2 == replicas_per_bit`) is treated as extraction failure and
/// returns `None` (fail-closed). This matches the F5 DCT path
/// ([`crate::jpeg_transcoder::DctStegoF5::extract_f5`]) which returns an
/// empty vector on even-redundancy ties to signal ambiguity. Both carriers
/// treat a tie as tamper-detected; the carrier-level public helpers
/// (`stego::lsb::extract`, `stego::jpeg::extract`) normalize the two
/// sentinels to `StegoError::MalformedInput`, and the application
/// verification pipeline maps them to `NotFound` (indistinguishable from
/// absence) for the high-level `VerificationReport` — callers that need
/// explicit tamper evidence should prefer odd redundancy (1,3,5…) or
/// inspect the carrier error directly.
pub fn extract_lsb_v2(
    img: &RgbaImage,
    expected_bits: usize,
    seed: u64,
    redundancy: usize,
) -> Option<Vec<u8>> {
    extract_v2_carrier(img, expected_bits, seed, redundancy)
}

pub(crate) fn extract_v2_carrier<C: PixelCarrier>(
    carrier: &C,
    expected_bits: usize,
    seed: u64,
    redundancy: usize,
) -> Option<Vec<u8>> {
    let (width, height) = (carrier.carrier_width(), carrier.carrier_height());
    let available = lsb_available_slots(width, height)?;
    let replicas_per_bit = STEGO_SPREAD_FACTOR.checked_mul(redundancy)?;

    let required_slots = expected_bits.checked_mul(replicas_per_bit)?;
    if required_slots > available {
        return None;
    }

    if !expected_bits.is_multiple_of(8) {
        return None;
    }

    let mut bytes = vec![0u8; expected_bits / 8];
    let threshold = (replicas_per_bit / 2) as u32;

    for i in 0..expected_bits {
        let mut ones = 0u32;

        for s in 0..replicas_per_bit {
            let logical = i.checked_mul(replicas_per_bit)?.checked_add(s)?;
            let slot = stego_permutation_v2(logical, available, seed)?;
            let (pixel_index, slot_channel) =
                carrier_v2_slot_to_pixel_channel(slot, width, height)?;
            let x = pixel_index as u32 % width;
            let y = pixel_index as u32 / width;
            ones += u32::from(carrier.read_channel(x, y, slot_channel)? & 1);
        }

        if ones * 2 == replicas_per_bit as u32 {
            return None;
        }
        if ones > threshold {
            bytes[i / 8] |= 1 << (i % 8);
        }
    }

    Some(bytes)
}

pub fn embed_lsb_tiled(
    img: &RgbaImage,
    payload: &[u8],
    master_seed: u64,
    tile_size: u32,
) -> EmbedOutcome<RgbaImage> {
    let mut output = img.clone();
    let report = embed_lsb_tiled_in_place(&mut output, payload, master_seed, tile_size);
    if report.embedded {
        EmbedOutcome::Embedded {
            output,
            payload_bytes: report.payload_bytes,
            required_capacity: report.required_capacity,
            available_capacity: report.available_capacity,
            path: crate::types::EmbedPath::LsbTiled,
        }
    } else {
        EmbedOutcome::SkippedCapacity {
            output,
            payload_bytes: report.payload_bytes,
            required_capacity: report.required_capacity,
            available_capacity: report.available_capacity,
            path: crate::types::EmbedPath::LsbTiled,
        }
    }
}

struct TiledEmbedPlan {
    x0: u32,
    y0: u32,
    sub_w: u32,
    sub_h: u32,
    seed_for_embed: u64,
    tile_available: usize,
}

struct TiledTileScan {
    tile_required: usize,
    tile_available: usize,
    embed: Option<TiledEmbedPlan>,
}

pub fn embed_lsb_tiled_in_place(
    image: &mut RgbaImage,
    payload: &[u8],
    master_seed: u64,
    tile_size: u32,
) -> InPlaceEmbedReport {
    embed_tiled_carrier(image, payload, master_seed, tile_size)
}

pub(crate) fn embed_tiled_carrier<C: PixelCarrierMut>(
    carrier: &mut C,
    payload: &[u8],
    master_seed: u64,
    tile_size: u32,
) -> InPlaceEmbedReport {
    const TILED_REDUNDANCY: usize = 1;
    let (width, height) = (carrier.carrier_width(), carrier.carrier_height());
    if tile_size == 0 || width < tile_size || height < tile_size {
        return InPlaceEmbedReport {
            embedded: false,
            payload_bytes: payload.len(),
            required_capacity: 0,
            available_capacity: 0,
            actual_redundancy: TILED_REDUNDANCY,
        };
    }

    let Some(bit_len) = payload.len().checked_mul(8) else {
        return InPlaceEmbedReport {
            embedded: false,
            payload_bytes: payload.len(),
            required_capacity: usize::MAX,
            available_capacity: 0,
            actual_redundancy: TILED_REDUNDANCY,
        };
    };

    let mut scans = Vec::new();
    let mut total_required = 0usize;
    let mut total_available = 0usize;

    let mut tile_y: u32 = 0;
    while tile_y * tile_size < height {
        let y0 = tile_y * tile_size;

        let mut tile_x: u32 = 0;
        while tile_x * tile_size < width {
            let x0 = tile_x * tile_size;
            let x1 = (x0 + tile_size).min(width);
            let y1 = (y0 + tile_size).min(height);
            let sub_w = x1 - x0;
            let sub_h = y1 - y0;

            if sub_w < tile_size || sub_h < tile_size {
                tile_x += 1;
                continue;
            }

            let local_seed = tile_seed(master_seed, tile_x, tile_y);
            let Some(tile_available) = lsb_available_slots(sub_w, sub_h) else {
                return InPlaceEmbedReport {
                    embedded: false,
                    payload_bytes: payload.len(),
                    required_capacity: usize::MAX,
                    available_capacity: total_available,
                    actual_redundancy: TILED_REDUNDANCY,
                };
            };
            let tile_required = lsb_required_capacity_v2(bit_len, TILED_REDUNDANCY);

            let embed = (tile_available >= tile_required && bit_len > 0).then(|| TiledEmbedPlan {
                x0,
                y0,
                sub_w,
                sub_h,
                seed_for_embed: local_seed.wrapping_mul(crate::constants::STEGO_OFFSET_SEED_1),
                tile_available,
            });
            scans.push(TiledTileScan {
                tile_required,
                tile_available,
                embed,
            });

            total_required = total_required.saturating_add(tile_required);
            total_available = total_available.saturating_add(tile_available);
            tile_x += 1;
        }
        tile_y += 1;
    }

    if scans.iter().all(|scan| scan.embed.is_none()) {
        return InPlaceEmbedReport {
            embedded: false,
            payload_bytes: payload.len(),
            required_capacity: total_required,
            available_capacity: total_available,
            actual_redundancy: TILED_REDUNDANCY,
        };
    }

    let replicas_per_bit = STEGO_SPREAD_FACTOR;
    let mut run_required = 0usize;
    let mut run_available = 0usize;
    for scan in &scans {
        run_required = run_required.saturating_add(scan.tile_required);
        run_available = run_available.saturating_add(scan.tile_available);
        let Some(plan) = &scan.embed else {
            continue;
        };
        let mut window = TileWindowMut::new(carrier, plan.x0, plan.y0, plan.sub_w, plan.sub_h);
        for i in 0..bit_len {
            let bit = payload_bit(payload, i);
            for s in 0..replicas_per_bit {
                let logical = i
                    .checked_mul(replicas_per_bit)
                    .and_then(|v| v.checked_add(s));
                let slot = logical.and_then(|logical| {
                    stego_permutation_v2(logical, plan.tile_available, plan.seed_for_embed)
                });
                let pixel_channel = slot.and_then(|slot| {
                    carrier_v2_slot_to_pixel_channel(slot, plan.sub_w, plan.sub_h)
                });
                let Some((pixel_index, slot_channel)) = pixel_channel else {
                    return InPlaceEmbedReport {
                        embedded: false,
                        payload_bytes: payload.len(),
                        required_capacity: run_required,
                        available_capacity: run_available,
                        actual_redundancy: TILED_REDUNDANCY,
                    };
                };
                let lx = pixel_index as u32 % plan.sub_w;
                let ly = pixel_index as u32 / plan.sub_w;
                if window
                    .write_channel_bit(lx, ly, slot_channel, bit)
                    .is_none()
                {
                    return InPlaceEmbedReport {
                        embedded: false,
                        payload_bytes: payload.len(),
                        required_capacity: run_required,
                        available_capacity: run_available,
                        actual_redundancy: TILED_REDUNDANCY,
                    };
                }
            }
        }
    }

    InPlaceEmbedReport {
        embedded: true,
        payload_bytes: payload.len(),
        required_capacity: total_required,
        available_capacity: total_available,
        actual_redundancy: TILED_REDUNDANCY,
    }
}

/// Maximum tile-grid coordinate probed per axis during tiled recovery.
///
/// Matches the application tiled search domain so generic framed recovery
/// remains compatible with payloads embedded by the current tiled path.
pub(crate) const TILED_MAX_GRID: u32 = 16;

/// Enumerate bounded crop-origin candidates for tiled recovery.
///
/// Origins step by `tile_size / 2` (minimum 1) in row-major order until
/// `max_origins` is reached. Uses saturating arithmetic so untrusted tile
/// sizes cannot overflow the scan.
pub(crate) fn tiled_origins(
    width: u32,
    height: u32,
    tile_size: u32,
    max_origins: u32,
) -> Vec<(u32, u32)> {
    let stride = (tile_size / 2).max(1);
    let mut origins = Vec::new();
    let mut y = 0u32;
    while y.saturating_add(tile_size) <= height {
        let mut x = 0u32;
        while x.saturating_add(tile_size) <= width {
            origins.push((x, y));
            if origins.len() as u32 >= max_origins {
                return origins;
            }
            x = x.saturating_add(stride);
            if x == u32::MAX {
                break;
            }
        }
        if origins.len() as u32 >= max_origins {
            break;
        }
        y = y.saturating_add(stride);
        if y == u32::MAX {
            break;
        }
    }
    origins
}

/// Derive the carrier seed for one tiled candidate.
///
/// Mirrors the tiled embed seed derivation (`tile_seed` mixed per tile
/// coordinate, then multiplied by the offset constant with a 5-pass history
/// for compatibility with payloads embedded by the current path).
pub(crate) fn tiled_candidate_seed(master_seed: u64, tile_x: u32, tile_y: u32, pass: u32) -> u64 {
    let local = tile_seed(master_seed, tile_x, tile_y);
    local.wrapping_mul(crate::constants::STEGO_OFFSET_SEED_1.wrapping_add(pass as u64))
}

/// Shared bounded tiled raw-extraction core over any pixel carrier.
///
/// Reads tile windows in place without allocating cropped copies. Returns
/// the first candidate in deterministic scan order.
pub(crate) fn extract_tiled_carrier<C: PixelCarrier>(
    carrier: &C,
    payload_len: usize,
    seed: u64,
    tile_size: u32,
    max_origins: u32,
) -> Result<Vec<u8>, super::StegoError> {
    crate::types::validate_max_origins(max_origins)?;
    let payload_bits = payload_len.checked_mul(8).ok_or_else(|| {
        super::StegoError::ResourceLimitExceeded("payload length overflow".to_string())
    })?;
    let (width, height) = (carrier.carrier_width(), carrier.carrier_height());
    if width < tile_size || height < tile_size {
        let required = lsb_required_capacity_v2(payload_bits, 1);
        return Err(super::StegoError::InsufficientCapacity {
            required,
            available: 0,
        });
    }
    let origins = tiled_origins(width, height, tile_size, max_origins);
    if origins.is_empty() {
        let required = lsb_required_capacity_v2(payload_bits, 1);
        return Err(super::StegoError::InsufficientCapacity {
            required,
            available: 0,
        });
    }
    for (x0, y0) in origins {
        let window = TileWindow::new(carrier, x0, y0, tile_size, tile_size);
        let base_x = x0 / tile_size;
        let base_y = y0 / tile_size;
        for dy in 0..=2u32 {
            if base_y.saturating_add(dy) >= TILED_MAX_GRID {
                break;
            }
            for dx in 0..=2u32 {
                if base_x.saturating_add(dx) >= TILED_MAX_GRID {
                    break;
                }
                for pass in 0..5u32 {
                    let candidate_seed = tiled_candidate_seed(seed, base_x + dx, base_y + dy, pass);
                    if let Some(bytes) =
                        extract_v2_carrier(&window, payload_bits, candidate_seed, 1)
                    {
                        if bytes.len() >= payload_len {
                            return Ok(bytes);
                        }
                    }
                }
            }
        }
    }
    let required = lsb_required_capacity_v2(payload_bits, 1);
    let available = lsb_available_slots(tile_size, tile_size).unwrap_or(0);
    Err(super::StegoError::InsufficientCapacity {
        required,
        available,
    })
}

/// Shared bounded tiled framed-extraction core over any pixel carrier.
///
/// Recovers the frame header with bounded search, validates the declared
/// length against frame bounds and per-tile carrier capacity before full
/// extraction, then recovers the full frame from the same candidate identity
/// (origin, tile-grid seed, pass) and validates CRC32.
pub(crate) fn extract_tiled_framed_carrier<C: PixelCarrier>(
    carrier: &C,
    seed: u64,
    tile_size: u32,
    max_origins: u32,
) -> Result<Vec<u8>, super::StegoError> {
    crate::types::validate_max_origins(max_origins)?;
    let (width, height) = (carrier.carrier_width(), carrier.carrier_height());
    let header_bits = crate::frame::FRAME_HEADER_SIZE
        .checked_mul(8)
        .ok_or_else(|| {
            super::StegoError::ResourceLimitExceeded("frame header size overflow".to_string())
        })?;
    let header_required = lsb_required_capacity_v2(header_bits, 1);
    if width < tile_size || height < tile_size {
        return Err(super::StegoError::InsufficientCapacity {
            required: header_required,
            available: 0,
        });
    }
    let origins = tiled_origins(width, height, tile_size, max_origins);
    if origins.is_empty() {
        return Err(super::StegoError::InsufficientCapacity {
            required: header_required,
            available: 0,
        });
    }
    let tile_available = lsb_available_slots(tile_size, tile_size).unwrap_or(0);
    for (x0, y0) in origins {
        let window = TileWindow::new(carrier, x0, y0, tile_size, tile_size);
        let base_x = x0 / tile_size;
        let base_y = y0 / tile_size;
        for dy in 0..=2u32 {
            if base_y.saturating_add(dy) >= TILED_MAX_GRID {
                break;
            }
            for dx in 0..=2u32 {
                if base_x.saturating_add(dx) >= TILED_MAX_GRID {
                    break;
                }
                for pass in 0..5u32 {
                    let candidate_seed = tiled_candidate_seed(seed, base_x + dx, base_y + dy, pass);
                    let Some(prefix) = extract_v2_carrier(&window, header_bits, candidate_seed, 1)
                    else {
                        continue;
                    };
                    let Ok((_, total_len)) = crate::frame::decode_prefix(&prefix) else {
                        continue;
                    };
                    let Some(total_bits) = total_len.checked_mul(8) else {
                        continue;
                    };
                    let full_required = lsb_required_capacity_v2(total_bits, 1);
                    if full_required > tile_available {
                        continue;
                    }
                    let Some(framed) = extract_v2_carrier(&window, total_bits, candidate_seed, 1)
                    else {
                        continue;
                    };
                    if let Ok((_, payload)) = crate::frame::decode(&framed) {
                        return Ok(payload);
                    }
                }
            }
        }
    }
    Err(super::StegoError::FrameNotFound)
}

#[allow(dead_code)]
pub fn crop_rgba(src: &RgbaImage, x: u32, y: u32, w: u32, h: u32) -> RgbaImage {
    let mut out = RgbaImage::new(w, h);
    let (width, height) = src.dimensions();
    if w == 0 || h == 0 || x >= width || y >= height {
        return out;
    }
    let copy_w = w.min(width - x);
    let copy_h = h.min(height - y);
    for dy in 0..copy_h {
        for dx in 0..copy_w {
            let p = src.get_pixel(x + dx, y + dy);
            out.put_pixel(dx, dy, *p);
        }
    }
    out
}

#[allow(dead_code)]
pub fn blit_rgba(dst: &mut RgbaImage, x: u32, y: u32, src: &RgbaImage) {
    if x >= dst.width() || y >= dst.height() {
        return;
    }
    let width = src.width().min(dst.width() - x);
    let height = src.height().min(dst.height() - y);
    for dy in 0..height {
        for dx in 0..width {
            let p = src.get_pixel(dx, dy);
            dst.put_pixel(x + dx, y + dy, *p);
        }
    }
}

pub fn embed_seed_lsb_fallback(img: &mut RgbaImage, seed: u64) {
    let (width, height) = img.dimensions();
    let total_channels = (width as usize)
        .checked_mul(height as usize)
        .and_then(|channels| channels.checked_mul(3))
        .unwrap_or(0);
    if total_channels < 64 {
        return;
    }
    let seed_bytes = seed.to_le_bytes();
    let mut channel_idx = 0;
    for &byte in &seed_bytes {
        for bit in 0..8 {
            let pixel_offset = channel_idx / 3;
            let channel = channel_idx % 3;
            let x = pixel_offset as u32 % width;
            let y = pixel_offset as u32 / width;
            let bit_val = (byte >> bit) & 1;
            let pixel = img.get_pixel(x, y);
            let old_val = pixel[channel];
            if (old_val & 1) == bit_val {
                channel_idx += 1;
                continue;
            }
            let new_val = if old_val == 0 {
                1
            } else if old_val == 255 {
                254
            } else {
                let direction_hash = x.wrapping_mul(31).wrapping_add(y.wrapping_mul(17));
                if direction_hash & 1 == 0 {
                    old_val + 1
                } else {
                    old_val - 1
                }
            };
            let new_pixel = Rgba([
                if channel == 0 { new_val } else { pixel[0] },
                if channel == 1 { new_val } else { pixel[1] },
                if channel == 2 { new_val } else { pixel[2] },
                pixel[3],
            ]);
            img.put_pixel(x, y, new_pixel);
            channel_idx += 1;
        }
    }
}

pub fn extract_seed_lsb_fallback(img: &RgbaImage) -> Option<u64> {
    let (width, height) = img.dimensions();
    let total_channels = (width as usize)
        .checked_mul(height as usize)
        .and_then(|channels| channels.checked_mul(3))
        .unwrap_or(0);
    if total_channels < 64 {
        return None;
    }
    let mut bytes = [0u8; 8];
    let mut channel_idx = 0;
    for byte in bytes.iter_mut() {
        for bit in 0..8 {
            let pixel_offset = channel_idx / 3;
            let channel = channel_idx % 3;
            let x = pixel_offset as u32 % width;
            let y = pixel_offset as u32 / width;
            let pixel = img.get_pixel(x, y);
            *byte |= (pixel[channel] & 1) << bit;
            channel_idx += 1;
        }
    }
    let seed = u64::from_le_bytes(bytes);
    Some(seed)
}

/// Configuration for LSB carrier operations.
///
/// Controls the seed, redundancy, and optional tile size for pixel-domain
/// steganographic embedding and extraction.
///
/// Use [`LsbConfig::new`] for compile-time-valid values and
/// [`LsbConfig::try_new`] / [`LsbConfig::try_with_redundancy`] when the
/// redundancy comes from untrusted input (configuration files, CLI parsing,
/// network requests, etc.).
///
/// # Examples
///
/// ```rust
/// use stegoeggo_stego::lsb::LsbConfig;
///
/// let config = LsbConfig::new(42);
/// assert_eq!(config.seed(), 42);
/// assert_eq!(config.redundancy(), 2);
/// ```
#[derive(Debug, Clone)]
pub struct LsbConfig {
    seed: u64,
    redundancy: usize,
}

impl LsbConfig {
    /// Create a new configuration with the given seed and default redundancy (2).
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self {
            seed,
            redundancy: 2,
        }
    }

    /// Fallible constructor that validates the redundancy up front.
    ///
    /// Use this when the redundancy value comes from untrusted input. Returns
    /// [`StegoError::InvalidConfig`](crate::StegoError::InvalidConfig) if `redundancy` is outside `1..=10`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use stegoeggo_stego::lsb::LsbConfig;
    ///
    /// let config = LsbConfig::try_new(42, 3)?;
    /// assert_eq!(config.redundancy(), 3);
    /// # Ok::<_, stegoeggo_stego::StegoError>(())
    /// ```
    pub fn try_new(seed: u64, redundancy: usize) -> Result<Self, super::StegoError> {
        crate::constants::validate_redundancy(redundancy)?;
        Ok(Self { seed, redundancy })
    }

    /// Infallible constructor from a validated [`Redundancy`](crate::Redundancy).
    ///
    /// This is the recommended configuration primitive for runtime values:
    /// validation happens once in [`Redundancy::from_usize`] with identical
    /// semantics in every build profile.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use stegoeggo_stego::lsb::LsbConfig;
    /// use stegoeggo_stego::Redundancy;
    ///
    /// let config = LsbConfig::from_redundancy(42, Redundancy::new(3)?);
    /// assert_eq!(config.redundancy(), 3);
    /// # Ok::<_, stegoeggo_stego::StegoError>(())
    /// ```
    #[must_use]
    pub fn from_redundancy(seed: u64, redundancy: crate::Redundancy) -> Self {
        Self {
            seed,
            redundancy: redundancy.get_usize(),
        }
    }

    /// Set the redundancy level (1–10). Higher redundancy increases
    /// robustness at the cost of reduced capacity.
    ///
    /// Compatibility builder for compile-time-constant values. In debug
    /// builds, panics if `redundancy` is 0 or greater than 10. In
    /// release builds with `panic=abort`, an out-of-range value is clamped to
    /// `1..=10` to avoid aborting the process; prefer
    /// [`LsbConfig::try_with_redundancy`](Self::try_with_redundancy) or
    /// [`LsbConfig::with_redundancy_value`](Self::with_redundancy_value) when
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
    /// Returns [`StegoError::InvalidConfig`](crate::StegoError::InvalidConfig) if `redundancy` is outside
    /// `1..=10`. Prefer this over [`with_redundancy`](Self::with_redundancy)
    /// when the value is derived from runtime configuration.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use stegoeggo_stego::lsb::LsbConfig;
    ///
    /// let user_redundancy: usize = 0;
    /// let result = LsbConfig::new(42).try_with_redundancy(user_redundancy);
    /// assert!(result.is_err());
    /// # Ok::<_, stegoeggo_stego::StegoError>(())
    /// ```
    pub fn try_with_redundancy(mut self, redundancy: usize) -> Result<Self, super::StegoError> {
        crate::constants::validate_redundancy(redundancy)?;
        self.redundancy = redundancy;
        Ok(self)
    }

    /// Infallible setter from a validated [`Redundancy`](crate::Redundancy).
    ///
    /// Recommended over [`with_redundancy`](Self::with_redundancy) for
    /// runtime values: the contract is identical in every build profile.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use stegoeggo_stego::lsb::LsbConfig;
    /// use stegoeggo_stego::Redundancy;
    ///
    /// let config = LsbConfig::new(42).with_redundancy_value(Redundancy::MAX);
    /// assert_eq!(config.redundancy(), 10);
    /// ```
    #[must_use]
    pub fn with_redundancy_value(mut self, redundancy: crate::Redundancy) -> Self {
        self.redundancy = redundancy.get_usize();
        self
    }

    /// The redundancy level as a validated [`Redundancy`](crate::Redundancy).
    #[must_use]
    pub fn redundancy_value(&self) -> crate::Redundancy {
        crate::Redundancy::from_usize(self.redundancy)
            .expect("LsbConfig invariant: redundancy is always validated")
    }

    /// The seed used for the carrier permutation.
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

/// Query the available LSB capacity for an RGBA image.
///
/// Returns a [`CapacityReport`](super::CapacityReport) indicating how many
/// payload bytes can be embedded with the given configuration.
///
/// # Arguments
///
/// * `img` — The RGBA image to query.
/// * `payload_len` — Desired payload length in bytes.
/// * `config` — LSB carrier configuration.
///
/// # Examples
///
/// ```rust
/// use image::RgbaImage;
/// use stegoeggo_stego::lsb::{self, LsbConfig};
///
/// let img = RgbaImage::new(100, 100);
/// let config = LsbConfig::new(42);
/// let report = lsb::capacity(&img, 100, &config).unwrap();
/// assert!(report.is_sufficient());
/// ```
#[must_use = "capacity reports should be inspected"]
pub fn capacity(
    img: &RgbaImage,
    payload_len: usize,
    config: &LsbConfig,
) -> Result<super::CapacityReport, super::StegoError> {
    let (w, h) = img.dimensions();
    let available = checked_lsb_available_slots(w, h)?;
    let payload_bits = payload_len.saturating_mul(8);
    let required = lsb_required_capacity_v2(payload_bits, config.redundancy());
    Ok(super::CapacityReport {
        required,
        available,
    })
}

/// Embed arbitrary bytes into an RGBA image using V2 corrected carrier LSB.
///
/// Returns an [`EmbedReport`](super::EmbedReport) with the output
/// [`RgbaImage`] and capacity information. No image encoding is performed.
///
/// # Arguments
///
/// * `img` — The RGBA image to embed into.
/// * `payload` — Arbitrary bytes to embed.
/// * `config` — LSB carrier configuration.
///
/// # Errors
///
/// Returns [`StegoError::EmptyCarrier`](crate::StegoError::EmptyCarrier) if the image has zero pixels.
///
/// # Examples
///
/// ```rust,no_run
/// use image::RgbaImage;
/// use stegoeggo_stego::lsb::{self, LsbConfig};
///
/// let img = RgbaImage::new(100, 100);
/// let config = LsbConfig::new(42);
/// let report = lsb::embed(&img, b"secret message", &config).unwrap();
/// assert!(report.embedded);
/// ```
pub fn embed(
    img: &RgbaImage,
    payload: &[u8],
    config: &LsbConfig,
) -> Result<super::EmbedReport<RgbaImage>, super::StegoError> {
    if img.dimensions() == (0, 0) {
        return Err(super::StegoError::EmptyCarrier);
    }
    checked_lsb_available_slots(img.width(), img.height())?;

    let mut output = img.clone();
    let report = embed_lsb_v2_in_place(&mut output, payload, config.seed(), config.redundancy());
    Ok(super::EmbedReport {
        embedded: report.embedded,
        output,
        payload_bytes: report.payload_bytes,
        required_capacity: report.required_capacity,
        available_capacity: report.available_capacity,
        actual_redundancy: report.actual_redundancy,
    })
}

/// Embed arbitrary bytes into an RGBA image in place using V2 corrected carrier LSB.
///
/// Unlike [`embed`], this operation mutates the caller's image and does not
/// allocate a replacement image. Capacity is checked before the first pixel
/// mutation, so an insufficient carrier is left unchanged.
pub fn embed_in_place(
    img: &mut RgbaImage,
    payload: &[u8],
    config: &LsbConfig,
) -> Result<InPlaceEmbedReport, super::StegoError> {
    if img.dimensions() == (0, 0) {
        return Err(super::StegoError::EmptyCarrier);
    }
    checked_lsb_available_slots(img.width(), img.height())?;

    Ok(embed_lsb_v2_in_place(
        img,
        payload,
        config.seed(),
        config.redundancy(),
    ))
}

/// Extract arbitrary bytes from an RGBA image using V2 corrected carrier LSB.
///
/// # Arguments
///
/// * `img` — The RGBA image to extract from.
/// * `payload_len` — Expected payload length in bytes.
/// * `config` — LSB carrier configuration (must match the embedding config).
///
/// # Errors
///
/// Returns [`StegoError::MalformedInput`](crate::StegoError::MalformedInput) if extraction produces invalid data.
///
/// # Examples
///
/// ```rust,no_run
/// use image::RgbaImage;
/// use stegoeggo_stego::lsb::{self, LsbConfig};
///
/// # let img = RgbaImage::new(100, 100);
/// # let config = LsbConfig::new(42);
/// # let embedded = lsb::embed(&img, b"secret", &config).unwrap();
/// let recovered = lsb::extract(&embedded.output, 6, &config).unwrap();
/// assert_eq!(&recovered, b"secret");
/// ```
pub fn extract(
    img: &RgbaImage,
    payload_len: usize,
    config: &LsbConfig,
) -> Result<Vec<u8>, super::StegoError> {
    let bits = payload_len.checked_mul(8).ok_or_else(|| {
        super::StegoError::ResourceLimitExceeded("payload length overflow".into())
    })?;
    checked_lsb_available_slots(img.width(), img.height())?;
    extract_lsb_v2(img, bits, config.seed(), config.redundancy())
        .ok_or_else(|| super::StegoError::MalformedInput("extraction returned no data".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn uniform_image(width: u32, height: u32, value: u8) -> RgbaImage {
        RgbaImage::from_fn(width, height, |_x, _y| Rgba([value, value, value, 255]))
    }

    #[test]
    fn carrier_slot_rejects_out_of_range_slot() {
        assert_eq!(carrier_v2_slot_to_pixel_channel(2, 1, 1), Some((0, 2)));
        assert_eq!(carrier_v2_slot_to_pixel_channel(3, 1, 1), None);
    }

    #[test]
    fn lsb_available_slots_rejects_overflow() {
        assert_eq!(lsb_available_slots(u32::MAX, u32::MAX), None);
    }

    #[test]
    fn embed_v2_rejects_out_of_range_redundancy() {
        let mut img = uniform_image(16, 16, 128);
        let report = embed_lsb_v2_in_place(&mut img, &[0xA5], 42, 0);
        assert!(!report.embedded);
        let mut img = uniform_image(16, 16, 128);
        let report = embed_lsb_v2_in_place(&mut img, &[0xA5], 42, 11);
        assert!(!report.embedded);
    }

    fn channel_value(img: &RgbaImage, x: u32, y: u32, channel: usize) -> u8 {
        let p = img.get_pixel(x, y);
        p[channel]
    }

    fn assert_bit_set(img: &RgbaImage, x: u32, y: u32, channel: usize, bit: u8) {
        let v = channel_value(img, x, y, channel);
        assert_eq!(
            v & 1,
            bit,
            "channel value {v} has wrong LSB (wanted {bit}) at ({x},{y},c{channel})"
        );
    }

    fn assert_bounded_mutation(old: u8, new: u8) {
        assert_ne!(old, new, "mutation must change the channel");
        assert_eq!(
            (old as i16 - new as i16).abs(),
            1,
            "mutation must differ by exactly ±1 (old={old}, new={new})"
        );
        assert!(
            new != 0 || old != 255,
            "mutation must not wrap 255 -> 0 (old={old}, new={new})"
        );
        assert!(
            new != 255 || old != 0,
            "mutation must not wrap 0 -> 255 (old={old}, new={new})"
        );
    }

    #[test]
    fn embed_bit_in_pixel_leaves_matching_lsb_unchanged() {
        for &initial in &[0u8, 1, 2, 127, 128, 253, 254, 255] {
            for &bit in &[0u8, 1] {
                let mut img = uniform_image(4, 4, initial);
                embed_bit_in_pixel(&mut img, 1, 2, 1, bit);
                if (initial & 1) == bit {
                    assert_eq!(channel_value(&img, 1, 2, 1), initial);
                } else {
                    let new = channel_value(&img, 1, 2, 1);
                    assert_bounded_mutation(initial, new);
                    assert_eq!(new & 1, bit);
                }
            }
        }
    }

    #[test]
    fn stego_permutation_empty_carrier_is_safe() {
        assert_eq!(stego_permutation(0, 0, 42), 0);
        assert_eq!(stego_permutation(17, 0, 42), 0);
    }

    #[test]
    fn embed_lsb_tiled_skips_partial_edge_tiles() {
        let img = uniform_image(100, 100, 128);
        let payload = b"tile";

        let outcome = embed_lsb_tiled(&img, payload, 42, 64);
        let EmbedOutcome::Embedded {
            output,
            required_capacity,
            available_capacity,
            ..
        } = outcome
        else {
            panic!("full 64x64 tile should embed");
        };

        assert_eq!(
            required_capacity,
            payload.len() * 8 * STEGO_SPREAD_FACTOR,
            "only the single full 64x64 tile counts toward required capacity"
        );
        assert_eq!(
            available_capacity,
            lsb_available_slots(64, 64).unwrap(),
            "partial edge tiles must not contribute capacity"
        );

        for y in 0..100u32 {
            for x in 0..100u32 {
                if x < 64 && y < 64 {
                    continue;
                }
                assert_eq!(
                    channel_value(&output, x, y, 0),
                    128,
                    "pixel outside the full tile must be untouched"
                );
            }
        }

        let sub = crop_rgba(&output, 0, 0, 64, 64);
        let seed = tile_seed(42, 0, 0).wrapping_mul(crate::constants::STEGO_OFFSET_SEED_1);
        let recovered = extract_lsb_v2(&sub, payload.len() * 8, seed, 1)
            .expect("full tile should carry the payload");
        assert_eq!(recovered, payload);
    }

    #[test]
    fn tiled_in_place_matches_cloning_embed() {
        let img = uniform_image(256, 256, 0x7F);
        let payload = vec![0xA5; 36];
        let cloned = embed_lsb_tiled(&img, &payload, 42, 64);
        let mut inplace = img.clone();
        let report = embed_lsb_tiled_in_place(&mut inplace, &payload, 42, 64);
        assert!(cloned.is_embedded());
        assert!(report.embedded);
        assert_eq!(inplace, cloned.into_inner());
        assert_eq!(report.payload_bytes, payload.len());
    }

    #[test]
    fn tiled_in_place_report_matches_cloning_capacities() {
        let img = uniform_image(100, 100, 128);
        let payload = b"tile";
        let cloned = embed_lsb_tiled(&img, payload, 42, 64);
        let mut inplace = img.clone();
        let report = embed_lsb_tiled_in_place(&mut inplace, payload, 42, 64);
        let EmbedOutcome::Embedded {
            required_capacity,
            available_capacity,
            ..
        } = cloned
        else {
            panic!("full 64x64 tile should embed");
        };
        assert_eq!(report.required_capacity, required_capacity);
        assert_eq!(report.available_capacity, available_capacity);
        assert_eq!(report.actual_redundancy, 1);
    }

    #[test]
    fn tiled_in_place_leaves_carrier_unchanged_on_insufficient_capacity() {
        let payload = vec![0xA5; 36];
        let tiny = uniform_image(40, 40, 0x7F);
        let mut untouched = tiny.clone();
        let report = embed_lsb_tiled_in_place(&mut untouched, &payload, 42, 64);
        assert!(!report.embedded);
        assert_eq!(untouched, tiny);

        let img = uniform_image(128, 128, 0x7F);
        let oversized = vec![0xA5; 10_000];
        let mut intact = img.clone();
        let skipped = embed_lsb_tiled_in_place(&mut intact, &oversized, 42, 64);
        assert!(!skipped.embedded);
        assert_eq!(intact, img);
    }

    #[test]
    fn stego_permutation_v2_bounded_on_tiny_slot_counts() {
        for slot_count in 2..=17usize {
            for seed in 0..64u64 {
                for index in 0..32usize {
                    let slot = stego_permutation_v2(index, slot_count, seed)
                        .expect("tiny slot counts have a valid permutation domain");
                    assert!(slot < slot_count, "slot {slot} out of range {slot_count}");
                }
            }
        }
    }

    #[test]
    fn stego_permutation_v2_rejects_power_of_two_overflow() {
        assert_eq!(stego_permutation_v2(0, usize::MAX, 42), None);
    }

    #[test]
    fn stego_permutation_v2_tiny_carriers_reach_every_slot() {
        for slot_count in [2usize, 3, 5] {
            let hits: std::collections::HashSet<usize> = (0..512u64)
                .flat_map(|seed| {
                    (0..16usize).map(move |index| {
                        stego_permutation_v2(index, slot_count, seed)
                            .expect("tiny slot counts have a valid permutation domain")
                    })
                })
                .collect();
            assert_eq!(
                hits.len(),
                slot_count,
                "slot_count {slot_count}: permutation collapsed onto {hits:?}"
            );
        }
    }

    #[test]
    fn stego_permutation_v2_chi_squared_uniformity() {
        for slot_count in [7usize, 10, 13, 100] {
            for seed in [42u64, 12345, 0xDEAD_BEEF] {
                let trials = 7000usize;
                let mut counts = vec![0usize; slot_count];
                for index in 0..trials {
                    let slot = stego_permutation_v2(index, slot_count, seed).unwrap();
                    counts[slot] += 1;
                }
                let expected = trials as f64 / slot_count as f64;
                let mut chi2 = 0.0;
                for &c in &counts {
                    let diff = c as f64 - expected;
                    chi2 += diff * diff / expected;
                }
                let threshold = slot_count as f64 * 1000.0;
                assert!(
                    chi2 < threshold,
                    "chi-squared {chi2:.2} exceeds threshold {threshold:.2} (non-uniform tail cutoff; 256-step fallback documented) for slot_count {slot_count} seed {seed} counts {counts:?}"
                );
                for &c in &counts {
                    assert!(
                        c > 0,
                        "bucket empty for slot_count {slot_count} seed {seed} counts {counts:?}"
                    );
                    let ratio = c as f64 / expected;
                    assert!(
                        (0.2..=5.0).contains(&ratio),
                        "bucket ratio {ratio:.2} outside 0.2..5.0 for slot_count {slot_count} seed {seed} (documented 256-step cutoff discontinuity)"
                    );
                }
            }
        }
    }

    fn assert_full_domain_injective(slot_count: usize, seeds: &[u64]) -> (usize, usize) {
        let mut max_depth = 0usize;
        let mut fallback_hits = 0usize;
        for &seed in seeds {
            let mut seen = vec![false; slot_count];
            for index in 0..slot_count {
                let (slot, depth, used_fallback) =
                    stego_permutation_v2_debug(index, slot_count, seed).unwrap_or_else(|| {
                        panic!("slot_count {slot_count} seed {seed} index {index} has no image")
                    });
                assert!(
                    slot < slot_count,
                    "slot_count {slot_count} seed {seed} index {index} mapped out of range to {slot}"
                );
                assert!(
                    !seen[slot],
                    "slot_count {slot_count} seed {seed}: logical indices collide at slot {slot}"
                );
                seen[slot] = true;
                max_depth = max_depth.max(depth);
                fallback_hits += usize::from(used_fallback);
            }
            assert!(
                seen.iter().all(|&hit| hit),
                "slot_count {slot_count} seed {seed}: full-domain image misses slots"
            );
        }
        (max_depth, fallback_hits)
    }

    #[test]
    fn stego_permutation_v2_injective_over_documented_domains() {
        const SEEDS: &[u64] = &[
            0,
            1,
            42,
            12345,
            0x0123_4567_89AB_CDEF,
            0xDEAD_BEEF,
            u64::MAX,
        ];
        let mut global_max_depth = 0usize;
        let mut global_fallback_hits = 0usize;
        let mut domains = Vec::new();
        domains.extend(2..=64usize);
        domains.extend(
            [
                65usize, 100, 127, 128, 129, 255, 256, 257, 511, 512, 513, 1000, 1023, 1024, 1025,
                2048, 2049, 4095, 4096, 4097, 8191, 8192, 8193, 12288, 16383, 16384, 16385,
            ]
            .iter()
            .copied(),
        );
        for slot_count in domains {
            let (max_depth, fallback_hits) = assert_full_domain_injective(slot_count, SEEDS);
            global_max_depth = global_max_depth.max(max_depth);
            global_fallback_hits += fallback_hits;
        }
        for slot_count in [65535usize, 65536, 65537, 100_000] {
            let (max_depth, fallback_hits) =
                assert_full_domain_injective(slot_count, &[42, u64::MAX]);
            global_max_depth = global_max_depth.max(max_depth);
            global_fallback_hits += fallback_hits;
        }
        assert_eq!(
            global_fallback_hits, 0,
            "256-step fallback reached {global_fallback_hits} times in documented domains; record disposition before freezing the mapping"
        );
        assert!(
            global_max_depth < 256,
            "observed walk depth {global_max_depth} must stay below the 256-step bound"
        );
    }

    #[test]
    fn stego_permutation_v2_pathological_domains_are_injective() {
        let mut seeds: Vec<u64> = (0..512u64).collect();
        seeds.extend([u64::MAX - 1, u64::MAX]);
        for slot_count in [2usize, 3, 5, 6, 7, 9, 17, 31, 33] {
            assert_full_domain_injective(slot_count, &seeds);
        }
    }

    #[test]
    fn stego_permutation_v2_large_dimensions_terminate_in_range() {
        for slot_count in [u32::MAX as usize, (u32::MAX as usize).saturating_mul(3)] {
            match stego_permutation_v2_debug(0, slot_count, 42) {
                Some((slot, _, _)) => assert!(slot < slot_count),
                None => assert_eq!(slot_count, usize::MAX),
            }
        }
        assert_eq!(lsb_available_slots(u32::MAX, u32::MAX), None);
        assert_eq!(
            lsb_available_slots(u32::MAX, 1),
            Some(u32::MAX as usize * 3)
        );
    }

    #[test]
    fn stego_permutation_v2_exact_capacity_round_trip() {
        let (width, height) = (512u32, 512u32);
        let available = lsb_available_slots(width, height).unwrap();
        let payload_bits = available / STEGO_SPREAD_FACTOR / 8 * 8;
        let payload_len = payload_bits / 8;
        assert!(payload_len > 0);
        let payload: Vec<u8> = (0..payload_len).map(|i| (i % 251) as u8).collect();
        let mut img = uniform_image(width, height, 0x7F);
        let report = embed_lsb_v2_in_place(&mut img, &payload, 0x0123_4567_89AB_CDEF, 1);
        assert!(report.embedded);
        let recovered = extract_lsb_v2(&img, payload_bits, 0x0123_4567_89AB_CDEF, 1)
            .expect("exact-capacity embed must extract");
        assert_eq!(recovered, payload);
    }

    #[test]
    fn embed_bit_in_pixel_bounded_at_zero() {
        for &bit in &[0u8, 1] {
            let mut img = uniform_image(2, 2, 0);
            embed_bit_in_pixel(&mut img, 0, 0, 0, bit);
            let new = channel_value(&img, 0, 0, 0);
            if bit == 0 {
                assert_eq!(new, 0);
            } else {
                assert_eq!(new, 1, "0 must mutate to 1, not 255");
            }
        }
    }

    #[test]
    fn embed_bit_in_pixel_bounded_at_255() {
        for &bit in &[0u8, 1] {
            let mut img = uniform_image(2, 2, 255);
            embed_bit_in_pixel(&mut img, 0, 0, 0, bit);
            let new = channel_value(&img, 0, 0, 0);
            if bit == 1 {
                assert_eq!(new, 255);
            } else {
                assert_eq!(new, 254, "255 must mutate to 254, not 0");
            }
        }
    }

    #[test]
    fn embed_bit_in_pixel_exhaustive_boundary_table() {
        for &initial in &[0u8, 1, 2, 127, 128, 253, 254, 255] {
            for &bit in &[0u8, 1] {
                for channel in 0..3usize {
                    let mut img = uniform_image(2, 2, initial);
                    embed_bit_in_pixel(&mut img, 1, 0, channel, bit);
                    let new = channel_value(&img, 1, 0, channel);
                    if (initial & 1) == bit {
                        assert_eq!(new, initial);
                    } else {
                        assert_bounded_mutation(initial, new);
                    }
                    assert_bit_set(&img, 1, 0, channel, bit);
                    let p = img.get_pixel(1, 0);
                    assert_eq!(p[3], 255, "alpha must remain unchanged");
                    for other in 0..3usize {
                        if other != channel {
                            assert_eq!(p[other], initial, "other RGB channels unchanged");
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn embed_bit_in_pixel_black_image_no_salt_and_pepper() {
        let mut img = uniform_image(16, 16, 0);
        embed_bit_in_pixel(&mut img, 0, 0, 0, 1);
        embed_bit_in_pixel(&mut img, 1, 0, 1, 1);
        embed_bit_in_pixel(&mut img, 2, 0, 2, 1);
        embed_bit_in_pixel(&mut img, 0, 1, 0, 0);

        let p0 = img.get_pixel(0, 0);
        assert_eq!(*p0, Rgba([1, 0, 0, 255]));
        let p1 = img.get_pixel(1, 0);
        assert_eq!(*p1, Rgba([0, 1, 0, 255]));
        let p2 = img.get_pixel(2, 0);
        assert_eq!(*p2, Rgba([0, 0, 1, 255]));
        let p3 = img.get_pixel(0, 1);
        assert_eq!(*p3, Rgba([0, 0, 0, 255]), "matching LSB must not change");
    }

    #[test]
    fn embed_bit_in_pixel_white_image_no_salt_and_pepper() {
        let mut img = uniform_image(16, 16, 255);
        embed_bit_in_pixel(&mut img, 0, 0, 0, 0);
        embed_bit_in_pixel(&mut img, 1, 0, 1, 0);
        embed_bit_in_pixel(&mut img, 2, 0, 2, 0);

        let p0 = img.get_pixel(0, 0);
        assert_eq!(*p0, Rgba([254, 255, 255, 255]));
        let p1 = img.get_pixel(1, 0);
        assert_eq!(*p1, Rgba([255, 254, 255, 255]));
        let p2 = img.get_pixel(2, 0);
        assert_eq!(*p2, Rgba([255, 255, 254, 255]));
    }

    #[test]
    fn embed_seed_lsb_fallback_bounded_at_extrema() {
        let mut img = uniform_image(16, 16, 0);
        embed_seed_lsb_fallback(&mut img, 42);
        for y in 0..16 {
            for x in 0..16 {
                let p = img.get_pixel(x, y);
                for c in 0..3 {
                    assert!(
                        p[c] <= 1,
                        "0-image seed embed must only produce 0/1 channels (got {} at c{c})",
                        p[c]
                    );
                }
            }
        }

        let mut img = uniform_image(16, 16, 255);
        embed_seed_lsb_fallback(&mut img, 42);
        for y in 0..16 {
            for x in 0..16 {
                let p = img.get_pixel(x, y);
                for c in 0..3 {
                    assert!(
                        p[c] == 254 || p[c] == 255,
                        "255-image seed embed must only produce 254/255 channels (got {} at c{c})",
                        p[c]
                    );
                }
            }
        }
    }

    #[test]
    fn embed_extract_roundtrip_on_extreme_images() {
        for &seed_byte in &[0u8, 1, 127, 128, 254, 255] {
            let mut img = uniform_image(64, 64, seed_byte);
            let payload = b"plan065 bounded mutation round-trip";
            embed_seed_lsb_fallback(&mut img, 7);

            for y in 0..64u32 {
                for x in 0..64u32 {
                    let p = img.get_pixel(x, y);
                    for c in 0..3 {
                        let v = p[c];
                        assert!(
                            v == seed_byte || v.abs_diff(seed_byte) <= 1,
                            "seed-fallback must not wrap (got {v} from {seed_byte} at ({x},{y},c{c}))"
                        );
                    }
                }
            }

            let seed = extract_seed_lsb_fallback(&img);
            assert_eq!(seed, Some(7));
            let _ = payload;
        }
    }

    #[test]
    fn embed_extract_seed_zero_roundtrips() {
        let mut img = uniform_image(16, 16, 0);
        embed_seed_lsb_fallback(&mut img, 0);
        assert_eq!(extract_seed_lsb_fallback(&img), Some(0));
    }

    #[test]
    fn lsb_config_try_new_accepts_valid_redundancy() {
        for r in 1..=10usize {
            let config = LsbConfig::try_new(42, r).unwrap();
            assert_eq!(config.redundancy(), r);
            assert_eq!(config.seed(), 42);
        }
    }

    #[test]
    fn lsb_config_try_new_rejects_out_of_range_redundancy() {
        assert!(matches!(
            LsbConfig::try_new(42, 0),
            Err(crate::StegoError::InvalidConfig(_))
        ));
        assert!(matches!(
            LsbConfig::try_new(42, 11),
            Err(crate::StegoError::InvalidConfig(_))
        ));
        assert!(matches!(
            LsbConfig::try_new(42, usize::MAX),
            Err(crate::StegoError::InvalidConfig(_))
        ));
    }

    #[test]
    fn lsb_config_try_with_redundancy_accepts_valid_values() {
        let config = LsbConfig::new(42).try_with_redundancy(7).unwrap();
        assert_eq!(config.redundancy(), 7);
    }

    #[test]
    fn lsb_config_try_with_redundancy_rejects_out_of_range() {
        assert!(LsbConfig::new(42).try_with_redundancy(0).is_err());
        assert!(LsbConfig::new(42).try_with_redundancy(11).is_err());
        assert!(LsbConfig::new(42).try_with_redundancy(usize::MAX).is_err());
    }

    #[test]
    fn lsb_validated_config_matches_legacy_carrier_bytes() {
        let payload = b"validated!";
        for redundancy in 1..=10u8 {
            let legacy = LsbConfig::new(99).with_redundancy(redundancy as usize);
            let validated =
                LsbConfig::from_redundancy(99, crate::Redundancy::new(redundancy).unwrap());
            assert_eq!(validated.redundancy(), legacy.redundancy());
            assert_eq!(validated.redundancy_value().get(), redundancy);
            let mut legacy_img = uniform_image(64, 64, 0x7F);
            let legacy_report =
                embed_lsb_v2_in_place(&mut legacy_img, payload, 99, redundancy as usize);
            let validated_report = embed(&uniform_image(64, 64, 0x7F), payload, &validated)
                .expect("validated config embeds");
            assert!(legacy_report.embedded);
            assert!(validated_report.embedded);
            let mut validated_in_place = uniform_image(64, 64, 0x7F);
            let in_place_report =
                embed_lsb_v2_in_place(&mut validated_in_place, payload, 99, redundancy as usize);
            assert!(in_place_report.embedded);
            assert_eq!(validated_report.output, validated_in_place);
            assert_eq!(legacy_img, validated_in_place);
        }
    }

    #[test]
    fn lsb_zero_seed_is_valid_carrier_config() {
        let payload = b"zero seed stays valid";
        let config = LsbConfig::try_new(0, 2).unwrap();
        let report =
            embed(&uniform_image(48, 48, 0x7F), payload, &config).expect("zero seed embeds");
        assert!(report.embedded);
        let recovered =
            extract(&report.output, payload.len(), &config).expect("zero seed extracts");
        assert_eq!(&recovered, payload);
    }

    #[test]
    fn lsb_config_fallible_does_not_panic() {
        for r in [0usize, 11, usize::MAX, 100, 1_000_000] {
            let result = std::panic::catch_unwind(|| LsbConfig::try_new(42, r));
            assert!(result.is_ok(), "try_new panicked for redundancy {r}");
            assert!(
                result.unwrap().is_err(),
                "expected error for redundancy {r}"
            );

            let result = std::panic::catch_unwind(|| LsbConfig::new(42).try_with_redundancy(r));
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
}
