#![no_main]
use libfuzzer_sys::fuzz_target;
use stegoeggo::detached::DetachedManifest;

fuzz_target!(|data: &[u8]| {
    if let Ok(manifest) = DetachedManifest::from_json(data) {
        let _ = manifest.canonical_bytes();
    }
});
