#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Fuzz the v3 payload parser with arbitrary bytes.
    // This covers v1/v2/v3 dispatch and TLV parsing.
    let _ = stegoeggo::payload_v3::parse_payload(data);
});
