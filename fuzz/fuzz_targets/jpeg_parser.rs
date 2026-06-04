#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }

    // Exercise the JPEG header parser
    let _ = cloakrs::parse_jpeg_for_fuzz(data);

    // Also exercise the progressive detection
    let _ = cloakrs::is_progressive_jpeg(data);
});
