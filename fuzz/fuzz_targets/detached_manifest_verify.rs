#![no_main]
use libfuzzer_sys::fuzz_target;
use stegoeggo::detached::{verify_detached_manifest, DetachedManifest, TrustPolicy};

fuzz_target!(|data: &[u8]| {
    if let Ok(manifest) = DetachedManifest::from_json(data) {
        let _ = verify_detached_manifest(data, &manifest, &TrustPolicy::TrustNone);
    }
});
