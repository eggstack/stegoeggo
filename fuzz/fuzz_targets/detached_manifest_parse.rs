#![no_main]
use libfuzzer_sys::fuzz_target;
use stegoeggo::detached::DetachedManifest;

fuzz_target!(|data: &[u8]| {
    let _ = DetachedManifest::from_json(data);
});
