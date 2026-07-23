#![no_main]
use libfuzzer_sys::fuzz_target;
use stegoeggo::MetadataTrapProtector;

fuzz_target!(|data: &[u8]| {
    let _ = MetadataTrapProtector::extract_seed_from_image(data);
    let _ = stegoeggo::verify_image_bytes(data, &[]);
});
