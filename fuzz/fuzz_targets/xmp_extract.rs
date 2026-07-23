#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = stegoeggo::verify_legal_notice(data, &[]);
    let _ = stegoeggo::verify_legal_notice(data, b"fuzz-key");
});
