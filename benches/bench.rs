use stegoeggo::{
    process_image_bytes, ImageOutputFormat, ProtectionContext, ProtectionLevel, ProtectionPipeline,
};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use image::DynamicImage;
use std::alloc::System;
use std::sync::atomic::{AtomicUsize, Ordering};

static ALLOCATION_COUNT: AtomicUsize = AtomicUsize::new(0);
static ALLOCATED_BYTES: AtomicUsize = AtomicUsize::new(0);

struct AllocationTracker;

#[global_allocator]
static ALLOCATOR: AllocationTracker = AllocationTracker;

unsafe impl std::alloc::GlobalAlloc for AllocationTracker {
    unsafe fn alloc(&self, layout: std::alloc::Layout) -> *mut u8 {
        ALLOCATION_COUNT.fetch_add(1, Ordering::Relaxed);
        ALLOCATED_BYTES.fetch_add(layout.size(), Ordering::Relaxed);
        System.alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: std::alloc::Layout) {
        System.dealloc(ptr, layout)
    }
}

fn reset_counters() {
    ALLOCATION_COUNT.store(0, Ordering::Relaxed);
    ALLOCATED_BYTES.store(0, Ordering::Relaxed);
}

fn get_allocation_count() -> usize {
    ALLOCATION_COUNT.load(Ordering::Relaxed)
}

fn get_allocated_bytes() -> usize {
    ALLOCATED_BYTES.load(Ordering::Relaxed)
}

fn create_test_image(width: u32, height: u32) -> DynamicImage {
    DynamicImage::new_rgb8(width, height)
}

fn benchmark_pipeline_sizes(c: &mut Criterion) {
    let pipeline = ProtectionPipeline::new();
    let ctx = ProtectionContext::default();

    // Sizes cover both typical WAF responses (256–1024 px) and CDN origin
    // images (2K / 4K). 8K (7680×4320) is intentionally not included here:
    // a single 32-bit RGBA frame at 8K is ~132 MB, which makes per-iteration
    // allocation noise dominate the signal. If you need that data point, run
    //   cargo bench --bench bench -- 'large_image_bytes' --warm-up-time 5
    // from a workstation with >16 GB RAM.
    let sizes = [
        (256, "256"),
        (512, "512"),
        (1024, "1024"),
        (2560, "2k"),
        (3840, "4k"),
    ];

    let mut group = c.benchmark_group("pipeline_image_sizes");
    group.sample_size(10);

    for (size, label) in sizes.iter() {
        let img = create_test_image(*size, *size);

        for level in [ProtectionLevel::Light, ProtectionLevel::Standard] {
            let id = format!("{}_{:?}", label, level);
            group.bench_with_input(BenchmarkId::new("image_size", id), &level, |b, &level| {
                b.iter(|| pipeline.process(black_box(&img), level, black_box(&ctx)));
            });
        }
    }

    group.finish();
}

fn benchmark_protection_levels(c: &mut Criterion) {
    let pipeline = ProtectionPipeline::new();
    let ctx = ProtectionContext::default();

    let img_512 = create_test_image(512, 512);

    let mut group = c.benchmark_group("protection_levels_512");

    for level in [
        ProtectionLevel::Disabled,
        ProtectionLevel::Light,
        ProtectionLevel::Standard,
    ] {
        group.bench_with_input(
            BenchmarkId::new("level", level.as_str()),
            &level,
            |b, &level| {
                b.iter(|| pipeline.process(black_box(&img_512), level, black_box(&ctx)));
            },
        );
    }

    group.finish();
}

fn benchmark_bytes_processing(c: &mut Criterion) {
    let img = create_test_image(512, 512);
    let mut png_bytes = Vec::new();
    {
        use image::ImageEncoder;
        let encoder = image::codecs::png::PngEncoder::new(&mut png_bytes);
        encoder
            .write_image(&img.to_rgb8(), 512, 512, image::ExtendedColorType::Rgb8)
            .unwrap();
    }

    let ctx = ProtectionContext::default();

    let mut group = c.benchmark_group("process_bytes");

    for level in [ProtectionLevel::Light, ProtectionLevel::Standard] {
        group.bench_with_input(
            BenchmarkId::new("png_512", level.as_str()),
            &level,
            |b, &level| {
                b.iter(|| process_image_bytes(black_box(&png_bytes), level, black_box(&ctx)));
            },
        );
    }

    group.finish();
}

/// End-to-end bytes-in / bytes-out at CDN-image sizes (2K and 4K). This is the
/// most production-relevant group: a WAF origin request sees bytes, not a
/// `DynamicImage`, and PNG-in / PNG-out with stego + metadata is the
/// "maximum legal evidence" path documented in the README.
fn benchmark_large_image_bytes(c: &mut Criterion) {
    let ctx = ProtectionContext::default();
    let mut group = c.benchmark_group("large_image_bytes");
    group.sample_size(10);

    for &(size, label) in &[(2560u32, "2k"), (3840u32, "4k")] {
        let img = create_test_image(size, size);
        let mut png_bytes = Vec::new();
        {
            use image::ImageEncoder;
            let encoder = image::codecs::png::PngEncoder::new(&mut png_bytes);
            encoder
                .write_image(&img.to_rgb8(), size, size, image::ExtendedColorType::Rgb8)
                .unwrap();
        }

        for level in [ProtectionLevel::Light, ProtectionLevel::Standard] {
            let id = format!("png_{}_{}", label, level.as_str());
            group.bench_with_input(BenchmarkId::new("large", id), &level, |b, &level| {
                b.iter(|| process_image_bytes(black_box(&png_bytes), level, black_box(&ctx)));
            });
        }
    }

    group.finish();
}

fn benchmark_format_preservation(c: &mut Criterion) {
    let img = create_test_image(256, 256);

    let mut png_bytes = Vec::new();
    let mut jpeg_bytes = Vec::new();
    {
        use image::ImageEncoder;

        let encoder = image::codecs::png::PngEncoder::new(&mut png_bytes);
        encoder
            .write_image(&img.to_rgb8(), 256, 256, image::ExtendedColorType::Rgb8)
            .unwrap();

        let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg_bytes, 85);
        encoder
            .write_image(&img.to_rgb8(), 256, 256, image::ExtendedColorType::Rgb8)
            .unwrap();
    }

    let ctx_png = ProtectionContext::default();
    let ctx_jpeg = ProtectionContext::new(0.5, 42).with_format(ImageOutputFormat::Jpeg);

    let mut group = c.benchmark_group("format_preservation");

    group.bench_function("png_in_png_out", |b| {
        b.iter(|| {
            process_image_bytes(
                black_box(&png_bytes),
                ProtectionLevel::Light,
                black_box(&ctx_png),
            )
        });
    });

    group.bench_function("jpeg_in_jpeg_out", |b| {
        b.iter(|| {
            process_image_bytes(
                black_box(&jpeg_bytes),
                ProtectionLevel::Light,
                black_box(&ctx_jpeg),
            )
        });
    });

    group.finish();
}

fn benchmark_allocations(c: &mut Criterion) {
    let pipeline = ProtectionPipeline::new();
    let ctx = ProtectionContext::default();
    let img_512 = create_test_image(512, 512);

    let mut group = c.benchmark_group("allocations_512x512");

    {
        let level = ProtectionLevel::Standard;
        reset_counters();
        let _ = pipeline.process(&img_512, level, &ctx);
        let count = get_allocation_count();
        let bytes = get_allocated_bytes();

        group.bench_function(level.as_str(), |b| {
            b.iter(|| {
                reset_counters();
                let _ = pipeline.process(black_box(&img_512), level, black_box(&ctx));
                get_allocation_count()
            });
        });

        eprintln!("{:?}: {} allocations, {} bytes", level, count, bytes);
    }

    group.finish();
}

fn benchmark_memory_usage(c: &mut Criterion) {
    let pipeline = ProtectionPipeline::new();
    let ctx = ProtectionContext::default();
    let img_512 = create_test_image(512, 512);

    let mut group = c.benchmark_group("memory_peak_512x512");

    for level in [
        ProtectionLevel::Disabled,
        ProtectionLevel::Light,
        ProtectionLevel::Standard,
    ] {
        group.bench_function(level.as_str(), |b| {
            b.iter(|| {
                let _ = pipeline.process(black_box(&img_512), level, black_box(&ctx));
                0
            });
        });
    }

    group.finish();
}

fn benchmark_jpeg_fast_path(c: &mut Criterion) {
    let img = create_test_image(512, 512);

    let mut jpeg_bytes = Vec::new();
    {
        use image::ImageEncoder;
        let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg_bytes, 85);
        encoder
            .write_image(&img.to_rgb8(), 512, 512, image::ExtendedColorType::Rgb8)
            .unwrap();
    }

    let ctx = ProtectionContext::new(0.5, 42).with_format(ImageOutputFormat::Jpeg);

    let mut group = c.benchmark_group("jpeg_fast_path_512x512");

    {
        let level = ProtectionLevel::Standard;
        group.bench_with_input(
            BenchmarkId::new("jpeg_in_out", level.as_str()),
            &level,
            |b, &level| {
                b.iter(|| process_image_bytes(black_box(&jpeg_bytes), level, black_box(&ctx)));
            },
        );
    }

    group.finish();
}

fn benchmark_tiled_embed(c: &mut Criterion) {
    let mut group = c.benchmark_group("tiled_embed");

    for size in [256u32, 1024] {
        let img = create_test_image(size, size);
        let jpeg_bytes = {
            let mut buf = std::io::Cursor::new(Vec::new());
            img.write_to(&mut buf, image::ImageFormat::Jpeg).unwrap();
            buf.into_inner()
        };

        let ctx = ProtectionContext::new(0.5, 42)
            .with_tile_size(64)
            .with_format(ImageOutputFormat::Jpeg);

        group.bench_with_input(
            BenchmarkId::new("jpeg_tiled", size),
            &(&jpeg_bytes, &ctx),
            |b, &(bytes, ctx)| {
                b.iter(|| {
                    process_image_bytes(black_box(bytes), ProtectionLevel::Standard, black_box(ctx))
                });
            },
        );
    }

    group.finish();
}

fn benchmark_tiled_extract(c: &mut Criterion) {
    let mut group = c.benchmark_group("tiled_extract");

    for size in [256u32, 1024] {
        let img = create_test_image(size, size);
        let ctx = ProtectionContext::new(0.5, 42)
            .with_tile_size(64)
            .with_format(ImageOutputFormat::Jpeg);

        let protected = process_image_bytes(
            &{
                let mut buf = std::io::Cursor::new(Vec::new());
                img.write_to(&mut buf, image::ImageFormat::Jpeg).unwrap();
                buf.into_inner()
            },
            ProtectionLevel::Standard,
            &ctx,
        )
        .unwrap();

        group.bench_with_input(
            BenchmarkId::new("jpeg_tiled", size),
            &protected,
            |b, bytes| {
                b.iter(|| stegoeggo::verify_image_bytes(black_box(bytes), &[]));
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    benchmark_pipeline_sizes,
    benchmark_protection_levels,
    benchmark_bytes_processing,
    benchmark_large_image_bytes,
    benchmark_format_preservation,
    benchmark_allocations,
    benchmark_memory_usage,
    benchmark_jpeg_fast_path,
    benchmark_tiled_embed,
    benchmark_tiled_extract,
);
criterion_main!(benches);
