#![allow(deprecated)]

use stegoeggo::{
    process_image_bytes_with_warnings, DmiValue, ImageOutputFormat, LegalMetadata,
    ProtectionContext, ProtectionLevel,
};

fn make_test_image_png(width: u32, height: u32) -> Vec<u8> {
    let img = image::DynamicImage::new_rgb8(width, height);
    let mut buf = std::io::Cursor::new(Vec::new());
    img.write_to(&mut buf, image::ImageFormat::Png).unwrap();
    buf.into_inner()
}

fn fixtures_dir() -> std::path::PathBuf {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    std::path::PathBuf::from(manifest_dir).join("tests/fixtures/conformance")
}

fn ensure_dir(dir: &std::path::Path) {
    std::fs::create_dir_all(dir).unwrap();
}

fn process_write(
    input: &[u8],
    format: ImageOutputFormat,
    legal: &LegalMetadata,
    dmi: DmiValue,
    path: &std::path::Path,
    seed: u64,
) {
    let ctx = ProtectionContext::new(0.5, seed)
        .with_format(format)
        .with_legal_metadata(legal.clone())
        .with_dmi(dmi);
    let (output, _warnings) =
        process_image_bytes_with_warnings(input, ProtectionLevel::Standard, &ctx).unwrap();
    std::fs::write(path, &output).unwrap();
}

fn make_png_with_text_chunks(base: &[u8], chunks: &[(&str, &str)]) -> Vec<u8> {
    let mut out = Vec::with_capacity(base.len() + chunks.len() * 200);
    out.extend_from_slice(&base[..8]);
    let mut i = 8;
    while i + 8 <= base.len() {
        let length = u32::from_be_bytes([base[i], base[i + 1], base[i + 2], base[i + 3]]) as usize;
        let chunk_type = &base[i + 4..i + 8];
        if chunk_type == b"IEND" {
            for (key, value) in chunks {
                let chunk_data = format!("{}\0{}", key, value);
                let chunk_bytes = chunk_data.as_bytes();
                let chunk_len = (chunk_bytes.len() as u32).to_be_bytes();
                out.extend_from_slice(&chunk_len);
                out.extend_from_slice(b"tEXt");
                out.extend_from_slice(chunk_bytes);
                let mut crc = crc32fast::Hasher::new();
                crc.update(b"tEXt");
                crc.update(chunk_bytes);
                out.extend_from_slice(&crc.finalize().to_be_bytes());
            }
            out.extend_from_slice(&base[i..i + 8 + length + 4]);
            break;
        } else {
            out.extend_from_slice(&base[i..i + 8 + length + 4]);
        }
        i += 8 + length + 4;
    }
    out
}

fn make_png_with_itx(base: &[u8], xmp_value: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(base.len() + xmp_value.len() + 100);
    out.extend_from_slice(&base[..8]);
    let mut i = 8;
    while i + 8 <= base.len() {
        let length = u32::from_be_bytes([base[i], base[i + 1], base[i + 2], base[i + 3]]) as usize;
        let chunk_type = &base[i + 4..i + 8];
        if chunk_type == b"IEND" {
            let key = "XML:com.adobe.xmp";
            let chunk_data = format!("{}\0{}", key, xmp_value);
            let chunk_bytes = chunk_data.as_bytes();
            let chunk_len = (chunk_bytes.len() as u32).to_be_bytes();
            out.extend_from_slice(&chunk_len);
            out.extend_from_slice(b"iTXt");
            out.extend_from_slice(chunk_bytes);
            let mut crc = crc32fast::Hasher::new();
            crc.update(b"iTXt");
            crc.update(chunk_bytes);
            out.extend_from_slice(&crc.finalize().to_be_bytes());
            out.extend_from_slice(&base[i..i + 8 + length + 4]);
            break;
        } else {
            out.extend_from_slice(&base[i..i + 8 + length + 4]);
        }
        i += 8 + length + 4;
    }
    out
}

fn make_jpeg_with_app1_xmp(xmp: &str) -> Vec<u8> {
    let img = image::DynamicImage::new_rgb8(64, 64);
    let mut buf = std::io::Cursor::new(Vec::new());
    img.write_to(&mut buf, image::ImageFormat::Jpeg).unwrap();
    let base = buf.into_inner();
    let prefix = b"http://ns.adobe.com/xap/1.0/\0";
    let xmp_bytes = xmp.as_bytes();
    let payload_len = prefix.len() + xmp_bytes.len();
    let marker_len = (payload_len + 2) as u16;
    let mut out = Vec::with_capacity(base.len() + 4 + payload_len);
    out.extend_from_slice(&base[..2]);
    out.extend_from_slice(&[0xFF, 0xE1]);
    out.extend_from_slice(&marker_len.to_be_bytes());
    out.extend_from_slice(prefix);
    out.extend_from_slice(xmp_bytes);
    out.extend_from_slice(&base[2..]);
    out
}

fn make_webp_with_xmp(xmp: &str) -> Vec<u8> {
    let img = image::DynamicImage::new_rgb8(64, 64);
    let mut buf = std::io::Cursor::new(Vec::new());
    img.write_to(&mut buf, image::ImageFormat::WebP).unwrap();
    let base = buf.into_inner();
    let xmp_with_header = format!(
        "<?xpacket begin=\"\u{feff}\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\n{}\n<?xpacket end=\"w\"?>",
        xmp
    );
    let xmp_bytes = xmp_with_header.as_bytes();
    let xmp_chunk_data_len = xmp_bytes.len() as u32;
    let xmp_chunk_total = 8 + xmp_chunk_data_len + (xmp_chunk_data_len & 1);
    let original_riff_size = u32::from_le_bytes([base[4], base[5], base[6], base[7]]);
    let new_riff_size = original_riff_size + xmp_chunk_total;
    let mut out = Vec::with_capacity(base.len() + xmp_chunk_total as usize);
    out.extend_from_slice(&base[..4]);
    out.extend_from_slice(&new_riff_size.to_le_bytes());
    out.extend_from_slice(&base[8..]);
    out.extend_from_slice(b"XMP ");
    out.extend_from_slice(&xmp_chunk_data_len.to_le_bytes());
    out.extend_from_slice(xmp_bytes);
    if xmp_chunk_data_len & 1 == 1 {
        out.push(0);
    }
    out
}

#[test]
fn generate_canonical_fixtures() {
    let base = fixtures_dir().join("canonical");
    ensure_dir(&base);

    let png_bytes = make_test_image_png(64, 64);

    let legal_complete = LegalMetadata::new()
        .with_copyright_holder("Conformance Test Holder")
        .with_creator("Conformance Creator")
        .with_copyright_owner("Conformance Owner")
        .with_contact_email("contact@conformance.test")
        .with_license_url("https://conformance.test/license")
        .with_usage_terms("All rights reserved for conformance testing")
        .with_web_statement_of_rights("https://conformance.test/rights")
        .with_creation_date("2025-01-15")
        .with_ai_constraints("No AI training permitted")
        .with_credit_line("Photo by Conformance Creator")
        .with_licensor_name("Conformance Licensor")
        .with_licensor_email("licensor@conformance.test")
        .with_licensor_url("https://licensor.conformance.test")
        .with_notice_applied_at("2025-01-15T00:00:00Z");

    process_write(
        &png_bytes,
        ImageOutputFormat::Png,
        &legal_complete,
        DmiValue::ProhibitedAiMlTraining,
        &base.join("canonical_complete.png"),
        42,
    );
    process_write(
        &png_bytes,
        ImageOutputFormat::Jpeg,
        &legal_complete,
        DmiValue::ProhibitedAiMlTraining,
        &base.join("canonical_complete.jpg"),
        42,
    );
    process_write(
        &png_bytes,
        ImageOutputFormat::WebP,
        &legal_complete,
        DmiValue::ProhibitedAiMlTraining,
        &base.join("canonical_complete.webp"),
        42,
    );

    let legal_policy_only = LegalMetadata::new().with_notice_applied_at("2025-01-15T00:00:00Z");
    process_write(
        &png_bytes,
        ImageOutputFormat::Png,
        &legal_policy_only,
        DmiValue::ProhibitedAiMlTraining,
        &base.join("canonical_policy_only.png"),
        43,
    );
    process_write(
        &png_bytes,
        ImageOutputFormat::Jpeg,
        &legal_policy_only,
        DmiValue::ProhibitedAiMlTraining,
        &base.join("canonical_policy_only.jpg"),
        43,
    );
    process_write(
        &png_bytes,
        ImageOutputFormat::WebP,
        &legal_policy_only,
        DmiValue::ProhibitedAiMlTraining,
        &base.join("canonical_policy_only.webp"),
        43,
    );

    let legal_copyright = LegalMetadata::new()
        .with_copyright_holder("Jane Doe")
        .with_notice_applied_at("2025-01-15T00:00:00Z");
    process_write(
        &png_bytes,
        ImageOutputFormat::Png,
        &legal_copyright,
        DmiValue::ProhibitedAiMlTraining,
        &base.join("canonical_copyright_only.png"),
        44,
    );

    let legal_unicode = LegalMetadata::new()
        .with_copyright_holder("日本テスト株式会社")
        .with_usage_terms("Alle Rechte vorbehalten. 版權所有。")
        .with_notice_applied_at("2025-01-15T00:00:00Z");
    process_write(
        &png_bytes,
        ImageOutputFormat::Png,
        &legal_unicode,
        DmiValue::ProhibitedAiMlTraining,
        &base.join("canonical_unicode.png"),
        45,
    );
    process_write(
        &png_bytes,
        ImageOutputFormat::WebP,
        &legal_unicode,
        DmiValue::ProhibitedAiMlTraining,
        &base.join("canonical_unicode.webp"),
        45,
    );

    let xmp_alt_prefix = r#"<x:xmpmeta xmlns:x="adobe:ns:meta/">
  <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
    <rdf:Description rdf:about=""
      xmlns:PLUS="http://ns.useplus.org/ldf/xmp/1.0/"
      xmlns:dc="http://purl.org/dc/elements/1.1/"
      dc:rights="Copyright (c) 2025 Alt Prefix Test"
      PLUS:DataMining="http://ns.useplus.org/ldf/value/DMI-PROHIBITED-AIMLTRAINING">
    </rdf:Description>
  </rdf:RDF>
</x:xmpmeta>"#;
    let alt_prefix_png = make_png_with_itx(&make_test_image_png(64, 64), xmp_alt_prefix);
    std::fs::write(base.join("canonical_alt_prefix.png"), &alt_prefix_png).unwrap();
    let alt_prefix_jpeg = make_jpeg_with_app1_xmp(xmp_alt_prefix);
    std::fs::write(base.join("canonical_alt_prefix.jpg"), &alt_prefix_jpeg).unwrap();

    let xmp_multi_creator = r#"<x:xmpmeta xmlns:x="adobe:ns:meta/">
  <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
    <rdf:Description rdf:about=""
      xmlns:dc="http://purl.org/dc/elements/1.1/"
      xmlns:plus="http://ns.useplus.org/ldf/xmp/1.0/"
      xmlns:xmpRights="http://ns.adobe.com/xap/1.0/rights/"
      xmlns:Iptc4xmpExt="http://iptc.org/std/Iptc4xmpExt/2008-02/29/"
      dc:rights="Copyright (c) 2025 Multi Creator Corp. All rights reserved."
      dc:creator="{'Alice Smith','Bob Jones','Carol Davis'}"
      xmpRights:WebStatement="https://example.com/rights"
      plus:DataMining="http://ns.useplus.org/ldf/value/DMI-PROHIBITED-AIMLTRAINING"
      Iptc4xmpExt:CopyrightOwner="Multi Creator Corp"
      Iptc4xmpExt:LicensorName="License Admin"
      Iptc4xmpExt:LicensorEmail="admin@multi-creator.test"
      Iptc4xmpExt:LicensorUrl="https://multi-creator.test/license">
    </rdf:Description>
  </rdf:RDF>
</x:xmpmeta>"#;
    let multi_creator_png = make_png_with_itx(&make_test_image_png(64, 64), xmp_multi_creator);
    std::fs::write(base.join("canonical_multi_creator.png"), &multi_creator_png).unwrap();
    let multi_creator_jpeg = make_jpeg_with_app1_xmp(xmp_multi_creator);
    std::fs::write(
        base.join("canonical_multi_creator.jpg"),
        &multi_creator_jpeg,
    )
    .unwrap();
    let multi_creator_webp = make_webp_with_xmp(xmp_multi_creator);
    std::fs::write(
        base.join("canonical_multi_creator.webp"),
        &multi_creator_webp,
    )
    .unwrap();
}

#[test]
fn generate_canonical_independent_fixtures() {
    let base = fixtures_dir().join("canonical");
    ensure_dir(&base);

    let xmp_doc = r#"<x:xmpmeta xmlns:x="adobe:ns:meta/">
  <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
    <rdf:Description rdf:about=""
      xmlns:dc="http://purl.org/dc/elements/1.1/"
      xmlns:plus="http://ns.useplus.org/ldf/xmp/1.0/"
      xmlns:xmpRights="http://ns.adobe.com/xap/1.0/rights/"
      dc:rights="Copyright (c) 2025 Independent Author. All rights reserved."
      xmpRights:WebStatement="https://example.com/rights"
      plus:DataMining="http://ns.useplus.org/ldf/value/DMI-PROHIBITED-AIMLTRAINING">
    </rdf:Description>
  </rdf:RDF>
</x:xmpmeta>"#;
    let png_independent = make_png_with_itx(&make_test_image_png(64, 64), xmp_doc);
    std::fs::write(base.join("canonical_independent.png"), &png_independent).unwrap();

    let jpeg_independent = make_jpeg_with_app1_xmp(xmp_doc);
    std::fs::write(base.join("canonical_independent.jpg"), &jpeg_independent).unwrap();

    let webp_independent = make_webp_with_xmp(xmp_doc);
    std::fs::write(base.join("canonical_independent.webp"), &webp_independent).unwrap();
}

#[test]
fn generate_legacy_fixtures() {
    let base = fixtures_dir().join("legacy");
    ensure_dir(&base);

    let png_bytes = make_test_image_png(64, 64);

    let legal = LegalMetadata::new()
        .with_copyright_holder("Legacy Holder")
        .with_usage_terms("Legacy Terms")
        .with_notice_applied_at("2025-01-15T00:00:00Z");
    process_write(
        &png_bytes,
        ImageOutputFormat::Png,
        &legal,
        DmiValue::ProhibitedAiMlTraining,
        &base.join("legacy_v02_dmi_prohibited.png"),
        50,
    );
    process_write(
        &png_bytes,
        ImageOutputFormat::Jpeg,
        &legal,
        DmiValue::ProhibitedAiMlTraining,
        &base.join("legacy_v02_dmi_prohibited.jpg"),
        50,
    );
    process_write(
        &png_bytes,
        ImageOutputFormat::WebP,
        &legal,
        DmiValue::ProhibitedAiMlTraining,
        &base.join("legacy_v02_dmi_prohibited.webp"),
        50,
    );
}

#[test]
fn generate_malformed_fixtures() {
    let base = fixtures_dir().join("malformed");
    ensure_dir(&base);

    let mut truncated_png = make_test_image_png(64, 64);
    truncated_png.truncate(64);
    std::fs::write(base.join("truncated.png"), &truncated_png).unwrap();

    let img = image::DynamicImage::new_rgb8(64, 64);
    let mut buf = std::io::Cursor::new(Vec::new());
    img.write_to(&mut buf, image::ImageFormat::Jpeg).unwrap();
    let mut truncated_jpeg = buf.into_inner();
    truncated_jpeg.truncate(64);
    std::fs::write(base.join("truncated.jpg"), &truncated_jpeg).unwrap();

    let xmp_truncated = r#"<x:xmpmeta xmlns:x="adobe:ns:meta/">
  <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
    <rdf:Description rdf:about=""
      xmlns:dc="http://purl.org/dc/elements/1.1/"
      dc:rights="Copyright (c) 2025"#;
    let truncated_xmp_png = make_png_with_itx(&make_test_image_png(64, 64), xmp_truncated);
    std::fs::write(base.join("truncated_xmp.png"), &truncated_xmp_png).unwrap();

    let xmp_invalid_entity = r#"<x:xmpmeta xmlns:x="adobe:ns:meta/">
  <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
    <rdf:Description rdf:about=""
      xmlns:dc="http://purl.org/dc/elements/1.1/"
      dc:rights="Copyright &invalid_entity; 2025. All rights reserved.">
    </rdf:Description>
  </rdf:RDF>
</x:xmpmeta>"#;
    let invalid_entity_png = make_png_with_itx(&make_test_image_png(64, 64), xmp_invalid_entity);
    std::fs::write(base.join("invalid_xml_entity.png"), &invalid_entity_png).unwrap();

    let xmp_invalid_ns = r#"<x:xmpmeta xmlns:x="adobe:ns:meta/">
  <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
    <rdf:Description rdf:about=""
      xmlns:unknown="http://example.com/unknown-namespace/"
      unknown:someProperty="test value">
    </rdf:Description>
  </rdf:RDF>
</x:xmpmeta>"#;
    let invalid_ns_png = make_png_with_itx(&make_test_image_png(64, 64), xmp_invalid_ns);
    std::fs::write(base.join("invalid_namespace.png"), &invalid_ns_png).unwrap();

    let xmp_unknown_cv = r#"<x:xmpmeta xmlns:x="adobe:ns:meta/">
  <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
    <rdf:Description rdf:about=""
      xmlns:plus="http://ns.useplus.org/ldf/xmp/1.0/"
      plus:DataMining="http://ns.useplus.org/ldf/value/DMI-UNKNOWN-VALUE">
    </rdf:Description>
  </rdf:RDF>
</x:xmpmeta>"#;
    let unknown_cv_png = make_png_with_itx(&make_test_image_png(64, 64), xmp_unknown_cv);
    std::fs::write(base.join("unknown_cv_uri.png"), &unknown_cv_png).unwrap();

    let oversized_value = "X".repeat(8000);
    let xmp_oversized = format!(
        r#"<x:xmpmeta xmlns:x="adobe:ns:meta/">
  <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
    <rdf:Description rdf:about=""
      xmlns:dc="http://purl.org/dc/elements/1.1/"
      dc:rights="{}">
    </rdf:Description>
  </rdf:RDF>
</x:xmpmeta>"#,
        oversized_value
    );
    let oversized_png = make_png_with_itx(&make_test_image_png(64, 64), &xmp_oversized);
    std::fs::write(base.join("oversized_metadata.png"), &oversized_png).unwrap();

    let img = image::DynamicImage::new_rgb8(64, 64);
    let mut buf = std::io::Cursor::new(Vec::new());
    img.write_to(&mut buf, image::ImageFormat::Jpeg).unwrap();
    let mut bad_jpeg = buf.into_inner();
    if bad_jpeg.len() > 10 {
        bad_jpeg[4] = 0xFF;
        bad_jpeg[5] = 0x00;
    }
    std::fs::write(base.join("invalid_marker_length.jpg"), &bad_jpeg).unwrap();

    let img = image::DynamicImage::new_rgb8(64, 64);
    let mut buf = std::io::Cursor::new(Vec::new());
    img.write_to(&mut buf, image::ImageFormat::WebP).unwrap();
    let mut bad_webp = buf.into_inner();
    if bad_webp.len() > 16 {
        bad_webp[12] = 0xFF;
        bad_webp[13] = 0xFF;
        bad_webp[14] = 0xFF;
        bad_webp[15] = 0xFF;
    }
    std::fs::write(base.join("invalid_riff_length.webp"), &bad_webp).unwrap();
}

#[test]
fn generate_conflicting_fixtures() {
    let base = fixtures_dir().join("conflicting");
    ensure_dir(&base);

    let png_bytes = make_test_image_png(64, 64);

    let legal_allowed = LegalMetadata::new()
        .with_copyright_holder("Allowed Holder")
        .with_notice_applied_at("2025-01-15T00:00:00Z");
    let ctx_allowed = ProtectionContext::new(0.5, 60)
        .with_format(ImageOutputFormat::Png)
        .with_legal_metadata(legal_allowed)
        .with_dmi(DmiValue::Allowed);
    let (allowed_png, _) =
        process_image_bytes_with_warnings(&png_bytes, ProtectionLevel::Standard, &ctx_allowed)
            .unwrap();

    let legal_prohibited = LegalMetadata::new()
        .with_copyright_holder("Prohibited Holder")
        .with_notice_applied_at("2025-01-15T00:00:00Z");
    let ctx_prohibited = ProtectionContext::new(0.5, 61)
        .with_format(ImageOutputFormat::Png)
        .with_legal_metadata(legal_prohibited)
        .with_dmi(DmiValue::ProhibitedAiMlTraining);
    let (prohibited_png, _) =
        process_image_bytes_with_warnings(&png_bytes, ProtectionLevel::Standard, &ctx_prohibited)
            .unwrap();

    std::fs::write(base.join("dmi_allowed_then_prohibited.png"), &allowed_png).unwrap();
    std::fs::write(
        base.join("dmi_prohibited_then_allowed.png"),
        &prohibited_png,
    )
    .unwrap();

    let base_png = make_test_image_png(64, 64);
    let multi_copyright = make_png_with_text_chunks(
        &base_png,
        &[("Copyright", "Holder A"), ("Copyright", "Holder B")],
    );
    std::fs::write(
        base.join("conflicting_copyright_owners.png"),
        &multi_copyright,
    )
    .unwrap();

    let multi_rights = make_png_with_text_chunks(
        &base_png,
        &[
            ("WebStatement", "https://example.com/rights-a"),
            ("WebStatement", "https://example.com/rights-b"),
        ],
    );
    std::fs::write(base.join("conflicting_rights_urls.png"), &multi_rights).unwrap();

    let xmp_tdm_conflict = r#"<x:xmpmeta xmlns:x="adobe:ns:meta/">
  <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
    <rdf:Description rdf:about=""
      xmlns:plus="http://ns.useplus.org/ldf/xmp/1.0/"
      xmlns:tdm="http://www.w3.org/2001/XMLSchema-instance"
      plus:DataMining="http://ns.useplus.org/ldf/value/DMI-PROHIBITED-AIMLTRAINING"
      tdm:reserve_tdm="http://www.tdm-alliance.org/tdm-reservation/1.0/reserved">
    </rdf:Description>
  </rdf:RDF>
</x:xmpmeta>"#;
    let tdm_conflict_png = make_png_with_itx(&make_test_image_png(64, 64), xmp_tdm_conflict);
    std::fs::write(base.join("tdm_conflicts_dmi.png"), &tdm_conflict_png).unwrap();
}

#[test]
fn generate_preservation_fixtures() {
    let base = fixtures_dir().join("preservation");
    ensure_dir(&base);

    let png_bytes = make_test_image_png(64, 64);

    let legal = LegalMetadata::new()
        .with_copyright_holder("Preservation Holder")
        .with_usage_terms("Preservation Terms")
        .with_notice_applied_at("2025-01-15T00:00:00Z");
    process_write(
        &png_bytes,
        ImageOutputFormat::Png,
        &legal,
        DmiValue::ProhibitedAiMlTraining,
        &base.join("preservation_plain.png"),
        70,
    );
    process_write(
        &png_bytes,
        ImageOutputFormat::Jpeg,
        &legal,
        DmiValue::ProhibitedAiMlTraining,
        &base.join("preservation_plain.jpg"),
        70,
    );
    process_write(
        &png_bytes,
        ImageOutputFormat::WebP,
        &legal,
        DmiValue::ProhibitedAiMlTraining,
        &base.join("preservation_plain.webp"),
        70,
    );

    let base_png = make_test_image_png(64, 64);
    let with_creator = make_png_with_text_chunks(&base_png, &[("Creator", "Original Creator")]);
    let legal_new = LegalMetadata::new()
        .with_copyright_holder("New Holder")
        .with_notice_applied_at("2025-01-15T00:00:00Z");
    let ctx = ProtectionContext::new(0.5, 71)
        .with_format(ImageOutputFormat::Png)
        .with_legal_metadata(legal_new)
        .with_dmi(DmiValue::ProhibitedAiMlTraining);
    let (out, _) =
        process_image_bytes_with_warnings(&with_creator, ProtectionLevel::Standard, &ctx).unwrap();
    std::fs::write(base.join("preservation_with_creator.png"), &out).unwrap();

    let with_author = make_png_with_text_chunks(&base_png, &[("Author", "Alice")]);
    let legal_auth = LegalMetadata::new()
        .with_copyright_holder("Auth Holder")
        .with_usage_terms("Auth Terms")
        .with_notice_applied_at("2025-01-15T00:00:00Z");
    let ctx_auth = ProtectionContext::new(0.5, 72)
        .with_format(ImageOutputFormat::Png)
        .with_legal_metadata(legal_auth)
        .with_dmi(DmiValue::ProhibitedAiMlTraining);
    let (out, _) =
        process_image_bytes_with_warnings(&with_author, ProtectionLevel::Standard, &ctx_auth)
            .unwrap();
    std::fs::write(base.join("preservation_unrelated_text.png"), &out).unwrap();

    let with_xmp_author = make_png_with_itx(
        &base_png,
        r#"<x:xmpmeta xmlns:x="adobe:ns:meta/">
  <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
    <rdf:Description rdf:about=""
      xmlns:dc="http://purl.org/dc/elements/1.1/"
      dc:creator="Original XMP Creator">
    </rdf:Description>
  </rdf:RDF>
</x:xmpmeta>"#,
    );
    let legal_xmp = LegalMetadata::new()
        .with_copyright_holder("XMP Holder")
        .with_notice_applied_at("2025-01-15T00:00:00Z");
    let ctx_xmp = ProtectionContext::new(0.5, 73)
        .with_format(ImageOutputFormat::Png)
        .with_legal_metadata(legal_xmp)
        .with_dmi(DmiValue::ProhibitedAiMlTraining);
    let (out, _) =
        process_image_bytes_with_warnings(&with_xmp_author, ProtectionLevel::Standard, &ctx_xmp)
            .unwrap();
    std::fs::write(base.join("preservation_unrelated_xmp.png"), &out).unwrap();

    let mut idempotent_base = make_test_image_png(64, 64);
    let legal_idem = LegalMetadata::new()
        .with_copyright_holder("Idempotent Holder")
        .with_usage_terms("Idempotent Terms")
        .with_notice_applied_at("2025-01-15T00:00:00Z");
    let ctx_idem = ProtectionContext::new(0.5, 74)
        .with_format(ImageOutputFormat::Png)
        .with_legal_metadata(legal_idem.clone())
        .with_dmi(DmiValue::ProhibitedAiMlTraining);
    let (first, _) =
        process_image_bytes_with_warnings(&idempotent_base, ProtectionLevel::Standard, &ctx_idem)
            .unwrap();
    idempotent_base = first.clone();
    let (second, _) =
        process_image_bytes_with_warnings(&idempotent_base, ProtectionLevel::Standard, &ctx_idem)
            .unwrap();
    std::fs::write(base.join("preservation_idempotent.png"), &second).unwrap();

    let notice1 = stegoeggo::verify_legal_notice(&first, b"");
    let notice2 = stegoeggo::verify_legal_notice(&second, b"");
    assert_eq!(notice1.copyright_holder(), notice2.copyright_holder());
    assert_eq!(notice1.usage_terms(), notice2.usage_terms());
    assert_eq!(notice1.dmi(), notice2.dmi());
}
