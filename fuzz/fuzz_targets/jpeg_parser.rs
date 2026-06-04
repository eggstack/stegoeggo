#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }

    // Exercise the JPEG header parser
    let _ = stegoeggo::parse_jpeg_for_fuzz(data);

    // Also exercise the progressive detection
    let _ = stegoeggo::is_progressive_jpeg(data);
});
