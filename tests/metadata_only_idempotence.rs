use stegoeggo::{
    process_request_bytes, ImageOutputFormat, LegalMetadata, MetadataUpdatePolicy,
    ProcessingOptions, ProtectionRequest, RightsNotice, RightsPolicy,
};

fn make_test_image_png(width: u32, height: u32) -> Vec<u8> {
    let img = image::DynamicImage::new_rgb8(width, height);
    let mut buf = std::io::Cursor::new(Vec::new());
    img.write_to(&mut buf, image::ImageFormat::Png).unwrap();
    buf.into_inner()
}

fn count_png_text_chunks(png: &[u8], keyword: &[u8]) -> usize {
    if png.len() < 8 || &png[0..8] != b"\x89PNG\r\n\x1a\n" {
        return 0;
    }
    let mut pos = 8;
    let mut count = 0;
    while pos + 12 <= png.len() {
        let chunk_len =
            u32::from_be_bytes([png[pos], png[pos + 1], png[pos + 2], png[pos + 3]]) as usize;
        let chunk_type = &png[pos + 4..pos + 8];
        if chunk_type == b"IEND" {
            break;
        }
        if chunk_type == b"tEXt" || chunk_type == b"iTXt" {
            let data_start = pos + 8;
            let data_end = (data_start + chunk_len).min(png.len());
            let data = &png[data_start..data_end];
            let Some(null_pos) = data.iter().position(|&b| b == 0) else {
                continue;
            };
            if &data[..null_pos] == keyword {
                count += 1;
            }
        }
        let Some(chunk_end) = pos.checked_add(12).and_then(|p| p.checked_add(chunk_len)) else {
            break;
        };
        if chunk_end > png.len() {
            break;
        }
        pos = chunk_end;
    }
    count
}

fn metadata_only_request() -> ProtectionRequest {
    ProtectionRequest::metadata_only(
        RightsNotice::default(),
        RightsPolicy::ProhibitedAiMlTraining,
    )
    .with_legal_metadata(
        LegalMetadata::new()
            .with_copyright_holder("Holder")
            .with_usage_terms("Terms")
            .with_creator("Creator")
            .with_credit_line("Credit")
            .with_copyright_owner("Owner")
            .with_ai_constraints("No AI"),
    )
    .with_output_format(ImageOutputFormat::Png)
    .with_processing(ProcessingOptions {
        metadata_update_policy: MetadataUpdatePolicy::ReplaceStegoOwned,
        ..ProcessingOptions::default()
    })
}

#[test]
fn png_replace_stego_owned_idempotent_on_metadata_only_path() {
    let base = make_test_image_png(32, 32);
    let request = metadata_only_request();

    let out1 = process_request_bytes(&base, &request).unwrap();
    let out2 = process_request_bytes(&out1, &request).unwrap();

    assert_eq!(
        count_png_text_chunks(&out1, b"Copyright"),
        1,
        "first round should write exactly one Copyright tEXt"
    );
    assert_eq!(
        count_png_text_chunks(&out2, b"Copyright"),
        1,
        "second round must not duplicate Copyright (BUG-01)"
    );
    assert_eq!(
        count_png_text_chunks(&out2, b"Creator"),
        1,
        "second round must not duplicate Creator (BUG-01)"
    );
    assert_eq!(
        count_png_text_chunks(&out2, b"UsageTerms"),
        1,
        "second round must not duplicate UsageTerms (BUG-01)"
    );

    let xmp_count = count_png_text_chunks(&out2, b"XML:com.adobe.xmp");
    assert!(
        xmp_count <= 1,
        "second round must not duplicate XMP iTXt (BUG-02), got {}",
        xmp_count
    );
}

#[test]
fn png_preserve_existing_does_not_duplicate_legal_keys() {
    let base = make_test_image_png(32, 32);
    let seed_request = metadata_only_request();

    let first = process_request_bytes(&base, &seed_request).unwrap();

    let preserved_request =
        ProtectionRequest::metadata_only(RightsNotice::default(), RightsPolicy::Allowed)
            .with_legal_metadata(
                LegalMetadata::new()
                    .with_copyright_holder("Different Holder")
                    .with_creator("Different Creator"),
            )
            .with_output_format(ImageOutputFormat::Png)
            .with_processing(ProcessingOptions {
                metadata_update_policy: MetadataUpdatePolicy::PreserveExisting,
                ..ProcessingOptions::default()
            });

    let second = process_request_bytes(&first, &preserved_request).unwrap();

    assert_eq!(
        count_png_text_chunks(&second, b"Copyright"),
        1,
        "PreserveExisting must keep a single Copyright (BUG-01)"
    );
    assert_eq!(
        count_png_text_chunks(&second, b"Creator"),
        1,
        "PreserveExisting must keep a single Creator (BUG-01)"
    );
}

#[test]
fn png_xmp_only_input_triggers_fail_on_conflict() {
    let mut png = make_test_image_png(32, 32);
    let xmp = b"<?xpacket begin=\"\xef\xbb\xbf\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?><x:xmpmeta xmlns:x=\"adobe:ns:meta/\"><rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\"><rdf:Description xmlns:stegoeggo=\"https://stegoeggo.dev/ns/\" stegoeggo:ProtectionSeed=\"42\" plus:DataMining xmlns:plus=\"http://ns.useplus.org/ldf/xmp/1.0/\" plus:dmAllowed=\"false\" /></rdf:RDF></x:xmpmeta><?xpacket end=\"w\"?>";
    let mut chunk_data = Vec::new();
    chunk_data.extend_from_slice(b"XML:com.adobe.xmp");
    chunk_data.push(0);
    chunk_data.push(0);
    chunk_data.push(0);
    chunk_data.push(0);
    chunk_data.push(0);
    chunk_data.extend_from_slice(xmp);
    let len = u32::try_from(chunk_data.len()).unwrap();
    let mut chunk = Vec::new();
    chunk.extend_from_slice(&len.to_be_bytes());
    chunk.extend_from_slice(b"iTXt");
    chunk.extend_from_slice(&chunk_data);
    let crc = {
        let mut h = crc32fast::Hasher::new();
        h.update(b"iTXt");
        h.update(&chunk_data);
        h.finalize()
    };
    chunk.extend_from_slice(&crc.to_be_bytes());

    let iend_pos = png.len() - 12;
    png.splice(iend_pos..iend_pos, chunk.iter().cloned());

    let request = ProtectionRequest::metadata_only(RightsNotice::default(), RightsPolicy::Allowed)
        .with_legal_metadata(LegalMetadata::new().with_copyright_holder("Test"))
        .with_output_format(ImageOutputFormat::Png)
        .with_processing(ProcessingOptions {
            metadata_update_policy: MetadataUpdatePolicy::FailOnConflict,
            ..ProcessingOptions::default()
        });

    let result = process_request_bytes(&png, &request);
    assert!(
        result.is_err(),
        "FailOnConflict must reject PNG with only an XMP iTXt (BUG-02)"
    );
}
