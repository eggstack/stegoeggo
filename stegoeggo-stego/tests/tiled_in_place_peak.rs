//! Peak auxiliary allocation regression test for tiled in-place embedding.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

use stegoeggo_stego::lsb::{self, TileConfig};

static ALLOCATED_BYTES: AtomicUsize = AtomicUsize::new(0);
static PEAK_BYTES: AtomicUsize = AtomicUsize::new(0);

struct CountingAllocator;

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            let in_use =
                ALLOCATED_BYTES.fetch_add(layout.size(), Ordering::Relaxed) + layout.size();
            let mut peak = PEAK_BYTES.load(Ordering::Relaxed);
            while in_use > peak {
                match PEAK_BYTES.compare_exchange_weak(
                    peak,
                    in_use,
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => break,
                    Err(observed) => peak = observed,
                }
            }
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) };
        ALLOCATED_BYTES.fetch_sub(layout.size(), Ordering::Relaxed);
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

fn reset_counters() {
    ALLOCATED_BYTES.store(0, Ordering::Relaxed);
    PEAK_BYTES.store(0, Ordering::Relaxed);
}

fn textured_rgba(size: u32) -> image::RgbaImage {
    image::RgbaImage::from_fn(size, size, |x, y| {
        image::Rgba([
            ((x * 7 + y * 13) % 251) as u8,
            ((x * 11 + y * 3) % 251) as u8,
            ((x * 5 + y * 17) % 251) as u8,
            255,
        ])
    })
}

#[test]
fn tiled_in_place_peak_aux_stays_far_below_image_bytes() {
    let size = 512u32;
    let image_bytes = size as usize * size as usize * 4;
    let payload = vec![0xA5u8; 36];
    let config = TileConfig::try_new(42, 64).unwrap();
    let mut warm = textured_rgba(size);
    lsb::embed_tiled_in_place(&mut warm, &payload, &config).unwrap();
    drop(warm);

    let mut image = textured_rgba(size);
    reset_counters();
    let report = lsb::embed_tiled_in_place(&mut image, &payload, &config).unwrap();
    assert!(report.embedded);
    let peak = PEAK_BYTES.load(Ordering::Relaxed);
    eprintln!("tiled in-place peak aux bytes: {peak} (image bytes: {image_bytes})");
    assert!(
        peak < image_bytes / 4,
        "peak aux {peak} exceeds a quarter of image bytes {image_bytes}"
    );
    let recovered = lsb::extract_tiled(&image, payload.len(), &config, 64).unwrap();
    assert_eq!(recovered, payload);
}
