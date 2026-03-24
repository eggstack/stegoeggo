use cloakrs::{
    process_image_bytes, ImageOutputFormat, ProtectionContext, ProtectionLevel, ProtectionPipeline,
    TargetModel,
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

    let sizes = [(256, "256"), (512, "512"), (1024, "1024")];

    let mut group = c.benchmark_group("pipeline_image_sizes");

    for (size, label) in sizes.iter() {
        let img = create_test_image(*size, *size);

        for level in [
            ProtectionLevel::Light,
            ProtectionLevel::Standard,
            ProtectionLevel::Strong,
        ] {
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
        ProtectionLevel::Strong,
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

    for level in [
        ProtectionLevel::Light,
        ProtectionLevel::Standard,
        ProtectionLevel::Strong,
    ] {
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
    let ctx_jpeg = ProtectionContext::new(TargetModel::StableDiffusionXL, 0.5, 42)
        .with_format(ImageOutputFormat::Jpeg);

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

    for level in [ProtectionLevel::Standard, ProtectionLevel::Enhanced] {
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
        ProtectionLevel::Enhanced,
        ProtectionLevel::Strong,
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

criterion_group!(
    benches,
    benchmark_pipeline_sizes,
    benchmark_protection_levels,
    benchmark_bytes_processing,
    benchmark_format_preservation,
    benchmark_allocations,
    benchmark_memory_usage
);
criterion_main!(benches);
