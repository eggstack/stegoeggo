#![no_main]
use libfuzzer_sys::fuzz_target;
use stegoeggo::RightsMetadataProtector;

fuzz_target!(|data: &[u8]| {
    let _ = RightsMetadataProtector::extract_seed_from_image(data);
    let _ = stegoeggo::verify_image_bytes(data, &[]);
});
