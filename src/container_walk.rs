use crate::error::Result;
use crate::types::ImageOutputFormat;

pub(crate) fn observe_container_work(
    bytes: &[u8],
    format: ImageOutputFormat,
    budget: &mut crate::resource_limits::OperationObserver,
) -> Result<()> {
    match format {
        ImageOutputFormat::Png => observe_png_work(bytes, budget),
        ImageOutputFormat::Jpeg => observe_jpeg_work(bytes, budget),
        ImageOutputFormat::WebP => observe_webp_work(bytes, budget),
    }
}

fn observe_png_work(
    img_bytes: &[u8],
    budget: &mut crate::resource_limits::OperationObserver,
) -> Result<()> {
    if img_bytes.len() < 8 || &img_bytes[0..8] != b"\x89PNG\r\n\x1a\n" {
        return Ok(());
    }
    let mut pos = 8;
    while pos + 12 <= img_bytes.len() {
        let chunk_len = u32::from_be_bytes([
            img_bytes[pos],
            img_bytes[pos + 1],
            img_bytes[pos + 2],
            img_bytes[pos + 3],
        ]) as usize;
        let chunk_type = &img_bytes[pos + 4..pos + 8];
        if chunk_type == b"IEND" {
            break;
        }
        let Some(chunk_total) = chunk_len.checked_add(12) else {
            break;
        };
        budget.observe_png_chunk(chunk_total);
        let data_start = pos + 8;
        let data_end = data_start
            .checked_add(chunk_len)
            .unwrap_or(img_bytes.len())
            .min(img_bytes.len());
        if (chunk_type == b"tEXt" || chunk_type == b"iTXt") && data_end > data_start {
            budget.observe_metadata_field(data_end - data_start);
        }
        pos = match pos.checked_add(chunk_total) {
            Some(next) => next,
            None => break,
        };
        if pos > img_bytes.len() {
            break;
        }
    }
    budget.check_limits()
}

fn observe_jpeg_work(
    img_bytes: &[u8],
    budget: &mut crate::resource_limits::OperationObserver,
) -> Result<()> {
    if img_bytes.len() < 2 || img_bytes[0] != 0xFF || img_bytes[1] != 0xD8 {
        return Ok(());
    }
    let mut pos = 2;
    while pos + 2 <= img_bytes.len() {
        if img_bytes[pos] != 0xFF {
            pos += 1;
            continue;
        }
        let marker = img_bytes[pos + 1];
        if marker == 0xD9 || marker == 0xDA {
            break;
        }
        if marker == 0x00 {
            pos += 1;
            continue;
        }
        if pos + 4 > img_bytes.len() {
            break;
        }
        let seg_len = u16::from_be_bytes([img_bytes[pos + 2], img_bytes[pos + 3]]) as usize;
        let Some(seg_end) = pos.checked_add(2).and_then(|p| p.checked_add(seg_len)) else {
            break;
        };
        if seg_end > img_bytes.len() {
            break;
        }
        budget.observe_jpeg_segment(seg_end - pos);
        if matches!(marker, 0xE1 | 0xED | 0xFE) {
            budget.observe_metadata_field(seg_len.saturating_sub(2));
        }
        pos = seg_end;
    }
    budget.check_limits()
}

fn observe_webp_work(
    img_bytes: &[u8],
    budget: &mut crate::resource_limits::OperationObserver,
) -> Result<()> {
    if img_bytes.len() < 12 || &img_bytes[0..4] != b"RIFF" || &img_bytes[8..12] != b"WEBP" {
        return Ok(());
    }
    let mut pos = 12;
    while pos + 8 <= img_bytes.len() {
        let chunk_size = u32::from_le_bytes([
            img_bytes[pos + 4],
            img_bytes[pos + 5],
            img_bytes[pos + 6],
            img_bytes[pos + 7],
        ]) as usize;
        let Some(padded) = chunk_size.checked_add(chunk_size & 1) else {
            budget.observe_webp_chunk(img_bytes.len() - pos);
            break;
        };
        let Some(chunk_end) = pos.checked_add(8).and_then(|p| p.checked_add(padded)) else {
            budget.observe_webp_chunk(img_bytes.len() - pos);
            break;
        };
        if chunk_end > img_bytes.len() {
            budget.observe_webp_chunk(img_bytes.len() - pos);
            break;
        }
        budget.observe_webp_chunk(chunk_end - pos);
        pos = chunk_end;
    }
    budget.check_limits()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resource_limits::ResourceLimits;

    fn observer(limits: &ResourceLimits) -> crate::resource_limits::OperationObserver {
        crate::resource_limits::OperationObserver::new(limits, 0)
    }

    fn png_chunk(chunk_type: &[u8; 4], data: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        out.extend_from_slice(chunk_type);
        out.extend_from_slice(data);
        out.extend_from_slice(&[0, 0, 0, 0]);
        out
    }

    fn minimal_png(chunks: &[Vec<u8>]) -> Vec<u8> {
        let mut out = Vec::from(b"\x89PNG\r\n\x1a\n".as_slice());
        for c in chunks {
            out.extend_from_slice(c);
        }
        out
    }

    fn jpeg_segment(marker: u8, payload: &[u8]) -> Vec<u8> {
        let mut out = vec![0xFF, marker];
        let len = (payload.len() + 2) as u16;
        out.extend_from_slice(&len.to_be_bytes());
        out.extend_from_slice(payload);
        out
    }

    fn webp_chunk(fourcc: &[u8; 4], payload: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(fourcc);
        out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        out.extend_from_slice(payload);
        if payload.len() & 1 != 0 {
            out.push(0);
        }
        out
    }

    fn webp_file(chunks: &[Vec<u8>]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&[0, 0, 0, 0]);
        out.extend_from_slice(b"WEBP");
        for c in chunks {
            out.extend_from_slice(c);
        }
        let riff_size = (out.len() - 8) as u32;
        out[4..8].copy_from_slice(&riff_size.to_le_bytes());
        out
    }

    #[test]
    fn png_counts_chunks_and_text_fields() {
        let png = minimal_png(&[
            png_chunk(b"IHDR", &[0; 13]),
            png_chunk(b"tEXt", b"Comment\x00hello"),
            png_chunk(b"IDAT", &[1, 2, 3, 4]),
            png_chunk(b"IEND", &[]),
        ]);
        let limits = ResourceLimits::default();
        let mut budget = observer(&limits);
        observe_container_work(&png, ImageOutputFormat::Png, &mut budget).unwrap();
        let usage = budget.finish(png.len());
        assert_eq!(usage.png_chunks_scanned, 3);
        assert_eq!(usage.metadata_fields_extracted, 1);
        assert_eq!(usage.metadata_bytes_copied, b"Comment\x00hello".len());
        assert_eq!(usage.jpeg_segments_scanned, 0);
        assert_eq!(usage.webp_riff_chunks_scanned, 0);
    }

    #[test]
    fn png_itxt_counts_as_metadata_field() {
        let png = minimal_png(&[
            png_chunk(b"IHDR", &[0; 13]),
            png_chunk(b"iTXt", b"XML:com.adobe.xmp\x00\x00\x00\x00\x00<x/>"),
            png_chunk(b"IEND", &[]),
        ]);
        let limits = ResourceLimits::default();
        let mut budget = observer(&limits);
        observe_container_work(&png, ImageOutputFormat::Png, &mut budget).unwrap();
        let usage = budget.finish(png.len());
        assert_eq!(usage.png_chunks_scanned, 2);
        assert_eq!(usage.metadata_fields_extracted, 1);
    }

    #[test]
    fn png_stops_at_iend() {
        let mut png = minimal_png(&[png_chunk(b"IHDR", &[0; 13]), png_chunk(b"IEND", &[])]);
        png.extend_from_slice(&png_chunk(b"tEXt", b"After\x00x"));
        let limits = ResourceLimits::default();
        let mut budget = observer(&limits);
        observe_container_work(&png, ImageOutputFormat::Png, &mut budget).unwrap();
        let usage = budget.finish(png.len());
        assert_eq!(usage.png_chunks_scanned, 1);
        assert_eq!(usage.metadata_fields_extracted, 0);
    }

    #[test]
    fn png_truncated_chunk_header_is_lenient() {
        let mut png = Vec::from(b"\x89PNG\r\n\x1a\n".as_slice());
        png.extend_from_slice(&[0, 0, 0]);
        let limits = ResourceLimits::default();
        let mut budget = observer(&limits);
        observe_container_work(&png, ImageOutputFormat::Png, &mut budget).unwrap();
        let usage = budget.finish(png.len());
        assert_eq!(usage.png_chunks_scanned, 0);
    }

    #[test]
    fn png_oversized_declared_length_does_not_panic() {
        let mut png = Vec::from(b"\x89PNG\r\n\x1a\n".as_slice());
        png.extend_from_slice(&0xFFFFFFFFu32.to_be_bytes());
        png.extend_from_slice(b"tEXt");
        png.extend_from_slice(&[0; 8]);
        let limits = ResourceLimits::default();
        let mut budget = observer(&limits);
        observe_container_work(&png, ImageOutputFormat::Png, &mut budget).unwrap();
        let usage = budget.finish(png.len());
        assert_eq!(usage.png_chunks_scanned, 1);
    }

    #[test]
    fn png_chunk_limit_at_and_beyond_threshold() {
        let chunks: Vec<Vec<u8>> = (0..5).map(|i| png_chunk(b"IDAT", &[i; 4])).collect();
        let mut all = vec![png_chunk(b"IHDR", &[0; 13])];
        all.extend(chunks);
        all.push(png_chunk(b"IEND", &[]));
        let png = minimal_png(&all);
        let at_limits = ResourceLimits::builder().max_png_chunks(6).build();
        let mut budget = observer(&at_limits);
        assert!(observe_container_work(&png, ImageOutputFormat::Png, &mut budget).is_ok());
        let beyond_limits = ResourceLimits::builder().max_png_chunks(5).build();
        let mut budget = observer(&beyond_limits);
        let err = observe_container_work(&png, ImageOutputFormat::Png, &mut budget).unwrap_err();
        assert!(matches!(err, crate::Error::ContainerLimitExceeded { .. }));
    }

    #[test]
    fn jpeg_counts_segments_and_metadata_markers() {
        let mut jpeg = vec![0xFF, 0xD8];
        jpeg.extend_from_slice(&jpeg_segment(0xFE, b"hello"));
        jpeg.extend_from_slice(&jpeg_segment(0xE0, b"JFIF"));
        jpeg.extend_from_slice(&[0xFF, 0xDA]);
        jpeg.extend_from_slice(&[0; 10]);
        let limits = ResourceLimits::default();
        let mut budget = observer(&limits);
        observe_container_work(&jpeg, ImageOutputFormat::Jpeg, &mut budget).unwrap();
        let usage = budget.finish(jpeg.len());
        assert_eq!(usage.jpeg_segments_scanned, 2);
        assert_eq!(usage.metadata_fields_extracted, 1);
        assert_eq!(usage.metadata_bytes_copied, b"hello".len());
    }

    #[test]
    fn jpeg_sos_stops_scan_and_ignores_entropy() {
        let mut jpeg = vec![0xFF, 0xD8];
        jpeg.extend_from_slice(&jpeg_segment(0xFE, b"before"));
        jpeg.extend_from_slice(&[0xFF, 0xDA]);
        jpeg.extend_from_slice(&[0xFF, 0xFE, 0x00, 0x06, b'x', b'y', b'z', b'w']);
        jpeg.extend_from_slice(&[0xFF, 0xD9]);
        let limits = ResourceLimits::default();
        let mut budget = observer(&limits);
        observe_container_work(&jpeg, ImageOutputFormat::Jpeg, &mut budget).unwrap();
        let usage = budget.finish(jpeg.len());
        assert_eq!(usage.jpeg_segments_scanned, 1);
        assert_eq!(usage.metadata_fields_extracted, 1);
    }

    #[test]
    fn jpeg_truncated_segment_header_is_lenient() {
        let jpeg = vec![0xFF, 0xD8, 0xFF, 0xFE, 0x00];
        let limits = ResourceLimits::default();
        let mut budget = observer(&limits);
        observe_container_work(&jpeg, ImageOutputFormat::Jpeg, &mut budget).unwrap();
        let usage = budget.finish(jpeg.len());
        assert_eq!(usage.jpeg_segments_scanned, 0);
    }

    #[test]
    fn jpeg_oversized_segment_length_is_lenient() {
        let jpeg = vec![0xFF, 0xD8, 0xFF, 0xFE, 0xFF, 0xFF, 0x00];
        let limits = ResourceLimits::default();
        let mut budget = observer(&limits);
        observe_container_work(&jpeg, ImageOutputFormat::Jpeg, &mut budget).unwrap();
        let usage = budget.finish(jpeg.len());
        assert_eq!(usage.jpeg_segments_scanned, 0);
    }

    #[test]
    fn jpeg_segment_limit_at_and_beyond_threshold() {
        let mut jpeg = vec![0xFF, 0xD8];
        for i in 0..4u8 {
            jpeg.extend_from_slice(&jpeg_segment(0xFE, &[i; 4]));
        }
        jpeg.extend_from_slice(&[0xFF, 0xD9]);
        let at_limits = ResourceLimits::builder().max_jpeg_segments(4).build();
        let mut budget = observer(&at_limits);
        assert!(observe_container_work(&jpeg, ImageOutputFormat::Jpeg, &mut budget).is_ok());
        let beyond_limits = ResourceLimits::builder().max_jpeg_segments(3).build();
        let mut budget = observer(&beyond_limits);
        let err = observe_container_work(&jpeg, ImageOutputFormat::Jpeg, &mut budget).unwrap_err();
        assert!(matches!(err, crate::Error::ContainerLimitExceeded { .. }));
    }

    #[test]
    fn webp_counts_chunks_with_odd_padding() {
        let webp = webp_file(&[
            webp_chunk(b"VP8 ", &[1, 2, 3]),
            webp_chunk(b"XMP ", &[9; 5]),
        ]);
        let limits = ResourceLimits::default();
        let mut budget = observer(&limits);
        observe_container_work(&webp, ImageOutputFormat::WebP, &mut budget).unwrap();
        let usage = budget.finish(webp.len());
        assert_eq!(usage.webp_riff_chunks_scanned, 2);
        assert_eq!(usage.png_chunks_scanned, 0);
    }

    #[test]
    fn webp_truncated_tail_observes_partial() {
        let mut webp = webp_file(&[webp_chunk(b"VP8 ", &[0; 8])]);
        webp.truncate(webp.len() - 3);
        let limits = ResourceLimits::default();
        let mut budget = observer(&limits);
        observe_container_work(&webp, ImageOutputFormat::WebP, &mut budget).unwrap();
        let usage = budget.finish(webp.len());
        assert_eq!(usage.webp_riff_chunks_scanned, 1);
    }

    #[test]
    fn webp_oversized_chunk_size_is_lenient() {
        let mut webp = Vec::new();
        webp.extend_from_slice(b"RIFF");
        webp.extend_from_slice(&[0, 0, 0, 0]);
        webp.extend_from_slice(b"WEBP");
        webp.extend_from_slice(b"VP8 ");
        webp.extend_from_slice(&0xFFFFFFFFu32.to_le_bytes());
        webp.extend_from_slice(&[0; 8]);
        let limits = ResourceLimits::default();
        let mut budget = observer(&limits);
        observe_container_work(&webp, ImageOutputFormat::WebP, &mut budget).unwrap();
        let usage = budget.finish(webp.len());
        assert_eq!(usage.webp_riff_chunks_scanned, 1);
    }

    #[test]
    fn webp_chunk_limit_at_and_beyond_threshold() {
        let chunks: Vec<Vec<u8>> = (0..4).map(|i| webp_chunk(b"VP8 ", &[i; 4])).collect();
        let webp = webp_file(&chunks);
        let at_limits = ResourceLimits::builder().max_webp_riff_chunks(4).build();
        let mut budget = observer(&at_limits);
        assert!(observe_container_work(&webp, ImageOutputFormat::WebP, &mut budget).is_ok());
        let beyond_limits = ResourceLimits::builder().max_webp_riff_chunks(3).build();
        let mut budget = observer(&beyond_limits);
        let err = observe_container_work(&webp, ImageOutputFormat::WebP, &mut budget).unwrap_err();
        assert!(matches!(err, crate::Error::ContainerLimitExceeded { .. }));
    }

    #[test]
    fn metadata_field_limit_triggers_on_large_text_payload() {
        let big = vec![b'x'; 9000];
        let mut payload = b"Comment\x00".to_vec();
        payload.extend_from_slice(&big);
        let png = minimal_png(&[png_chunk(b"IHDR", &[0; 13]), png_chunk(b"tEXt", &payload)]);
        let limits = ResourceLimits::builder()
            .max_metadata_field_bytes(8192)
            .build();
        let mut budget = observer(&limits);
        let err = observe_container_work(&png, ImageOutputFormat::Png, &mut budget).unwrap_err();
        assert!(matches!(err, crate::Error::MetadataLimitExceeded { .. }));
    }
}
