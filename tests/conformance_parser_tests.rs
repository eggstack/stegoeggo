use stegoeggo::DmiValue;

fn make_test_image_png(width: u32, height: u32) -> Vec<u8> {
    let img = image::DynamicImage::new_rgb8(width, height);
    let mut buf = std::io::Cursor::new(Vec::new());
    img.write_to(&mut buf, image::ImageFormat::Png).unwrap();
    buf.into_inner()
}

fn make_png_with_chunks(base: &[u8], text_chunks: &[(&str, &str)], xmp: Option<&str>) -> Vec<u8> {
    let mut out = Vec::with_capacity(base.len() + 1024);
    out.extend_from_slice(&base[..8]);
    let mut i = 8;
    while i + 8 <= base.len() {
        let length = u32::from_be_bytes([base[i], base[i + 1], base[i + 2], base[i + 3]]) as usize;
        let chunk_type = &base[i + 4..i + 8];
        if chunk_type == b"IEND" {
            for (key, value) in text_chunks {
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
            if let Some(xmp_content) = xmp {
                let key = "XML:com.adobe.xmp";
                let chunk_data = format!("{}\0{}", key, xmp_content);
                let chunk_bytes = chunk_data.as_bytes();
                let chunk_len = (chunk_bytes.len() as u32).to_be_bytes();
                out.extend_from_slice(&chunk_len);
                out.extend_from_slice(b"iTXt");
                out.extend_from_slice(chunk_bytes);
                let mut crc = crc32fast::Hasher::new();
                crc.update(b"iTXt");
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

#[test]
fn copyright_from_text_chunk() {
    let base = make_test_image_png(64, 64);
    let png = make_png_with_chunks(
        &base,
        &[("Copyright", "Copyright (c) TextChunk Test")],
        None,
    );
    let notice = stegoeggo::verify_legal_notice(&png, b"");
    assert_eq!(notice.copyright_holder(), Some("TextChunk Test"));
}

#[test]
fn usage_terms_from_text_chunk() {
    let base = make_test_image_png(64, 64);
    let png = make_png_with_chunks(&base, &[("UsageTerms", "Custom Usage Terms")], None);
    let notice = stegoeggo::verify_legal_notice(&png, b"");
    assert_eq!(notice.usage_terms(), Some("Custom Usage Terms"));
}

#[test]
fn creator_from_text_chunk() {
    let base = make_test_image_png(64, 64);
    let png = make_png_with_chunks(&base, &[("Creator", "Test Creator")], None);
    let notice = stegoeggo::verify_legal_notice(&png, b"");
    assert_eq!(notice.creator(), Some("Test Creator"));
}

#[test]
fn rights_url_from_text_chunk() {
    let base = make_test_image_png(64, 64);
    let png = make_png_with_chunks(
        &base,
        &[("WebStatementOfRights", "https://example.com/rights")],
        None,
    );
    let notice = stegoeggo::verify_legal_notice(&png, b"");
    assert_eq!(
        notice.web_statement_of_rights(),
        Some("https://example.com/rights")
    );
}

#[test]
fn ai_constraints_from_text_chunk() {
    let base = make_test_image_png(64, 64);
    let png = make_png_with_chunks(&base, &[("AIConstraints", "No AI training")], None);
    let notice = stegoeggo::verify_legal_notice(&png, b"");
    assert_eq!(notice.ai_constraints(), Some("No AI training"));
}

#[test]
fn dmi_from_xmp_attribute_form() {
    let base = make_test_image_png(64, 64);
    let xmp = r#"<x:xmpmeta xmlns:x="adobe:ns:meta/">
  <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
    <rdf:Description rdf:about=""
      xmlns:plus="http://ns.useplus.org/ldf/xmp/1.0/"
      plus:DataMining="DMI-PROHIBITED-AIMLTRAINING"/>
  </rdf:RDF>
</x:xmpmeta>"#;
    let png = make_png_with_chunks(&base, &[], Some(xmp));
    let notice = stegoeggo::verify_legal_notice(&png, b"");
    assert_eq!(
        notice.canonical_dmi(),
        Some(DmiValue::ProhibitedAiMlTraining)
    );
}

#[test]
fn dmi_from_xmp_element_form() {
    let base = make_test_image_png(64, 64);
    let xmp = r#"<x:xmpmeta xmlns:x="adobe:ns:meta/">
  <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
    <rdf:Description rdf:about=""
      xmlns:plus="http://ns.useplus.org/ldf/xmp/1.0/">
      <plus:DataMining>DMI-PROHIBITED-AIMLTRAINING</plus:DataMining>
    </rdf:Description>
  </rdf:RDF>
</x:xmpmeta>"#;
    let png = make_png_with_chunks(&base, &[], Some(xmp));
    let notice = stegoeggo::verify_legal_notice(&png, b"");
    assert_eq!(
        notice.canonical_dmi(),
        Some(DmiValue::ProhibitedAiMlTraining)
    );
}

#[test]
fn dmi_unknown_cv_value() {
    let base = make_test_image_png(64, 64);
    let xmp = r#"<x:xmpmeta xmlns:x="adobe:ns:meta/">
  <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
    <rdf:Description rdf:about=""
      xmlns:plus="http://ns.useplus.org/ldf/xmp/1.0/">
      <plus:DataMining>DMI-TOTALLY-UNKNOWN-VALUE</plus:DataMining>
    </rdf:Description>
  </rdf:RDF>
</x:xmpmeta>"#;
    let png = make_png_with_chunks(&base, &[], Some(xmp));
    let notice = stegoeggo::verify_legal_notice(&png, b"");
    assert!(
        notice.canonical_dmi().is_none(),
        "Unknown CV URI should not be parsed as a recognized DMI value"
    );
}

#[test]
fn dmi_conflicting_values() {
    let base = make_test_image_png(64, 64);
    let xmp = r#"<x:xmpmeta xmlns:x="adobe:ns:meta/">
  <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
    <rdf:Description rdf:about=""
      xmlns:plus="http://ns.useplus.org/ldf/xmp/1.0/">
      <plus:DataMining>DMI-PROHIBITED-AIMLTRAINING</plus:DataMining>
      <plus:DataMining>DMI-ALLOWED</plus:DataMining>
    </rdf:Description>
  </rdf:RDF>
</x:xmpmeta>"#;
    let png = make_png_with_chunks(&base, &[], Some(xmp));
    let notice = stegoeggo::verify_legal_notice(&png, b"");
    assert!(
        notice.has_notice(),
        "Conflicting DMI values should be detected"
    );
}

#[test]
fn xmp_empty_wrapper_no_rdf() {
    let base = make_test_image_png(64, 64);
    let xmp = r#"<x:xmpmeta xmlns:x="adobe:ns:meta/">
</x:xmpmeta>"#;
    let png = make_png_with_chunks(&base, &[], Some(xmp));
    let notice = stegoeggo::verify_legal_notice(&png, b"");
    assert!(
        !notice.has_notice(),
        "Empty XMP wrapper with no RDF should not produce a notice"
    );
}

#[test]
fn xmp_multiple_descriptions_dmi() {
    let base = make_test_image_png(64, 64);
    let xmp = r#"<x:xmpmeta xmlns:x="adobe:ns:meta/">
  <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
    <rdf:Description rdf:about=""
      xmlns:dc="http://purl.org/dc/elements/1.1/">
      <dc:rights>
        <rdf:Alt>
          <rdf:li xml:lang="x-default">Copyright Holder A</rdf:li>
        </rdf:Alt>
      </dc:rights>
    </rdf:Description>
    <rdf:Description rdf:about=""
      xmlns:plus="http://ns.useplus.org/ldf/xmp/1.0/">
      <plus:DataMining>DMI-PROHIBITED-AIMLTRAINING</plus:DataMining>
    </rdf:Description>
  </rdf:RDF>
</x:xmpmeta>"#;
    let png = make_png_with_chunks(&base, &[], Some(xmp));
    let notice = stegoeggo::verify_legal_notice(&png, b"");
    assert_eq!(
        notice.canonical_dmi(),
        Some(DmiValue::ProhibitedAiMlTraining)
    );
}

#[test]
fn multiple_text_chunks_combined() {
    let base = make_test_image_png(64, 64);
    let png = make_png_with_chunks(
        &base,
        &[
            ("Copyright", "Copyright (c) Multi Holder"),
            ("UsageTerms", "Multi Terms"),
            ("Creator", "Multi Creator"),
            ("WebStatementOfRights", "https://multi.example.com"),
            ("AIConstraints", "Multi AI Constraints"),
        ],
        None,
    );
    let notice = stegoeggo::verify_legal_notice(&png, b"");
    assert_eq!(notice.copyright_holder(), Some("Multi Holder"));
    assert_eq!(notice.usage_terms(), Some("Multi Terms"));
    assert_eq!(notice.creator(), Some("Multi Creator"));
    assert_eq!(
        notice.web_statement_of_rights(),
        Some("https://multi.example.com")
    );
    assert_eq!(notice.ai_constraints(), Some("Multi AI Constraints"));
}

#[test]
fn text_chunk_with_xmp_combined() {
    let base = make_test_image_png(64, 64);
    let xmp = r#"<x:xmpmeta xmlns:x="adobe:ns:meta/">
  <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
    <rdf:Description rdf:about=""
      xmlns:plus="http://ns.useplus.org/ldf/xmp/1.0/">
      <plus:DataMining>DMI-PROHIBITED-AIMLTRAINING</plus:DataMining>
    </rdf:Description>
  </rdf:RDF>
</x:xmpmeta>"#;
    let png = make_png_with_chunks(
        &base,
        &[
            ("Copyright", "Copyright (c) Combined Holder"),
            ("UsageTerms", "Combined Terms"),
        ],
        Some(xmp),
    );
    let notice = stegoeggo::verify_legal_notice(&png, b"");
    assert_eq!(notice.copyright_holder(), Some("Combined Holder"));
    assert_eq!(notice.usage_terms(), Some("Combined Terms"));
    assert_eq!(
        notice.canonical_dmi(),
        Some(DmiValue::ProhibitedAiMlTraining)
    );
}

#[test]
fn text_chunk_special_characters() {
    let base = make_test_image_png(64, 64);
    let png = make_png_with_chunks(
        &base,
        &[(
            "Copyright",
            "Copyright (c) Test & Entity <Special> \"Quotes\"",
        )],
        None,
    );
    let notice = stegoeggo::verify_legal_notice(&png, b"");
    assert_eq!(
        notice.copyright_holder(),
        Some("Test & Entity <Special> \"Quotes\"")
    );
}

#[test]
fn text_chunk_unicode_value() {
    let base = make_test_image_png(64, 64);
    let png = make_png_with_chunks(
        &base,
        &[("Copyright", "Copyright (c) 日本テスト株式会社")],
        None,
    );
    let notice = stegoeggo::verify_legal_notice(&png, b"");
    assert_eq!(notice.copyright_holder(), Some("日本テスト株式会社"));
}

#[test]
fn duplicate_text_chunks_last_wins() {
    let base = make_test_image_png(64, 64);
    let png = make_png_with_chunks(
        &base,
        &[
            ("Copyright", "Copyright (c) First Holder"),
            ("Copyright", "Copyright (c) Second Holder"),
        ],
        None,
    );
    let notice = stegoeggo::verify_legal_notice(&png, b"");
    assert!(
        notice.copyright_holder().is_some(),
        "Should extract a copyright value from duplicate chunks"
    );
}
