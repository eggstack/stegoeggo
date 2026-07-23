#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = stegoeggo::verify_legal_notice(data, &[]);
    let _ = stegoeggo::process_image_bytes(
        data,
        stegoeggo::ProtectionLevel::Standard,
        &stegoeggo::ProtectionContext::default(),
    );
});
