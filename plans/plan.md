# Cloakrs Implementation Plan

## Status: Completed

All items from all waves have been implemented and verified.

---

## Summary

| Wave | Items | Status |
|------|-------|--------|
| 1A | 1A.1 (stego redundancy bug), 1A.2 (JPEG segment bounds) | ✅ Complete |
| 1B | 1B.1 (JPEG segment truncation) | ✅ Complete |
| 1C | 1C.1 (division by zero assertion) | ✅ Complete |
| 2 | 2.1 (From<TranscoderError>), 2.2 (remove dead error variants) | ✅ Complete |
| 3 | 3.1 (bits_to_bytes runtime check) | ✅ Complete |
| 4 | 4.1 (batch helper), 4.2 (error messages) | ✅ Complete |
| 5 | 5.1 (redundancy test), 5.2 (error variant test) | ✅ Complete |

**Total: 11 items across 5 waves — all completed**

---

## Verification Commands

```bash
cargo test --all-features
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

---

## Prior Items (Completed)

- Dimension validation in `process_bytes`
- Seed embedding unit quant error
- `Option<bool>` documentation
- verify_image_bytes DCT stego

---

## File Reference

| File | Items |
|------|-------|
| `src/protected/steganography.rs` | 1A.1, 1A.2, 3.1, 5.1 |
| `src/protected/metadata_trap.rs` | 1B.1 |
| `src/util/image.rs` | 1C.1 |
| `src/error.rs` | 1B.1, 2.1, 2.2, 5.2 |
| `cloakrs-cli/src/main.rs` | 4.1, 4.2 |