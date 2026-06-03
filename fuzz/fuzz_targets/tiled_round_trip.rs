#![no_main]

use cloakrs::{process_image_bytes, verify_image_bytes, ProtectionContext, ProtectionLevel};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if data.len() < 8 {
        return;
    }

    let tile_size = ((data[0] as u32 % 3) * 32 + 32).min(128);
    let seed = u64::from_le_bytes(data[..8].try_into().unwrap());

    let ctx = ProtectionContext::new(0.5, seed).with_tile_size(tile_size);

    let protected = process_image_bytes(data, ProtectionLevel::Standard, &ctx);
    if let Ok(ref protected_bytes) = protected {
        let _ = verify_image_bytes(protected_bytes, &[]);
        let _ = verify_image_bytes(protected_bytes, b"fuzz-key");
    }
});
