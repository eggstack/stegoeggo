use stegoeggo::{verify_legal_notice, DmiValue, RightsSignalKind};

fn make_png_with_xmp(xmp_content: &[u8]) -> Vec<u8> {
    let img = image::DynamicImage::new_rgb8(1, 1);
    let mut png = Vec::new();
    {
        use image::ImageEncoder;
        let encoder = image::codecs::png::PngEncoder::new(&mut png);
        let rgb = img.to_rgb8();
        encoder
            .write_image(&rgb, 1, 1, image::ExtendedColorType::Rgb8)
            .unwrap();
    }

    let keyword = b"XML:com.adobe.xmp";
    let mut itxt_data = Vec::new();
    itxt_data.extend_from_slice(keyword);
    itxt_data.push(0);
    itxt_data.push(0);
    itxt_data.push(0);
    itxt_data.push(0);
    itxt_data.push(0);
    itxt_data.extend_from_slice(xmp_content);

    let itxt_type = b"iTXt";
    let mut itxt_chunk_data = Vec::new();
    itxt_chunk_data.extend_from_slice(itxt_type);
    itxt_chunk_data.extend_from_slice(&itxt_data);
    let itxt_crc = crc32fast::hash(&itxt_chunk_data);

    let iend_pos = png
        .windows(4)
        .position(|w| w == b"IEND")
        .expect("IEND not found");
    let insert_pos = iend_pos - 4;

    let mut result = Vec::new();
    result.extend_from_slice(&png[..insert_pos]);
    result.extend_from_slice(&(itxt_data.len() as u32).to_be_bytes());
    result.extend_from_slice(&itxt_chunk_data);
    result.extend_from_slice(&itxt_crc.to_be_bytes());
    result.extend_from_slice(&png[insert_pos..]);

    result
}

fn canonical_xmp(vocab_key: &str) -> Vec<u8> {
    format!(
        r#"<?xpacket begin="﻿" id="W5M0MpCehiHzreSzNTczkc9d"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/">
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
<rdf:Description rdf:about=""
    xmlns:plus="http://ns.useplus.org/ldf/xmp/1.0/"
    plus:DataMining="{vocab_key}">
</rdf:Description>
</rdf:RDF>
</x:xmpmeta>
<?xpacket end="w"?>"#
    )
    .into_bytes()
}

fn legacy_xmp_dmi_prohibited() -> Vec<u8> {
    r#"<?xpacket begin="﻿" id="W5M0MpCehiHzreSzNTczkc9d"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/">
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
<rdf:Description rdf:about=""
    xmlns:iptc4xmpExt="http://iptc.org/std/Iptc4xmpExt/2008-02-29/"
    Iptc4xmpExt:DMI-Prohibited="ProhibitedAiMlTraining">
</rdf:Description>
</rdf:RDF>
</x:xmpmeta>
<?xpacket end="w"?>"#
        .as_bytes()
        .to_vec()
}

#[test]
fn canonical_plus_dmi_prohibited_ai_training() {
    let xmp = canonical_xmp("DMI-PROHIBITED-AIMLTRAINING");
    let png = make_png_with_xmp(&xmp);
    let report = verify_legal_notice(&png, b"");
    assert_eq!(
        report.canonical_dmi(),
        Some(DmiValue::ProhibitedAiMlTraining)
    );
    assert_eq!(report.dmi(), Some(DmiValue::ProhibitedAiMlTraining));
    assert_eq!(
        report.rights_signal_kind(),
        RightsSignalKind::CanonicalPlusDataMining
    );
}

#[test]
fn canonical_plus_dmi_allowed() {
    let xmp = canonical_xmp("DMI-ALLOWED");
    let png = make_png_with_xmp(&xmp);
    let report = verify_legal_notice(&png, b"");
    assert_eq!(report.canonical_dmi(), Some(DmiValue::Allowed));
    assert_eq!(
        report.rights_signal_kind(),
        RightsSignalKind::CanonicalPlusDataMining
    );
}

#[test]
fn canonical_plus_dmi_prohibited() {
    let xmp = canonical_xmp("DMI-PROHIBITED");
    let png = make_png_with_xmp(&xmp);
    let report = verify_legal_notice(&png, b"");
    assert_eq!(report.canonical_dmi(), Some(DmiValue::Prohibited));
}

#[test]
fn canonical_plus_dmi_prohibited_gen_ai() {
    let xmp = canonical_xmp("DMI-PROHIBITED-GENAIMLTRAINING");
    let png = make_png_with_xmp(&xmp);
    let report = verify_legal_notice(&png, b"");
    assert_eq!(
        report.canonical_dmi(),
        Some(DmiValue::ProhibitedGenAiMlTraining)
    );
}

#[test]
fn canonical_plus_dmi_unspecified() {
    let xmp = canonical_xmp("DMI-UNSPECIFIED");
    let png = make_png_with_xmp(&xmp);
    let report = verify_legal_notice(&png, b"");
    assert_eq!(report.canonical_dmi(), Some(DmiValue::Unspecified));
}

#[test]
fn legacy_dmi_prohibited_backward_compat() {
    let xmp = legacy_xmp_dmi_prohibited();
    let png = make_png_with_xmp(&xmp);
    let report = verify_legal_notice(&png, b"");
    assert_eq!(report.legacy_dmi(), Some(DmiValue::ProhibitedAiMlTraining));
    assert_eq!(
        report.rights_signal_kind(),
        RightsSignalKind::LegacyStegoEggoDmi
    );
}

#[test]
fn unknown_plus_vocab_key() {
    let xmp = canonical_xmp("DMI-CUSTOM-UNKNOWN");
    let png = make_png_with_xmp(&xmp);
    let report = verify_legal_notice(&png, b"");
    assert_eq!(report.canonical_dmi(), None);
    assert_eq!(report.dmi(), None);
}

#[test]
fn no_rights_metadata() {
    let img = image::DynamicImage::new_rgb8(1, 1);
    let mut png = Vec::new();
    {
        use image::ImageEncoder;
        let encoder = image::codecs::png::PngEncoder::new(&mut png);
        let rgb = img.to_rgb8();
        encoder
            .write_image(&rgb, 1, 1, image::ExtendedColorType::Rgb8)
            .unwrap();
    }

    let report = verify_legal_notice(&png, b"");
    assert_eq!(report.dmi(), None);
    assert_eq!(report.canonical_dmi(), None);
    assert_eq!(report.legacy_dmi(), None);
}

#[test]
fn conflict_detection_canonical_vs_legacy() {
    let xmp = canonical_xmp("DMI-PROHIBITED");
    let png = make_png_with_xmp(&xmp);
    let report = verify_legal_notice(&png, b"");
    assert!(!report.has_dmi_conflict());
    assert_eq!(report.canonical_dmi(), Some(DmiValue::Prohibited));
}

fn canonical_xmp_element_form(vocab_key: &str) -> Vec<u8> {
    format!(
        r#"<?xpacket begin="﻿" id="W5M0MpCehiHzreSzNTczkc9d"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/">
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
<rdf:Description rdf:about=""
    xmlns:plus="http://ns.useplus.org/ldf/xmp/1.0/">
    <plus:DataMining>{vocab_key}</plus:DataMining>
</rdf:Description>
</rdf:RDF>
</x:xmpmeta>
<?xpacket end="w"?>"#
    )
    .into_bytes()
}

#[test]
fn canonical_plus_element_form() {
    let xmp = canonical_xmp_element_form("DMI-PROHIBITED-AIMLTRAINING");
    let png = make_png_with_xmp(&xmp);
    let report = verify_legal_notice(&png, b"");
    assert_eq!(
        report.canonical_dmi(),
        Some(DmiValue::ProhibitedAiMlTraining)
    );
    assert_eq!(
        report.rights_signal_kind(),
        RightsSignalKind::CanonicalPlusDataMining
    );
}

fn canonical_xmp_alternate_prefix(vocab_key: &str) -> Vec<u8> {
    format!(
        r#"<?xpacket begin="﻿" id="W5M0MpCehiHzreSzNTczkc9d"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/">
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
<rdf:Description rdf:about=""
    xmlns:myplus="http://ns.useplus.org/ldf/xmp/1.0/"
    myplus:DataMining="{vocab_key}">
</rdf:Description>
</rdf:RDF>
</x:xmpmeta>
<?xpacket end="w"?>"#
    )
    .into_bytes()
}

#[test]
fn alternate_namespace_prefix_resolves() {
    let xmp = canonical_xmp_alternate_prefix("DMI-PROHIBITED-AIMLTRAINING");
    let png = make_png_with_xmp(&xmp);
    let report = verify_legal_notice(&png, b"");
    assert_eq!(
        report.canonical_dmi(),
        Some(DmiValue::ProhibitedAiMlTraining)
    );
    assert_eq!(
        report.rights_signal_kind(),
        RightsSignalKind::CanonicalPlusDataMining
    );
}

fn canonical_and_legacy_xmp(canonical_key: &str, legacy_value: &str) -> Vec<u8> {
    format!(
        r#"<?xpacket begin="﻿" id="W5M0MpCehiHzreSzNTczkc9d"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/">
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
<rdf:Description rdf:about=""
    xmlns:plus="http://ns.useplus.org/ldf/xmp/1.0/"
    xmlns:iptc4xmpExt="http://iptc.org/std/Iptc4xmpExt/2008-02-29/"
    plus:DataMining="{canonical_key}"
    Iptc4xmpExt:DMI-Prohibited="{legacy_value}">
</rdf:Description>
</rdf:RDF>
</x:xmpmeta>
<?xpacket end="w"?>"#
    )
    .into_bytes()
}

#[test]
fn canonical_and_legacy_agreeing_no_conflict() {
    let xmp = canonical_and_legacy_xmp("DMI-PROHIBITED-AIMLTRAINING", "ProhibitedAiMlTraining");
    let png = make_png_with_xmp(&xmp);
    let report = verify_legal_notice(&png, b"");
    assert_eq!(
        report.canonical_dmi(),
        Some(DmiValue::ProhibitedAiMlTraining)
    );
    assert_eq!(report.legacy_dmi(), Some(DmiValue::ProhibitedAiMlTraining));
    assert!(!report.has_dmi_conflict());
    assert_eq!(
        report.rights_signal_kind(),
        RightsSignalKind::CanonicalPlusDataMining
    );
}

#[test]
fn canonical_and_legacy_conflicting_detected() {
    let xmp = canonical_and_legacy_xmp("DMI-ALLOWED", "ProhibitedAiMlTraining");
    let png = make_png_with_xmp(&xmp);
    let report = verify_legal_notice(&png, b"");
    assert_eq!(report.canonical_dmi(), Some(DmiValue::Allowed));
    assert_eq!(report.legacy_dmi(), Some(DmiValue::ProhibitedAiMlTraining));
    assert!(report.has_dmi_conflict());
    assert_eq!(
        report.rights_signal_kind(),
        RightsSignalKind::CanonicalPlusDataMining
    );
}

fn malformed_xmp() -> Vec<u8> {
    "<?xpacket begin=\"﻿\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\n\
      <x:xmpmeta xmlns:x=\"adobe:ns:meta/\">\n\
      <rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\n\
      <rdf:Description rdf:about=\"\"\n\
      <plus:DataMining>unclosed\n\
      </x:xmpmeta>\n\
      <?xpacket end=\"w\"?>"
        .as_bytes()
        .to_vec()
}

#[test]
fn malformed_xmp_does_not_panic() {
    let png = make_png_with_xmp(&malformed_xmp());
    let report = verify_legal_notice(&png, b"");
    assert_eq!(report.dmi(), None);
    assert_eq!(report.canonical_dmi(), None);
}
