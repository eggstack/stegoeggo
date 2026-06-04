#![no_main]

use libfuzzer_sys::fuzz_target;
use stegoeggo::{process_image_bytes, verify_image_bytes, ProtectionContext, ProtectionLevel};

fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }

    for level in [
        ProtectionLevel::Disabled,
        ProtectionLevel::Light,
        ProtectionLevel::Standard,
    ] {
        let _ = process_image_bytes(data, level, &ProtectionContext::default());
    }

    let _ = verify_image_bytes(data, &[]);
    let _ = verify_image_bytes(data, b"some-fuzz-key");
});
