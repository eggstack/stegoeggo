# JPEG Transcoder Review Findings

## Document: jpeg-transcoder.md
### Verified Claims
- `Coefficients` type defined as `HashMap<u8, Vec<...>>` mapping component IDs to 8x8 blocks — confirmed at `mod.rs:14`
- Natural (row-major) order storage via `block[ZIGZAG[k]] = magnitude` — confirmed at `entropy.rs:388` with explicit comment at `entropy.rs:400-402`
- `get_scan_data_start` uses `checked_add` for overflow protection — confirmed at `mod.rs:272`
- `is_progressive_jpeg` checks for progressive mode (SOF2 marker via `JpegHeader::parse`) — confirmed at `mod.rs:286-290`, SOF2 detection at `header.rs:269-272`
- `debug_assert!` for 8-bit quantization values exceeding 255 in `assemble_jpeg` — confirmed at `mod.rs:131-135`
- Standalone marker handling (RST0-RST7, SOI, EOI) in `get_scan_data_start` — confirmed at `mod.rs:252-259`
- Stuffed byte handling (0xFF 0x00) in `get_scan_data_start` — confirmed at `mod.rs:262-265`
- Module line count ~325 lines — actual is 325 lines
- JpegTranscoder struct with static methods `decode_coefficients`, `encode_coefficients`, `assemble_jpeg` — confirmed at `mod.rs:45-200`
- Module interactions with header.rs, entropy.rs, stego_f5.rs, steganography.rs — confirmed
- Progressive JPEG: `decode_coefficients` returns `Unsupported` error — confirmed at `mod.rs:58-65`

### Discrepancies
- **Coefficients inner type**: Doc says `[i64; 64]`, actual is `[i16; 64]` (`mod.rs:14`). The `i64` in the doc is incorrect.
- **TranscoderError variants**: Doc shows `InvalidData(String)`, `UnsupportedFeature(String)`, `EncodingError(String)`. Actual variants (`mod.rs:17-35`) are `InvalidFormat(String)`, `Unsupported(String)`, `HuffmanDecode(String)`, `HuffmanEncode(String)`, `Io(std::io::Error)`, `EmbeddingFailed(String)`. Three additional variants not documented.
- **assemble_jpeg visibility**: Doc says `pub fn assemble_jpeg`, actual is `fn assemble_jpeg` (private, `mod.rs:93`).
- **assemble_jpeg return type**: Doc says returns `Vec<u8>`, actual returns `Result<Vec<u8>>` (`mod.rs:93`).
- **get_scan_data_start return type**: Doc says `Result<usize>`, actual returns `Option<usize>` (`mod.rs:233`). The caller at `mod.rs:68-69` converts via `.ok_or_else(...)`.

### Improvement Opportunities
- None identified (read-only review).

### Potential Bugs/Edge Cases
- The `end_pos < 10` guard at `header.rs:164-168` rejects valid JPEGs shorter than 10 bytes after the last SOI. A minimal valid JPEG (SOI + SOF + minimal scan) could be rejected if the last SOI position + 10 exceeds the file length. However, such images cannot contain usable DCT data anyway, so this is a practical non-issue.
- `get_scan_data_start` stops at the first SOS marker found between SOI and EOI. If a JPEG has multiple SOS markers (e.g., multi-scan progressive), only the first scan data is returned. This is acceptable because the transcoder rejects progressive JPEGs before reaching this point.

---

## Document: jpeg-header.md
### Verified Claims
- `JpegHeader::parse` handles embedded thumbnails by finding the last SOI marker — confirmed at `header.rs:127-148`
- Bounds validation `data.len() < 2` — confirmed at `header.rs:122-124`
- Bounds validation `end_pos < 10` — confirmed at `header.rs:164-168`
- Segment data end uses `.max(segment_data_start)` to prevent inverted slice ranges — confirmed at `header.rs:240-242`
- QuantizationTable struct with `table_id`, `precision`, `values: [u16; 64]` — confirmed at `header.rs:27-31`
- ScanComponent struct with `component_id`, `h_sampling`, `v_sampling`, `quant_table_id`, `dc_table_id`, `ac_table_id` — confirmed at `header.rs:65-73`
- DQT parsing supports 8-bit and 16-bit precision — confirmed at `header.rs:329-346`
- SOF2 detection sets `is_progressive = true` and `coding_process = ProgressiveDCT` — confirmed at `header.rs:269-272`
- `JpegCodingProcess` and `JpegColorSpace` enums with `#[allow(dead_code)]` — file-level `#![allow(dead_code)]` at `header.rs:5` covers these
- Module interactions with mod.rs, entropy.rs, stego_f5.rs — confirmed
- Module line count ~513 lines — actual is 513 lines
- App markers preserved (APP0 and APP1) — confirmed at `header.rs:248-254`
- DRI (restart interval) parsing — confirmed at `header.rs:289-293`
- SOS parsing stops header parsing — confirmed at `header.rs:280-283`
- `JpegHeader::default()` provides reasonable defaults — confirmed at `header.rs:97-115`

### Discrepancies
- **quantization_tables type**: Doc says `Vec<QuantizationTable>`. Actual is `[Option<QuantizationTable>; 4]` (`header.rs:83`). Fixed-size array with Option wrappers, not a dynamic Vec.
- **Huffman table organization**: Doc says `huffman_tables: Vec<HuffmanTable>`. Actual is split into `huffman_tables_dc: Vec<Option<HuffmanTable>>` and `huffman_tables_ac: Vec<Option<HuffmanTable>>` (`header.rs:84-85`). Two separate vectors with Option wrappers.
- **App markers type**: Doc says `app_markers: Vec<Vec<u8>>`. Actual is `app0_marker: Option<Vec<u8>>` and `app1_marker: Option<Vec<u8>>` (`header.rs:89-90`). Only two specific app markers tracked, not a generic vec.
- **HuffmanTable::class field name**: Doc says `class: u8`. Actual is `table_class: u8` (`header.rs:49`).
- **HuffmanTable::counts type**: Doc says `counts: [u8; 16]`. Actual is `counts: [u16; 16]` (`header.rs:51`).

### Improvement Opportunities
- None identified (read-only review).

### Potential Bugs/Edge Cases
- `parse_dqt` uses `while pos + 64 < data.len()` (`header.rs:316`), which requires at least 65 bytes remaining to parse a single 8-bit table. A DQT segment with exactly 64 bytes of table data (the minimum for 8-bit precision) but total segment length of 65 bytes (including the info byte) would be parsed correctly, but a truncated segment with only 64 bytes total would silently skip.
- `parse_sos` silently returns without error if the segment is too short (`header.rs:448-454`). If the SOS segment is truncated, Huffman table assignments may be incomplete, leading to decode errors downstream.
- The header parser processes SOF0 (0xC0), SOF1 (0xC1), and SOF2 (0xC2) but does not handle SOF3 (lossless), SOF9 (arithmetic), or SOF11 (arithmetic lossless). These would be silently skipped as unknown markers, potentially producing incorrect headers.
- Multiple SOI markers: The parser uses the last SOI position, which handles embedded thumbnails correctly. However, if a file has a thumbnail after the main image (non-standard), the parser would use the thumbnail as the main image.

---

## Document: jpeg-entropy.md
### Verified Claims
- CoefficientDecoder pre-builds Huffman decoders from header tables once before MCU loop — confirmed at `entropy.rs:292-316`
- Natural (row-major) order storage via `block[ZIGZAG[k]] = magnitude` — confirmed at `entropy.rs:388` with comment at `entropy.rs:400-402`
- DC differential encoding — confirmed at `entropy.rs:346-356`
- AC run-length + magnitude coding — confirmed at `entropy.rs:359-398`
- Restart marker handling (RST0-RST7) — confirmed at `entropy.rs:216`
- Byte stuffing (0xFF → 0xFF 0x00) — confirmed at `entropy.rs:452-453` (BitWriter) and `entropy.rs:216` (BitReader)
- Coefficient clamping: DC max 11 bits — confirmed at `entropy.rs:620-621`
- Coefficient clamping: AC max 10 bits — confirmed at `entropy.rs:682-683`
- BitReader handles marker stuffing (0xFF 0x00 → 0xFF) — confirmed at `entropy.rs:213-228`
- BitWriter handles byte alignment and stuffing — confirmed at `entropy.rs:462-484`
- Standard Huffman tables (luminance DC/AC, chrominance DC/AC) — confirmed at `entropy.rs:24-67`
- ZIGZAG and REVERSE_ZIGZAG constants — confirmed at `entropy.rs:11-22`
- Zigzag is a valid permutation (tested in unit test at `entropy.rs:774-781`) — confirmed
- REVERSE_ZIGZAG is the inverse of ZIGZAG (tested in unit test at `entropy.rs:783-792`) — confirmed
- Module interactions with mod.rs, header.rs, stego_f5.rs, steganography.rs — confirmed
- Module line count ~808 lines — actual is 808 lines
- Pre-builds HuffmanEncoderTable for O(1) lookup — confirmed at `entropy.rs:76-99`
- `HuffmanEncoderTable` uses `[(u16, u8); 256]` array — confirmed at `entropy.rs:77`

### Discrepancies
- **HuffmanEncoderTable field name**: Doc says `symbols: [(u16, u8); 256]`. Actual field name is `entries: [(u16, u8); 256]` (`entropy.rs:77`).

### Improvement Opportunities
- None identified (read-only review).

### Potential Bugs/Edge Cases
- The Huffman decoder's sign extension at `entropy.rs:172-174` uses `magnitude -= sign_bit - 1` for negative values. For size=15 (max JPEG category), `sign_bit = 32768`, and `magnitude` could be up to 32767. This works correctly because `magnitude` is `i16` and the subtraction stays within range. However, a malformed Huffman stream could produce size values >= 16, which is guarded at `entropy.rs:167-169` (returns `None`).
- `BitReader` stops reading at any non-RST, non-EOI, non-stuffed marker (`entropy.rs:225-227`). This means if the scan data contains a stray 0xFF byte followed by a non-zero marker (e.g., from a corrupted file), decoding terminates early. This is correct behavior for robustness.
- DC predictor overflow: `entropy.rs:349-353` checks for `i16` overflow when accumulating DC differences. If overflow occurs, the block is skipped entirely (`continue`). This silently drops blocks from the coefficient map.
- The encoder's DC coefficient encoding at `entropy.rs:589-591` computes `diff_i32 = block[0] - dc_predictor` and casts to `i16`. For large coefficient values near `i16::MAX/MIN`, this cast could overflow. The `encode_dc_coefficient` method then clamps to 11-bit range, but the intermediate `i16` cast at `entropy.rs:590` could already be problematic if the true difference exceeds `i16` range.

---

## Document: jpeg-stego-f5.md
### Verified Claims
- F5XorShiftRng is separate from XorShiftRng in util/image.rs — confirmed: F5XorShiftRng uses `x ^= x << 13; x ^= x >> 7; x ^= x << 17` (`stego_f5.rs:43-47`), XorShiftRng uses `x ^= x >> 12; x ^= x << 25; x ^= x >> 27; wrapping_mul(...)` (`util/image.rs:53-58`). Different algorithms, both with explicit warnings.
- Seed embedding: 12 bytes (4B magic "SEED" + 8B seed u64 LE) — confirmed at `stego_f5.rs:79-83`
- Q-table LSB clearing edge case: `&= 0xFE` on value 1 produces 0, clamped back to 1 — confirmed at `stego_f5.rs:103-108`
- No-zero variant: when |coef|==1 and LSB mismatches, increments absolute value (+1→+2, -1→-2) — confirmed at `stego_f5.rs:287-300`
- F5XorShiftRng is used for coefficient position shuffling — confirmed at `stego_f5.rs:222-227` (Fisher-Yates shuffle using F5XorShiftRng)
- Coefficient positions sorted before shuffling for deterministic order — confirmed at `stego_f5.rs:220` with comment about HashMap iteration order
- embed_f5 returns number of original bits embedded — confirmed at `stego_f5.rs:311` (returns `original_bit_count`)
- extract_f5 returns extracted bits without Result wrapper — confirmed at `stego_f5.rs:322` (returns `Vec<u8>`)
- Redundancy-based majority voting in extract_f5 — confirmed at `stego_f5.rs:357-371`
- Module interactions with steganography.rs, jpeg-transcoder.md, jpeg-header.md, jpeg-entropy.md — confirmed
- Module line count ~658 lines — actual is 658 lines

### Discrepancies
- **embed_f5 return type**: Doc says `Result<()>`. Actual is `Result<usize>` (`stego_f5.rs:183`), returning the original bit count before redundancy expansion.
- **extract_f5 return type**: Doc says `Result<Vec<u8>>`. Actual is `Vec<u8>` (`stego_f5.rs:322`) — no Result wrapper.
- **embed_seed_in_quantization_tables signature**: Doc says `pub fn embed_seed_in_quantization_tables(header: &mut JpegHeader, seed: u64) -> bool`. Actual (`stego_f5.rs:74-78`) takes `&self` as first parameter and returns `Result<()>`. The method is not a free function; it requires a `DctStegoF5` instance.
- **extract_seed_from_quantization_tables signature**: Doc says it's a static function. Actual (`stego_f5.rs:119`) takes `&self`. Same issue as above.
- **Majority voting: 5 passes**: Doc says "Extraction runs 5 passes with majority voting" under the F5 extraction section. The `extract_f5` method in `stego_f5.rs` does NOT implement 5-pass extraction. It does a single-pass extraction with redundancy-based majority voting (`stego_f5.rs:357-371`). The 5-pass logic exists in `steganography.rs:222` (`extract_with_redundancy`), which is a separate pixel-domain stego function, not part of F5 DCT stego. The architecture doc incorrectly attributes this to F5 extraction.
- **F5 embedding algorithm description**: Doc step 2 says "If LSB doesn't match, modify LSB to match payload bit" and separately describes shrinkage handling. The actual implementation (`stego_f5.rs:273-301`) treats |coef| >= 2 and |coef| == 1 differently, which the doc does describe but frames it as an alternative to standard F5 rather than the actual implementation path.

### Improvement Opportunities
- None identified (read-only review).

### Potential Bugs/Edge Cases
- **Seed embedding silent failure**: When too many quantization table values are 1, clearing LSBs (`&= 0xFE`) produces 0, which is clamped back to 1. This means the 0-bit cannot be reliably embedded. The seed embedding may fail silently — the function returns `Ok(())` even if some bits could not be embedded. The extract function would then return an incorrect seed. This is documented in the architecture doc but not guarded against in code.
- **HashMap iteration order**: `embed_f5` and `extract_f5` both sort positions before shuffling (`stego_f5.rs:220`, `stego_f5.rs:336`). This correctly handles HashMap's non-deterministic iteration order. However, if the coefficients HashMap is mutated between embed and extract calls in a way that changes which keys exist, the position lists could differ. This is an edge case that the current API contract (caller provides matching coefficient maps) handles correctly.
- **Capacity check race**: `embed_f5` checks `bits.len() > positions.len()` at `stego_f5.rs:229` before shuffling. After shuffling, some positions that were non-zero during collection might be zero at embed time if the coefficient map was mutated externally. The code handles this with `if current == 0 { continue; }` at `stego_f5.rs:259-261`, which means the capacity check could be optimistic.
- **F5XorShiftRng state=0**: The constructor handles `seed == 0` by setting state to 1 (`stego_f5.rs:39`). This prevents the PRNG from being stuck at 0, but the sequence for seed=0 is the same as seed=1. This is a minor collision.
- **Redundancy clamped to [1, 5]**: `DctStegoF5::with_redundancy` clamps to 1-5 (`stego_f5.rs:68`). The default redundancy is 3 (`stego_f5.rs:63`). The caller in `steganography.rs` uses `ctx.stego_redundancy().max(1)` which may exceed 5, but the clamp in `with_redundancy` limits it.

---

## Cross-Cutting Findings
- The Coefficients type uses `i16` inner arrays, not `i64` as documented. This limits coefficient magnitude to ±32767, which is sufficient for JPEG (max category is 11 bits = ±2047 for DC, 10 bits = ±1023 for AC) but would silently truncate if a malformed file decoded to larger values. The encoder's clamping at 11/10 bits provides an additional safety net.
- The transcoder module is well-separated: header parsing, entropy codec, and F5 stego are independent concerns with clean interfaces. The Coefficients type is the shared data structure between entropy and stego modules.
- Progressive JPEG handling is split across modules: the transcoder rejects progressive JPEGs (`mod.rs:58-65`), and the fallback logic (seed-only stego) lives in `steganography.rs:92-124`. This is clean separation but means the transcoder module has no progressive support at all.
- The F5 extraction's majority voting (`stego_f5.rs:357-371`) operates on the redundancy dimension only (repeated payload bits). The 5-pass extraction in `steganography.rs:222` is a separate mechanism using offset seeds for pixel-domain LSB stego, not DCT stego. These two mechanisms are independent.
- The `get_scan_data_start` function returning `Option<usize>` instead of `Result<usize>` means error context is lost. The caller converts to `Result` with a generic error message.
- The header parser preserves APP0 and APP1 markers verbatim, but `assemble_jpeg` reconstructs the JPEG from parsed data. APP1 (EXIF) is intentionally omitted during reassembly (`mod.rs:110-112` comment), which means EXIF data is lost during the DCT transcode path. The protection pipeline re-injects metadata separately.
