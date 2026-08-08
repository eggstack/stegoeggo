//! XMP parsing and namespace-aware field filtering using `quick-xml`.
//!
//! Provides ownership-based XMP field stripping by namespace URI and local name,
//! and namespace conflict detection for XMP packet merging.

use quick_xml::events::attributes::Attribute;
use quick_xml::events::Event;
use quick_xml::name::ResolveResult;
use quick_xml::reader::NsReader;

use crate::error::{Error, Result};

pub(crate) const PLUS_NAMESPACE: &str = "http://ns.useplus.org/ldf/xmp/1.0/";
pub(crate) const STEGOEGGO_NAMESPACE: &str = "https://github.com/eggstack/stegoeggo";
pub(crate) const RDF_NAMESPACE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OwnedField {
    pub namespace_uri: &'static str,
    pub local_name: &'static str,
}

impl OwnedField {
    const fn new(namespace_uri: &'static str, local_name: &'static str) -> Self {
        Self {
            namespace_uri,
            local_name,
        }
    }
}

pub(crate) const OWNED_FIELDS: &[OwnedField] = &[
    OwnedField::new(PLUS_NAMESPACE, "DataMining"),
    OwnedField::new(PLUS_NAMESPACE, "OtherConstraints"),
    OwnedField::new(STEGOEGGO_NAMESPACE, "ProtectionSeed"),
    OwnedField::new(STEGOEGGO_NAMESPACE, "ProtectionLevel"),
    OwnedField::new(STEGOEGGO_NAMESPACE, "RightsPolicy"),
    OwnedField::new(STEGOEGGO_NAMESPACE, "AIConstraints"),
    OwnedField::new(STEGOEGGO_NAMESPACE, "CopyrightOwner"),
    OwnedField::new(STEGOEGGO_NAMESPACE, "LicensorName"),
    OwnedField::new(STEGOEGGO_NAMESPACE, "LicensorEmail"),
    OwnedField::new(STEGOEGGO_NAMESPACE, "LicensorURL"),
    OwnedField::new(STEGOEGGO_NAMESPACE, "NoticeAppliedAt"),
];

fn is_owned_field(uri: &[u8], local: &[u8]) -> bool {
    let uri_str = match std::str::from_utf8(uri) {
        Ok(s) => s,
        Err(_) => return false,
    };
    let local_str = match std::str::from_utf8(local) {
        Ok(s) => s,
        Err(_) => return false,
    };
    OWNED_FIELDS
        .iter()
        .any(|f| f.namespace_uri == uri_str && f.local_name == local_str)
}

#[derive(Debug, Clone)]
pub(crate) struct PreservedDescription {
    pub xml: Vec<u8>,
    #[allow(dead_code)]
    pub has_unrelated: bool,
}

fn xmp_xml_error(e: quick_xml::Error) -> Error {
    Error::Metadata(format!("XMP XML error: {}", e))
}

fn xmp_attr_error(e: quick_xml::events::attributes::AttrError) -> Error {
    Error::Metadata(format!("XMP attribute error: {}", e))
}

fn xmp_internal_error(msg: &str) -> Error {
    Error::Metadata(format!("XMP internal state error: {}", msg))
}

fn is_xmlns_attr(key: &[u8]) -> bool {
    key.starts_with(b"xmlns:") || key == b"xmlns"
}

fn is_xml_lang_attr(key: &[u8]) -> bool {
    key.starts_with(b"xml:") && key.len() > 4
}

#[derive(Debug, Clone, Default)]
struct NsStack {
    frames: Vec<Vec<(Vec<u8>, Vec<u8>)>>,
}

impl NsStack {
    fn new() -> Self {
        Self {
            frames: vec![Vec::new()],
        }
    }

    fn push_frame(&mut self) {
        let parent = self.frames.last().cloned().unwrap_or_default();
        self.frames.push(parent);
    }

    fn pop_frame(&mut self) {
        if self.frames.len() > 1 {
            self.frames.pop();
        }
    }

    fn declare(&mut self, prefix: &[u8], uri: &[u8]) {
        if let Some(frame) = self.frames.last_mut() {
            frame.retain(|(p, _)| p.as_slice() != prefix);
            frame.push((prefix.to_vec(), uri.to_vec()));
        }
    }

    fn lookup(&self, prefix: &[u8]) -> Option<Vec<u8>> {
        for frame in self.frames.iter().rev() {
            for (p, uri) in frame.iter().rev() {
                if p.as_slice() == prefix {
                    return Some(uri.clone());
                }
            }
        }
        None
    }

    fn snapshot(&self) -> Vec<(Vec<u8>, Vec<u8>)> {
        let mut out = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for frame in self.frames.iter().rev() {
            for (p, uri) in frame.iter().rev() {
                if seen.insert(p.clone()) {
                    out.push((p.clone(), uri.clone()));
                }
            }
        }
        out
    }
}

fn extract_prefix(key: &[u8]) -> &[u8] {
    if let Some(colon_pos) = key.iter().position(|&b| b == b':') {
        &key[..colon_pos]
    } else {
        &[]
    }
}

fn capture_resolver_bindings(reader: &NsReader<&[u8]>) -> Vec<(Vec<u8>, Vec<u8>)> {
    let mut out = Vec::new();
    let bindings: Vec<_> = reader.resolver().bindings().collect();
    for (prefix_decl, ns) in bindings {
        if let quick_xml::name::PrefixDeclaration::Named(prefix) = prefix_decl {
            if prefix.is_empty() {
                continue;
            }
            out.push((prefix.to_vec(), ns.as_ref().to_vec()));
        }
    }
    out
}

fn extract_local(key: &[u8]) -> &[u8] {
    if let Some(colon_pos) = key.iter().position(|&b| b == b':') {
        &key[colon_pos + 1..]
    } else {
        key
    }
}

fn attribute_is_owned(attr: &Attribute, ns_stack: &NsStack) -> bool {
    let key = attr.key.as_ref();
    if is_xmlns_attr(key) || is_xml_lang_attr(key) {
        return false;
    }
    let prefix = extract_prefix(key);
    if prefix.is_empty() || prefix == b"xml" || prefix == b"xmlns" {
        return false;
    }
    if let Some(uri) = ns_stack.lookup(prefix) {
        is_owned_field(&uri, extract_local(key))
    } else {
        false
    }
}

fn start_element_is_owned(start: &[u8], ns_stack: &NsStack) -> bool {
    let prefix = extract_prefix(start);
    if prefix.is_empty() || prefix == b"xml" || prefix == b"xmlns" {
        return false;
    }
    if let Some(uri) = ns_stack.lookup(prefix) {
        is_owned_field(&uri, extract_local(start))
    } else {
        false
    }
}

#[allow(dead_code)]
fn collect_xmlns_declarations(
    start_bytes: &[u8],
    attrs: &mut std::collections::HashMap<Vec<u8>, Vec<u8>>,
) -> Result<()> {
    let mut reader = NsReader::from_reader(start_bytes);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let event = reader.read_event_into(&mut buf).map_err(xmp_xml_error)?;
    if let Event::Start(start) = event {
        for attr_res in start.attributes() {
            let attr = attr_res.map_err(xmp_attr_error)?;
            let key = attr.key.as_ref();
            if !is_xmlns_attr(key) {
                continue;
            }
            let value = attr
                .decoded_and_normalized_value(quick_xml::XmlVersion::Implicit1_0, reader.decoder())
                .map_err(xmp_xml_error)?
                .into_owned()
                .into_bytes();
            let prefix = if key == b"xmlns" {
                Vec::new()
            } else if key.starts_with(b"xmlns:") {
                key[6..].to_vec()
            } else {
                continue;
            };
            attrs.insert(prefix, value);
        }
    }
    Ok(())
}

fn attr_raw_value(attr: &Attribute, reader: &NsReader<&[u8]>) -> Result<Vec<u8>> {
    let v = attr
        .decoded_and_normalized_value(quick_xml::XmlVersion::Implicit1_0, reader.decoder())
        .map_err(xmp_xml_error)?;
    Ok(v.into_owned().into_bytes())
}

fn escape_attr_value(value: &[u8], out: &mut Vec<u8>) {
    for &b in value {
        match b {
            b'&' => out.extend_from_slice(b"&amp;"),
            b'<' => out.extend_from_slice(b"&lt;"),
            b'>' => out.extend_from_slice(b"&gt;"),
            b'"' => out.extend_from_slice(b"&quot;"),
            _ => out.push(b),
        }
    }
}

fn escape_text_value(value: &[u8], out: &mut Vec<u8>) {
    for &b in value {
        match b {
            b'&' => out.extend_from_slice(b"&amp;"),
            b'<' => out.extend_from_slice(b"&lt;"),
            b'>' => out.extend_from_slice(b"&gt;"),
            _ => out.push(b),
        }
    }
}

fn append_xml_reference(
    reference: &quick_xml::events::BytesRef<'_>,
    out: &mut Vec<u8>,
) -> Result<()> {
    let character = reference
        .resolve_char_ref()
        .map_err(|e| Error::Metadata(format!("Invalid XML character reference: {}", e)))?
        .or_else(|| match &reference[..] {
            b"amp" => Some('&'),
            b"lt" => Some('<'),
            b"gt" => Some('>'),
            b"apos" => Some('\''),
            b"quot" => Some('"'),
            _ => None,
        })
        .ok_or_else(|| Error::Metadata("Unknown XML entity reference".to_string()))?;

    let valid = matches!(character, '\u{9}' | '\u{a}' | '\u{d}')
        || ('\u{20}'..='\u{d7ff}').contains(&character)
        || ('\u{e000}'..='\u{fffd}').contains(&character)
        || ('\u{10000}'..='\u{10ffff}').contains(&character);
    if !valid {
        return Err(Error::Metadata(
            "XML character reference resolves to an illegal XML 1.0 character".to_string(),
        ));
    }

    let mut encoded = [0; 4];
    let value = character.encode_utf8(&mut encoded);
    escape_text_value(value.as_bytes(), out);
    Ok(())
}

fn append_attr(out: &mut Vec<u8>, key: &[u8], value: &[u8]) {
    out.push(b' ');
    out.extend_from_slice(key);
    out.extend_from_slice(b"=\"");
    escape_attr_value(value, out);
    out.push(b'"');
}

/// Filter an existing XMP packet, removing all owned fields and returning preserved
/// descriptions (in document order) that may be merged with new content.
///
/// Returns an `Err` for any malformed packet or unrecognized structural issue.
pub(crate) fn filter_xmp_packet(packet: &[u8]) -> Result<Vec<PreservedDescription>> {
    let packet_str = std::str::from_utf8(packet)
        .map_err(|e| Error::Metadata(format!("XMP packet is not valid UTF-8: {}", e)))?;

    let mut reader = NsReader::from_str(packet_str);
    reader.config_mut().expand_empty_elements = false;
    reader.config_mut().check_end_names = true;

    let mut buf = Vec::new();
    let mut descriptions: Vec<PreservedDescription> = Vec::new();

    let mut current_out: Option<Vec<u8>> = None;
    let mut in_description = false;
    let mut description_has_unrelated = false;
    let mut owned_depth: usize = 0;
    let mut ns_stack = NsStack::new();

    loop {
        let event = reader.read_event_into(&mut buf).map_err(xmp_xml_error)?;

        match event {
            Event::Decl(_decl) => {
                if in_description {
                    return Err(Error::Metadata(
                        "XML declaration inside rdf:Description is not allowed".to_string(),
                    ));
                }
            }
            Event::Start(start) => {
                let (resolve, _) = reader.resolver().resolve_element(start.name());
                let local = start.local_name();
                let local_str = local.as_ref();
                let ns_uri = if let ResolveResult::Bound(ns) = resolve {
                    Some(ns.as_ref().to_vec())
                } else {
                    None
                };

                let is_rdf = local_name_eq(local_str, "RDF")
                    && ns_uri
                        .as_deref()
                        .map(|u| u == RDF_NAMESPACE.as_bytes())
                        .unwrap_or(false);
                let is_desc = local_name_eq(local_str, "Description")
                    && ns_uri
                        .as_deref()
                        .map(|u| u == RDF_NAMESPACE.as_bytes())
                        .unwrap_or(false);

                if !in_description && !is_rdf && !is_desc {
                    continue;
                }

                if !in_description {
                    if is_rdf {
                        continue;
                    }
                    in_description = true;
                    description_has_unrelated = false;
                    owned_depth = 0;

                    let inherited = capture_resolver_bindings(&reader);
                    for (prefix, uri) in inherited {
                        ns_stack.frames[0].insert(0, (prefix, uri));
                    }
                    ns_stack.push_frame();

                    for attr_res in start.attributes() {
                        let attr = attr_res.map_err(xmp_attr_error)?;
                        let key = attr.key.as_ref();
                        if is_xmlns_attr(key) {
                            let value = attr_raw_value(&attr, &reader)?;
                            let prefix = if key == b"xmlns" {
                                Vec::new()
                            } else if key.starts_with(b"xmlns:") {
                                key[6..].to_vec()
                            } else {
                                continue;
                            };
                            ns_stack.declare(&prefix, &value);
                        }
                    }

                    let mut out = Vec::new();
                    append_description_open(&mut out, &start, &ns_stack, &reader, false)?;
                    current_out = Some(out);
                } else {
                    ns_stack.push_frame();
                    for attr_res in start.attributes() {
                        let attr = attr_res.map_err(xmp_attr_error)?;
                        let key = attr.key.as_ref();
                        if is_xmlns_attr(key) {
                            let value = attr_raw_value(&attr, &reader)?;
                            let prefix = if key == b"xmlns" {
                                Vec::new()
                            } else if key.starts_with(b"xmlns:") {
                                key[6..].to_vec()
                            } else {
                                continue;
                            };
                            ns_stack.declare(&prefix, &value);
                        }
                    }
                    if owned_depth > 0 {
                        owned_depth += 1;
                    } else if start_element_is_owned(start.name().as_ref(), &ns_stack) {
                        owned_depth = 1;
                    } else {
                        description_has_unrelated = true;
                        let out = current_out
                            .as_mut()
                            .ok_or_else(|| xmp_internal_error("writer missing"))?;
                        append_start(out, &start, &ns_stack, &reader)?;
                    }
                }
            }
            Event::Empty(empty) => {
                let (resolve, _) = reader.resolver().resolve_element(empty.name());
                let local = empty.local_name();
                let local_str = local.as_ref();
                let ns_uri = if let ResolveResult::Bound(ns) = resolve {
                    Some(ns.as_ref().to_vec())
                } else {
                    None
                };

                let is_rdf = local_name_eq(local_str, "RDF")
                    && ns_uri
                        .as_deref()
                        .map(|u| u == RDF_NAMESPACE.as_bytes())
                        .unwrap_or(false);
                let is_desc = local_name_eq(local_str, "Description")
                    && ns_uri
                        .as_deref()
                        .map(|u| u == RDF_NAMESPACE.as_bytes())
                        .unwrap_or(false);

                if !in_description && !is_rdf && !is_desc {
                    continue;
                }
                if !in_description && is_rdf {
                    continue;
                }

                if !in_description {
                    let inherited = capture_resolver_bindings(&reader);
                    for (prefix, uri) in inherited {
                        ns_stack.frames[0].insert(0, (prefix, uri));
                    }
                }

                ns_stack.push_frame();
                for attr_res in empty.attributes() {
                    let attr = attr_res.map_err(xmp_attr_error)?;
                    let key = attr.key.as_ref();
                    if is_xmlns_attr(key) {
                        let value = attr_raw_value(&attr, &reader)?;
                        let prefix = if key == b"xmlns" {
                            Vec::new()
                        } else if key.starts_with(b"xmlns:") {
                            key[6..].to_vec()
                        } else {
                            continue;
                        };
                        ns_stack.declare(&prefix, &value);
                    }
                }

                if !in_description {
                    let has_unrelated_attr = empty.attributes().any(|a| {
                        a.as_ref().is_ok_and(|attr| {
                            let key = attr.key.as_ref();
                            !(is_xmlns_attr(key) || is_xml_lang_attr(key))
                                && !attribute_is_owned(attr, &ns_stack)
                        })
                    });
                    let mut out = Vec::new();
                    append_description_open(&mut out, &empty, &ns_stack, &reader, true)?;
                    if has_unrelated_attr {
                        descriptions.push(PreservedDescription {
                            xml: out,
                            has_unrelated: true,
                        });
                    }
                    ns_stack.pop_frame();
                } else if owned_depth > 0 {
                    ns_stack.pop_frame();
                } else {
                    let owned = start_element_is_owned(empty.name().as_ref(), &ns_stack);
                    if !owned {
                        description_has_unrelated = true;
                        let out = current_out
                            .as_mut()
                            .ok_or_else(|| xmp_internal_error("writer missing"))?;
                        append_empty(out, &empty, &ns_stack, &reader)?;
                    }
                    ns_stack.pop_frame();
                }
            }
            Event::Text(text) => {
                if in_description && owned_depth == 0 {
                    let bytes: &[u8] = text.as_ref();
                    let has_meaningful = !bytes
                        .iter()
                        .all(|b| matches!(b, b' ' | b'\t' | b'\n' | b'\r'));
                    if has_meaningful {
                        description_has_unrelated = true;
                    }
                    let out = current_out
                        .as_mut()
                        .ok_or_else(|| xmp_internal_error("writer missing"))?;
                    escape_text_value(bytes, out);
                }
            }
            Event::End(end) => {
                if !in_description {
                    continue;
                }
                let (resolve, _) = reader.resolver().resolve_element(end.name());
                let local = end.local_name();
                let ns_uri = if let ResolveResult::Bound(ns) = resolve {
                    Some(ns.as_ref().to_vec())
                } else {
                    None
                };
                let is_rdf_desc = local_name_eq(local.as_ref(), "Description")
                    && ns_uri
                        .as_deref()
                        .map(|u| u == RDF_NAMESPACE.as_bytes())
                        .unwrap_or(false);

                if is_rdf_desc {
                    if owned_depth > 0 {
                        owned_depth -= 1;
                        ns_stack.pop_frame();
                        continue;
                    }
                    let mut out = current_out
                        .take()
                        .ok_or_else(|| xmp_internal_error("writer missing"))?;
                    out.extend_from_slice(b"</rdf:Description>");
                    let xml = out;
                    if description_has_unrelated {
                        descriptions.push(PreservedDescription {
                            xml,
                            has_unrelated: true,
                        });
                    }
                    in_description = false;
                    description_has_unrelated = false;
                    owned_depth = 0;
                    ns_stack.pop_frame();
                } else if owned_depth > 0 {
                    owned_depth -= 1;
                    ns_stack.pop_frame();
                } else {
                    let out = current_out
                        .as_mut()
                        .ok_or_else(|| xmp_internal_error("writer missing"))?;
                    out.push(b'<');
                    out.push(b'/');
                    out.extend_from_slice(end.name().as_ref());
                    out.push(b'>');
                    ns_stack.pop_frame();
                }
            }
            Event::Eof => break,
            Event::Comment(c) => {
                if in_description && owned_depth == 0 {
                    let out = current_out
                        .as_mut()
                        .ok_or_else(|| xmp_internal_error("writer missing"))?;
                    out.extend_from_slice(b"<!--");
                    out.extend_from_slice(c.as_ref());
                    out.extend_from_slice(b"-->");
                }
            }
            Event::CData(cdata) => {
                if in_description && owned_depth == 0 {
                    description_has_unrelated = true;
                    let out = current_out
                        .as_mut()
                        .ok_or_else(|| xmp_internal_error("writer missing"))?;
                    out.extend_from_slice(b"<![CDATA[");
                    out.extend_from_slice(cdata.as_ref());
                    out.extend_from_slice(b"]]>");
                }
            }
            Event::PI(pi) => {
                if in_description && owned_depth == 0 {
                    let out = current_out
                        .as_mut()
                        .ok_or_else(|| xmp_internal_error("writer missing"))?;
                    out.push(b'<');
                    out.push(b'?');
                    out.extend_from_slice(pi.as_ref());
                    out.extend_from_slice(b"?>");
                }
            }
            Event::GeneralRef(reference) => {
                let mut value = Vec::new();
                append_xml_reference(&reference, &mut value)?;
                if in_description && owned_depth == 0 {
                    description_has_unrelated = true;
                    let out = current_out
                        .as_mut()
                        .ok_or_else(|| xmp_internal_error("writer missing"))?;
                    out.extend_from_slice(&value);
                }
            }
            other => {
                return Err(Error::Metadata(format!(
                    "Unsupported XMP event: {:?}",
                    other
                )));
            }
        }
        buf.clear();
    }

    if in_description {
        return Err(Error::Metadata(
            "Truncated XMP packet: rdf:Description was not closed".to_string(),
        ));
    }

    Ok(descriptions)
}

/// Merge one or more preserved descriptions into the canonical new XMP packet
/// using a structural XML event stream (no substring parsing).
///
/// The canonical packet's RDF container is identified by expanded name
/// (`{RDF_NS}RDF`). The preserved descriptions are inserted immediately
/// before the matching `</rdf:RDF>` end event.
///
/// The canonical packet must contain exactly one usable RDF container.
/// Malformed input returns `Err` rather than falling back to the unmodified
/// packet.
pub(crate) fn merge_preserved_descriptions(
    canonical_new_packet: &[u8],
    preserved: &[PreservedDescription],
) -> Result<Vec<u8>> {
    let packet_str = std::str::from_utf8(canonical_new_packet)
        .map_err(|e| Error::Metadata(format!("Canonical XMP packet is not valid UTF-8: {}", e)))?;

    let mut reader = NsReader::from_str(packet_str);
    reader.config_mut().expand_empty_elements = false;
    reader.config_mut().check_end_names = true;

    let mut buf = Vec::new();
    let mut output = Vec::new();
    let mut rdf_depth: usize = 0;
    let mut rdf_open_seen = false;
    let mut inserted = false;

    fn write_attr_value(value: &[u8], out: &mut Vec<u8>) {
        for &b in value {
            match b {
                b'&' => out.extend_from_slice(b"&amp;"),
                b'<' => out.extend_from_slice(b"&lt;"),
                b'>' => out.extend_from_slice(b"&gt;"),
                b'"' => out.extend_from_slice(b"&quot;"),
                _ => out.push(b),
            }
        }
    }

    fn write_escaped_text(value: &[u8], out: &mut Vec<u8>) {
        for &b in value {
            match b {
                b'&' => out.extend_from_slice(b"&amp;"),
                b'<' => out.extend_from_slice(b"&lt;"),
                b'>' => out.extend_from_slice(b"&gt;"),
                _ => out.push(b),
            }
        }
    }

    loop {
        let event = reader.read_event_into(&mut buf).map_err(xmp_xml_error)?;

        match event {
            Event::Eof => break,
            Event::Decl(decl) => {
                let bytes = decl.as_ref();
                output.extend_from_slice(b"<?");
                output.extend_from_slice(bytes);
                output.extend_from_slice(b"?>");
            }
            Event::Comment(c) => {
                output.extend_from_slice(b"<!--");
                output.extend_from_slice(c.as_ref());
                output.extend_from_slice(b"-->");
            }
            Event::PI(pi) => {
                output.push(b'<');
                output.push(b'?');
                output.extend_from_slice(pi.as_ref());
                output.extend_from_slice(b"?>");
            }
            Event::Start(start) => {
                let (resolve, _) = reader.resolver().resolve_element(start.name());
                let local = start.local_name();
                let ns_uri = if let ResolveResult::Bound(ns) = resolve {
                    Some(ns.as_ref().to_vec())
                } else {
                    None
                };
                let is_rdf = local_name_eq(local.as_ref(), "RDF")
                    && ns_uri
                        .as_deref()
                        .map(|u| u == RDF_NAMESPACE.as_bytes())
                        .unwrap_or(false);

                output.push(b'<');
                output.extend_from_slice(start.name().as_ref());
                for attr_res in start.attributes() {
                    let attr = attr_res.map_err(xmp_attr_error)?;
                    output.push(b' ');
                    output.extend_from_slice(attr.key.as_ref());
                    output.extend_from_slice(b"=\"");
                    let value = attr_raw_value(&attr, &reader)?;
                    write_attr_value(&value, &mut output);
                    output.push(b'"');
                }
                output.push(b'>');

                if is_rdf {
                    rdf_open_seen = true;
                    rdf_depth += 1;
                }
            }
            Event::Empty(empty) => {
                output.push(b'<');
                output.extend_from_slice(empty.name().as_ref());
                for attr_res in empty.attributes() {
                    let attr = attr_res.map_err(xmp_attr_error)?;
                    output.push(b' ');
                    output.extend_from_slice(attr.key.as_ref());
                    output.extend_from_slice(b"=\"");
                    let value = attr_raw_value(&attr, &reader)?;
                    write_attr_value(&value, &mut output);
                    output.push(b'"');
                }
                output.extend_from_slice(b"/>");
            }
            Event::Text(text) => {
                let bytes: &[u8] = text.as_ref();
                write_escaped_text(bytes, &mut output);
            }
            Event::CData(cdata) => {
                output.extend_from_slice(b"<![CDATA[");
                output.extend_from_slice(cdata.as_ref());
                output.extend_from_slice(b"]]>");
            }
            Event::End(end) => {
                let (resolve, _) = reader.resolver().resolve_element(end.name());
                let local = end.local_name();
                let ns_uri = if let ResolveResult::Bound(ns) = resolve {
                    Some(ns.as_ref().to_vec())
                } else {
                    None
                };
                let is_rdf = local_name_eq(local.as_ref(), "RDF")
                    && ns_uri
                        .as_deref()
                        .map(|u| u == RDF_NAMESPACE.as_bytes())
                        .unwrap_or(false);

                if is_rdf && !inserted {
                    for desc in preserved {
                        output.extend_from_slice(&desc.xml);
                    }
                    inserted = true;
                }
                output.push(b'<');
                output.push(b'/');
                output.extend_from_slice(end.name().as_ref());
                output.push(b'>');

                if is_rdf {
                    rdf_depth -= 1;
                }
            }
            Event::GeneralRef(reference) => {
                append_xml_reference(&reference, &mut output)?;
            }
            other => {
                return Err(Error::Metadata(format!(
                    "Unsupported XMP event in merge: {:?}",
                    other
                )));
            }
        }
        buf.clear();
    }

    if !rdf_open_seen {
        return Err(Error::Metadata(
            "Canonical XMP packet missing rdf:RDF container".to_string(),
        ));
    }
    if rdf_depth != 0 {
        return Err(Error::Metadata(
            "Canonical XMP packet RDF container was not closed".to_string(),
        ));
    }
    if !inserted {
        return Err(Error::Metadata(
            "Canonical XMP packet did not contain a closing rdf:RDF".to_string(),
        ));
    }

    Ok(output)
}

/// Deduplicate preserved descriptions by byte-identical serialized XML,
/// preserving the first occurrence's order. Also excludes any preserved
/// description that is byte-identical to a canonical description.
pub(crate) fn deduplicate_descriptions(
    descs: &[PreservedDescription],
    canonical: &[PreservedDescription],
) -> Vec<PreservedDescription> {
    let mut seen: std::collections::HashSet<Vec<u8>> = std::collections::HashSet::new();
    for c in canonical {
        seen.insert(c.xml.clone());
    }
    let mut out: Vec<PreservedDescription> = Vec::new();
    for d in descs {
        if seen.insert(d.xml.clone()) {
            out.push(d.clone());
        }
    }
    out
}

#[allow(dead_code)]
fn namespace_uri_eq(a: quick_xml::name::Namespace<'_>, b: &str) -> bool {
    if let Ok(s) = std::str::from_utf8(a.as_ref()) {
        s == b
    } else {
        false
    }
}

fn local_name_eq(a: &[u8], b: &str) -> bool {
    if let Ok(s) = std::str::from_utf8(a) {
        s == b
    } else {
        false
    }
}

#[allow(dead_code)]
fn resolved_uri_eq(resolved: ResolveResult<'_>, target: &str) -> bool {
    if let ResolveResult::Bound(ns) = resolved {
        namespace_uri_eq(ns, target)
    } else {
        false
    }
}

fn append_description_open(
    out: &mut Vec<u8>,
    start: &quick_xml::events::BytesStart<'_>,
    ns_stack: &NsStack,
    reader: &NsReader<&[u8]>,
    self_closing: bool,
) -> Result<()> {
    out.push(b'<');
    out.extend_from_slice(b"rdf:Description");

    let mut written: std::collections::BTreeSet<Vec<u8>> = std::collections::BTreeSet::new();

    for attr_res in start.attributes() {
        let attr = attr_res.map_err(xmp_attr_error)?;
        let key = attr.key.as_ref();
        if !is_xmlns_attr(key) && !is_xml_lang_attr(key) && attribute_is_owned(&attr, ns_stack) {
            continue;
        }
        let value = attr_raw_value(&attr, reader)?;
        append_attr(out, key, &value);
        written.insert(key.to_vec());
    }

    let rdf_uri = RDF_NAMESPACE.as_bytes();
    let rdf_attr = b"xmlns:rdf" as &[u8];
    let has_rdf_decl = written.contains(rdf_attr);
    if !has_rdf_decl {
        let already = start.attributes().any(|a| {
            a.as_ref()
                .is_ok_and(|attr| attr.key.as_ref() == rdf_attr && attr.value.as_ref() == rdf_uri)
        });
        if !already {
            append_attr(out, rdf_attr, rdf_uri);
            written.insert(rdf_attr.to_vec());
        }
    }

    let mut inherited: Vec<(Vec<u8>, Vec<u8>)> = ns_stack
        .snapshot()
        .into_iter()
        .filter(|(p, _)| !p.is_empty())
        .collect();
    inherited.sort_by(|a, b| a.0.cmp(&b.0));

    for (prefix, uri) in inherited {
        let mut attr_name = Vec::with_capacity(prefix.len() + 6);
        attr_name.extend_from_slice(b"xmlns:");
        attr_name.extend_from_slice(&prefix);
        if written.contains(&attr_name) {
            continue;
        }
        let already = start.attributes().any(|a| {
            a.as_ref().is_ok_and(|attr| {
                attr.key.as_ref() == attr_name.as_slice() && attr.value.as_ref() == uri.as_slice()
            })
        });
        if already {
            continue;
        }
        append_attr(out, &attr_name, &uri);
        written.insert(attr_name);
    }

    if self_closing {
        out.extend_from_slice(b"/>");
    } else {
        out.push(b'>');
    }
    Ok(())
}

fn append_start(
    out: &mut Vec<u8>,
    start: &quick_xml::events::BytesStart<'_>,
    ns_stack: &NsStack,
    reader: &NsReader<&[u8]>,
) -> Result<()> {
    out.push(b'<');
    out.extend_from_slice(start.name().as_ref());

    let frame_prefixes: std::collections::HashSet<Vec<u8>> = ns_stack
        .frames
        .last()
        .map(|f| f.iter().map(|(p, _)| p.clone()).collect())
        .unwrap_or_default();

    for attr_res in start.attributes() {
        let attr = attr_res.map_err(xmp_attr_error)?;
        let key = attr.key.as_ref();
        if is_xmlns_attr(key) || is_xml_lang_attr(key) {
            let value = attr_raw_value(&attr, reader)?;
            append_attr(out, key, &value);
            continue;
        }
        if attribute_is_owned(&attr, ns_stack) {
            continue;
        }
        let value = attr_raw_value(&attr, reader)?;
        append_attr(out, key, &value);
    }

    for (prefix, uri) in ns_stack.snapshot() {
        if prefix.is_empty() {
            continue;
        }
        if !frame_prefixes.contains(&prefix) {
            continue;
        }
        let mut attr_name = Vec::with_capacity(prefix.len() + 6);
        attr_name.extend_from_slice(b"xmlns:");
        attr_name.extend_from_slice(&prefix);
        let already = start.attributes().any(|a| {
            a.as_ref()
                .is_ok_and(|attr| attr.key.as_ref() == attr_name.as_slice())
        });
        if already {
            continue;
        }
        append_attr(out, &attr_name, &uri);
    }

    out.push(b'>');
    Ok(())
}

fn append_empty(
    out: &mut Vec<u8>,
    empty: &quick_xml::events::BytesStart<'_>,
    ns_stack: &NsStack,
    reader: &NsReader<&[u8]>,
) -> Result<()> {
    out.push(b'<');
    out.extend_from_slice(empty.name().as_ref());

    let frame_prefixes: std::collections::HashSet<Vec<u8>> = ns_stack
        .frames
        .last()
        .map(|f| f.iter().map(|(p, _)| p.clone()).collect())
        .unwrap_or_default();

    for attr_res in empty.attributes() {
        let attr = attr_res.map_err(xmp_attr_error)?;
        let key = attr.key.as_ref();
        if is_xmlns_attr(key) || is_xml_lang_attr(key) {
            let value = attr_raw_value(&attr, reader)?;
            append_attr(out, key, &value);
            continue;
        }
        if attribute_is_owned(&attr, ns_stack) {
            continue;
        }
        let value = attr_raw_value(&attr, reader)?;
        append_attr(out, key, &value);
    }

    for (prefix, uri) in ns_stack.snapshot() {
        if prefix.is_empty() {
            continue;
        }
        if !frame_prefixes.contains(&prefix) {
            continue;
        }
        let mut attr_name = Vec::with_capacity(prefix.len() + 6);
        attr_name.extend_from_slice(b"xmlns:");
        attr_name.extend_from_slice(&prefix);
        let already = empty.attributes().any(|a| {
            a.as_ref()
                .is_ok_and(|attr| attr.key.as_ref() == attr_name.as_slice())
        });
        if already {
            continue;
        }
        append_attr(out, &attr_name, &uri);
    }

    out.extend_from_slice(b"/>");
    Ok(())
}

#[cfg(test)]
#[cfg(test)]
mod tests {
    use super::*;

    const PACKET_TEMPLATE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/" xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
<rdf:RDF>
{descriptions}
</rdf:RDF>
</x:xmpmeta>"#;

    fn build_packet(descriptions: &str) -> Vec<u8> {
        PACKET_TEMPLATE
            .replace("{descriptions}", descriptions)
            .into_bytes()
    }

    #[test]
    fn recognizes_rdf_description_with_standard_prefix() {
        let packet = build_packet(r#"<rdf:Description rdf:about=""/>"#);
        let result = filter_xmp_packet(&packet).expect("should parse");
        assert_eq!(result.len(), 1, "one description found");
    }

    #[test]
    fn recognizes_rdf_description_with_alternate_prefix() {
        let packet = r#"<?xml version="1.0" encoding="UTF-8"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/">
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
<r:Description xmlns:r="http://www.w3.org/1999/02/22-rdf-syntax-ns#" rdf:about=""/>
</rdf:RDF>
</x:xmpmeta>"#
            .as_bytes()
            .to_vec();
        let _ = filter_xmp_packet(&packet).expect("alternate prefix should parse");
    }

    #[test]
    fn recognizes_namespace_declared_on_outer_xmpmeta() {
        let packet = r#"<?xml version="1.0" encoding="UTF-8"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/" xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
            xmlns:dc="http://purl.org/dc/elements/1.1/">
<rdf:RDF>
<rdf:Description rdf:about="">
<dc:creator>OuterNsCreator</dc:creator>
</rdf:Description>
</rdf:RDF>
</x:xmpmeta>"#
            .as_bytes()
            .to_vec();
        let result = filter_xmp_packet(&packet).expect("should parse");
        assert_eq!(result.len(), 1);
        let xml = std::str::from_utf8(&result[0].xml).expect("utf8");
        assert!(xml.contains("OuterNsCreator"));
        assert!(xml.contains("xmlns:dc"));
    }

    #[test]
    fn recognizes_namespace_declared_on_rdf_element() {
        let packet = r#"<?xml version="1.0" encoding="UTF-8"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/">
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:dc="http://purl.org/dc/elements/1.1/">
<rdf:Description rdf:about="">
<dc:creator>RdfNsCreator</dc:creator>
</rdf:Description>
</rdf:RDF>
</x:xmpmeta>"#
            .as_bytes()
            .to_vec();
        let result = filter_xmp_packet(&packet).expect("should parse");
        assert_eq!(result.len(), 1);
        let xml = std::str::from_utf8(&result[0].xml).expect("utf8");
        assert!(xml.contains("RdfNsCreator"));
        assert!(xml.contains("xmlns:dc"));
    }

    #[test]
    fn removes_owned_plus_attribute_under_alternate_prefix() {
        let packet = r#"<?xml version="1.0" encoding="UTF-8"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/" xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
<rdf:RDF>
<rdf:Description xmlns:p="http://ns.useplus.org/ldf/xmp/1.0/"
                p:DataMining="http://example.com/v1"/>
</rdf:RDF>
</x:xmpmeta>"#
            .as_bytes()
            .to_vec();
        let result = filter_xmp_packet(&packet).expect("should parse");
        assert!(
            result.is_empty(),
            "owned-only description should be removed; got: {:?}",
            result
        );
    }

    #[test]
    fn removes_owned_stegoeggo_attribute_under_alternate_prefix() {
        let packet = r#"<?xml version="1.0" encoding="UTF-8"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/" xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
<rdf:RDF>
<rdf:Description xmlns:s="https://github.com/eggstack/stegoeggo"
                s:ProtectionSeed="123"/>
</rdf:RDF>
</x:xmpmeta>"#
            .as_bytes()
            .to_vec();
        let result = filter_xmp_packet(&packet).expect("should parse");
        assert!(
            result.is_empty(),
            "owned-only description should be removed; got: {:?}",
            result
        );
    }

    #[test]
    fn preserves_same_local_name_under_unrelated_namespace() {
        let packet = r#"<?xml version="1.0" encoding="UTF-8"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/" xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
<rdf:RDF>
<rdf:Description xmlns:other="http://example.com/other/"
                other:DataMining="UnrelatedDataMiningValue"/>
</rdf:RDF>
</x:xmpmeta>"#
            .as_bytes()
            .to_vec();
        let result = filter_xmp_packet(&packet).expect("should parse");
        assert_eq!(result.len(), 1);
        let xml = std::str::from_utf8(&result[0].xml).expect("utf8");
        assert!(
            xml.contains("UnrelatedDataMiningValue"),
            "unrelated same-local should survive: {}",
            xml
        );
    }

    #[test]
    fn preserves_unprefixed_same_local_name() {
        let packet = build_packet(r#"<rdf:Description DataMining="UnrelatedUnprefixed"/>"#);
        let result = filter_xmp_packet(&packet).expect("should parse");
        assert_eq!(result.len(), 1);
        let xml = std::str::from_utf8(&result[0].xml).expect("utf8");
        assert!(xml.contains("UnrelatedUnprefixed"));
    }

    #[test]
    fn removes_owned_child_element_subtree() {
        let packet = r#"<?xml version="1.0" encoding="UTF-8"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/" xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
<rdf:RDF>
<rdf:Description xmlns:plus="http://ns.useplus.org/ldf/xmp/1.0/">
<plus:DataMining>some-claim</plus:DataMining>
</rdf:Description>
</rdf:RDF>
</x:xmpmeta>"#
            .as_bytes()
            .to_vec();
        let result = filter_xmp_packet(&packet).expect("should parse");
        assert!(result.is_empty());
    }

    #[test]
    fn preserves_unrelated_child_before_and_after_owned_child() {
        let packet = r#"<?xml version="1.0" encoding="UTF-8"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/" xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
<rdf:RDF>
<rdf:Description xmlns:plus="http://ns.useplus.org/ldf/xmp/1.0/"
                xmlns:dc="http://purl.org/dc/elements/1.1/">
<dc:creator>BeforeOwner</dc:creator>
<plus:DataMining>owned</plus:DataMining>
<dc:rights>AfterOwner</dc:rights>
</rdf:Description>
</rdf:RDF>
</x:xmpmeta>"#
            .as_bytes()
            .to_vec();
        let result = filter_xmp_packet(&packet).expect("should parse");
        assert_eq!(result.len(), 1);
        let xml = std::str::from_utf8(&result[0].xml).expect("utf8");
        assert!(xml.contains("BeforeOwner"));
        assert!(xml.contains("AfterOwner"));
        assert!(!xml.contains("owned"));
    }

    #[test]
    fn preserves_nested_unrelated_elements_and_text() {
        let packet = r#"<?xml version="1.0" encoding="UTF-8"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/" xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
<rdf:RDF>
<rdf:Description xmlns:dc="http://purl.org/dc/elements/1.1/">
<dc:rights>
<rdf:Alt>
<rdf:li xml:lang="x-default">Nested Text</rdf:li>
</rdf:Alt>
</dc:rights>
</rdf:Description>
</rdf:RDF>
</x:xmpmeta>"#
            .as_bytes()
            .to_vec();
        let result = filter_xmp_packet(&packet).expect("should parse");
        assert_eq!(result.len(), 1);
        let xml = std::str::from_utf8(&result[0].xml).expect("utf8");
        assert!(xml.contains("Nested Text"));
    }

    #[test]
    fn preserves_attribute_value_containing_greater_than() {
        let packet = r#"<?xml version="1.0" encoding="UTF-8"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/" xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
<rdf:RDF>
<rdf:Description xmlns:dc="http://purl.org/dc/elements/1.1/"
                dc:rights="A &gt; B"/>
</rdf:RDF>
</x:xmpmeta>"#
            .as_bytes()
            .to_vec();
        let result = filter_xmp_packet(&packet).expect("should parse");
        assert_eq!(result.len(), 1);
        let xml = std::str::from_utf8(&result[0].xml).expect("utf8");
        assert!(xml.contains("A &gt; B"));
    }

    #[test]
    fn preserves_comments_and_processing_instructions_where_supported() {
        let packet = r#"<?xml version="1.0" encoding="UTF-8"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/" xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
<!-- a comment -->
<?xpacket begin="" id="W5M0MpCehiHzreSzNTczkc9d"?>
<rdf:RDF>
<rdf:Description rdf:about=""/>
</rdf:RDF>
</x:xmpmeta>"#
            .as_bytes()
            .to_vec();
        let _ = filter_xmp_packet(&packet).expect("should parse with comments/PIs");
    }

    #[test]
    fn rejects_invalid_utf8() {
        let bad: &[u8] = &[0xFF, 0xFE, 0x00, 0x3C];
        let result = filter_xmp_packet(bad);
        assert!(result.is_err(), "non-UTF8 must fail");
    }

    #[test]
    fn rejects_unclosed_description() {
        let packet = r#"<?xml version="1.0" encoding="UTF-8"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/" xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
<rdf:RDF>
<rdf:Description rdf:about="">
</rdf:RDF>
</x:xmpmeta>"#
            .as_bytes()
            .to_vec();
        let result = filter_xmp_packet(&packet);
        assert!(result.is_err(), "unclosed rdf:Description must fail");
    }

    #[test]
    fn rejects_mismatched_end_tag() {
        let packet = build_packet(r#"<rdf:Description></OtherDesc></rdf:RDF>"#);
        let result = filter_xmp_packet(&packet);
        assert!(result.is_err(), "mismatched end tag must fail");
    }

    #[test]
    fn rejects_truncated_owned_element() {
        let packet = r#"<?xml version="1.0" encoding="UTF-8"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/" xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
<rdf:RDF>
<rdf:Description xmlns:plus="http://ns.useplus.org/ldf/xmp/1.0/">
<plus:DataMining>
</rdf:RDF>
</x:xmpmeta>"#
            .as_bytes()
            .to_vec();
        let result = filter_xmp_packet(&packet);
        assert!(result.is_err(), "truncated owned element must fail");
    }

    #[test]
    fn owned_only_description_is_removed() {
        let packet = r#"<?xml version="1.0" encoding="UTF-8"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/" xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
<rdf:RDF>
<rdf:Description xmlns:plus="http://ns.useplus.org/ldf/xmp/1.0/">
<plus:DataMining>X</plus:DataMining>
<plus:OtherConstraints>Y</plus:OtherConstraints>
</rdf:Description>
</rdf:RDF>
</x:xmpmeta>"#
            .as_bytes()
            .to_vec();
        let result = filter_xmp_packet(&packet).expect("should parse");
        assert!(result.is_empty(), "owned-only description must be removed");
    }

    #[test]
    fn mixed_description_is_returned_without_owned_fields() {
        let packet = r#"<?xml version="1.0" encoding="UTF-8"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/" xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
<rdf:RDF>
<rdf:Description xmlns:plus="http://ns.useplus.org/ldf/xmp/1.0/"
                xmlns:dc="http://purl.org/dc/elements/1.1/">
<dc:creator>Alice</dc:creator>
<plus:DataMining>claim</plus:DataMining>
</rdf:Description>
</rdf:RDF>
</x:xmpmeta>"#
            .as_bytes()
            .to_vec();
        let result = filter_xmp_packet(&packet).expect("should parse");
        assert_eq!(result.len(), 1);
        let xml = std::str::from_utf8(&result[0].xml).expect("utf8");
        assert!(xml.contains("Alice"));
        assert!(!xml.contains("claim"));
        assert!(!xml.contains("plus:DataMining"));
    }

    #[test]
    fn mixed_empty_description_attributes_are_returned_without_owned_fields() {
        let packet = r#"<?xml version="1.0" encoding="UTF-8"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/" xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
<rdf:RDF><rdf:Description xmlns:dc="http://purl.org/dc/elements/1.1/"
xmlns:plus="http://ns.useplus.org/ldf/xmp/1.0/"
dc:creator="Alice"
plus:DataMining="old-claim"/></rdf:RDF>
</x:xmpmeta>"#
            .as_bytes()
            .to_vec();
        let result = filter_xmp_packet(&packet).expect("should parse");
        assert_eq!(result.len(), 1, "got {:?}", result);
        let xml = std::str::from_utf8(&result[0].xml).expect("utf8");
        assert!(xml.contains("Alice"));
        assert!(!xml.contains("old-claim"));
        assert!(!xml.contains("plus:DataMining"));
    }

    fn assert_rdf_qualified(xml: &[u8]) {
        let packet_str = std::str::from_utf8(xml).expect("utf8");
        let mut reader = NsReader::from_str(packet_str);
        let mut buf = Vec::new();
        let mut found_rdf_desc = false;
        loop {
            let event = reader.read_event_into(&mut buf).expect("parse");
            match event {
                Event::Start(s) | Event::Empty(s) => {
                    let (resolve, _) = reader.resolver().resolve_element(s.name());
                    if let ResolveResult::Bound(ns) = resolve {
                        let ns_str = ns.as_ref();
                        if local_name_eq(s.local_name().as_ref(), "Description")
                            && ns_str == RDF_NAMESPACE.as_bytes()
                        {
                            found_rdf_desc = true;
                        }
                    }
                }
                Event::Eof => break,
                _ => {}
            }
        }
        assert!(
            found_rdf_desc,
            "rdf:Description (RDF namespace) not found in: {}",
            packet_str
        );
    }

    #[test]
    fn preserved_description_remains_rdf_qualified() {
        let packet = r#"<?xml version="1.0" encoding="UTF-8"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/" xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
<rdf:RDF>
<rdf:Description rdf:about="">
<dc:creator xmlns:dc="http://purl.org/dc/elements/1.1/">Example</dc:creator>
</rdf:Description>
</rdf:RDF>
</x:xmpmeta>"#
            .as_bytes()
            .to_vec();
        let result = filter_xmp_packet(&packet).expect("should parse");
        assert_eq!(result.len(), 1);
        assert_rdf_qualified(&result[0].xml);
    }

    #[test]
    fn preserved_description_reparses_as_rdf_description() {
        let packet = r#"<?xml version="1.0" encoding="UTF-8"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/" xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
<rdf:RDF>
<rdf:Description rdf:about="" xmlns:dc="http://purl.org/dc/elements/1.1/"
dc:creator="Example"/>
</rdf:RDF>
</x:xmpmeta>"#
            .as_bytes()
            .to_vec();
        let result = filter_xmp_packet(&packet).expect("should parse");
        assert_eq!(result.len(), 1);
        let xml = std::str::from_utf8(&result[0].xml).expect("utf8");
        let mut reader = NsReader::from_str(xml);
        let mut buf = Vec::new();
        let mut found = false;
        loop {
            let event = reader.read_event_into(&mut buf).expect("reparse");
            match event {
                Event::Empty(s) => {
                    let (resolve, _) = reader.resolver().resolve_element(s.name());
                    if let ResolveResult::Bound(ns) = resolve {
                        if ns.as_ref() == RDF_NAMESPACE.as_bytes()
                            && local_name_eq(s.local_name().as_ref(), "Description")
                        {
                            found = true;
                        }
                    }
                }
                Event::Eof => break,
                _ => {}
            }
        }
        assert!(
            found,
            "preserved description did not reparse as RDF Description"
        );
    }

    #[test]
    fn preserved_description_outer_namespace_becomes_self_contained() {
        let packet = r#"<?xml version="1.0" encoding="UTF-8"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/" xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
            xmlns:dc="http://purl.org/dc/elements/1.1/">
<rdf:RDF>
<rdf:Description rdf:about="">
<dc:creator>OuterNsCreator</dc:creator>
</rdf:Description>
</rdf:RDF>
</x:xmpmeta>"#
            .as_bytes()
            .to_vec();
        let result = filter_xmp_packet(&packet).expect("should parse");
        assert_eq!(result.len(), 1);
        let xml = std::str::from_utf8(&result[0].xml).expect("utf8");
        assert!(
            xml.contains("xmlns:dc"),
            "dc namespace must be self-contained: {}",
            xml
        );
        let reader = NsReader::from_str(xml);
        let mut buf = Vec::new();
        let mut reader = reader;
        loop {
            let event = reader.read_event_into(&mut buf).expect("reparse");
            if let Event::Eof = event {
                break;
            }
        }
    }

    #[test]
    fn alternate_rdf_prefix_normalizes_without_semantic_loss() {
        let packet = r#"<?xml version="1.0" encoding="UTF-8"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/">
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
<r:Description xmlns:r="http://www.w3.org/1999/02/22-rdf-syntax-ns#" rdf:about="" xmlns:dc="http://purl.org/dc/elements/1.1/" dc:creator="Example"/>
</rdf:RDF>
</x:xmpmeta>"#
            .as_bytes()
            .to_vec();
        let result = filter_xmp_packet(&packet).expect("should parse");
        assert_eq!(result.len(), 1);
        let xml = std::str::from_utf8(&result[0].xml).expect("utf8");
        assert!(xml.contains("Example"), "value must survive: {}", xml);
        assert_rdf_qualified(&result[0].xml);
    }

    #[test]
    fn owned_other_constraints_with_rdf_alt_is_removed_whole() {
        let packet = r#"<?xml version="1.0" encoding="UTF-8"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/" xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
<rdf:RDF>
<rdf:Description xmlns:plus="http://ns.useplus.org/ldf/xmp/1.0/">
<plus:OtherConstraints>
<rdf:Alt>
<rdf:li xml:lang="x-default">old constraints</rdf:li>
</rdf:Alt>
</plus:OtherConstraints>
</rdf:Description>
</rdf:RDF>
</x:xmpmeta>"#
            .as_bytes()
            .to_vec();
        let result = filter_xmp_packet(&packet).expect("should parse");
        assert!(
            result.is_empty(),
            "all-owned description must be removed: {:?}",
            result
        );
    }

    #[test]
    fn owned_data_mining_with_nested_rdf_structure_is_removed_whole() {
        let packet = r#"<?xml version="1.0" encoding="UTF-8"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/" xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
<rdf:RDF>
<rdf:Description xmlns:plus="http://ns.useplus.org/ldf/xmp/1.0/">
<plus:DataMining>
<rdf:Seq>
<rdf:li>claim</rdf:li>
</rdf:Seq>
</plus:DataMining>
</rdf:Description>
</rdf:RDF>
</x:xmpmeta>"#
            .as_bytes()
            .to_vec();
        let result = filter_xmp_packet(&packet).expect("should parse");
        assert!(
            result.is_empty(),
            "all-owned description must be removed: {:?}",
            result
        );
    }

    #[test]
    fn owned_subtree_between_two_unrelated_children_preserves_both_neighbors() {
        let packet = r#"<?xml version="1.0" encoding="UTF-8"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/" xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
<rdf:RDF>
<rdf:Description xmlns:plus="http://ns.useplus.org/ldf/xmp/1.0/"
                xmlns:dc="http://purl.org/dc/elements/1.1/">
<dc:creator>BeforeNeighbor</dc:creator>
<plus:DataMining>
<rdf:Seq>
<rdf:li>secret</rdf:li>
</rdf:Seq>
</plus:DataMining>
<dc:rights>AfterNeighbor</dc:rights>
</rdf:Description>
</rdf:RDF>
</x:xmpmeta>"#
            .as_bytes()
            .to_vec();
        let result = filter_xmp_packet(&packet).expect("should parse");
        let xml = std::str::from_utf8(&result[0].xml).expect("utf8");
        assert!(xml.contains("BeforeNeighbor"));
        assert!(xml.contains("AfterNeighbor"));
        assert!(!xml.contains("secret"));
    }

    #[test]
    fn owned_subtree_with_nested_same_local_wrong_namespace_still_removed_whole() {
        let packet = r#"<?xml version="1.0" encoding="UTF-8"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/" xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
<rdf:RDF>
<rdf:Description xmlns:plus="http://ns.useplus.org/ldf/xmp/1.0/">
<plus:DataMining>
<other:DataMining xmlns:other="http://example.com/other/">nestedWrong</other:DataMining>
</plus:DataMining>
</rdf:Description>
</rdf:RDF>
</x:xmpmeta>"#
            .as_bytes()
            .to_vec();
        let result = filter_xmp_packet(&packet).expect("should parse");
        assert!(
            result.is_empty(),
            "all-owned description must be removed: {:?}",
            result
        );
    }

    #[test]
    fn owned_empty_element_removed_without_affecting_following_sibling() {
        let packet = r#"<?xml version="1.0" encoding="UTF-8"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/" xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
<rdf:RDF>
<rdf:Description xmlns:plus="http://ns.useplus.org/ldf/xmp/1.0/"
                xmlns:dc="http://purl.org/dc/elements/1.1/">
<dc:creator>Before</dc:creator>
<plus:DataMining/>
<dc:rights>After</dc:rights>
</rdf:Description>
</rdf:RDF>
</x:xmpmeta>"#
            .as_bytes()
            .to_vec();
        let result = filter_xmp_packet(&packet).expect("should parse");
        let xml = std::str::from_utf8(&result[0].xml).expect("utf8");
        assert!(xml.contains("Before"));
        assert!(xml.contains("After"));
        assert!(!xml.contains("DataMining"));
    }

    #[test]
    fn owned_nested_depth_returns_to_zero_exactly_once() {
        let packet = r#"<?xml version="1.0" encoding="UTF-8"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/" xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
<rdf:RDF>
<rdf:Description xmlns:plus="http://ns.useplus.org/ldf/xmp/1.0/"
                xmlns:dc="http://purl.org/dc/elements/1.1/">
<plus:DataMining>
<rdf:Alt>
<rdf:li xml:lang="x-default">deep</rdf:li>
</rdf:Alt>
</plus:DataMining>
<dc:creator>AfterDepth</dc:creator>
</rdf:Description>
</rdf:RDF>
</x:xmpmeta>"#
            .as_bytes()
            .to_vec();
        let result = filter_xmp_packet(&packet).expect("should parse");
        let xml = std::str::from_utf8(&result[0].xml).expect("utf8");
        assert!(xml.contains("AfterDepth"));
        assert!(!xml.contains("deep"));
        assert!(!xml.contains("DataMining"));
    }

    #[test]
    fn filter_accepts_xml_references_in_unrelated_text() {
        let packet = build_packet(
            r#"<rdf:Description xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:title>A &amp; B &lt; C &gt; D &apos;Q&apos; &quot;Q&quot; &#169; &#x1F642;</dc:title></rdf:Description>"#,
        );
        let result = filter_xmp_packet(&packet).expect("valid XML references should parse");
        let xml = String::from_utf8(result[0].xml.clone()).expect("UTF-8");
        assert!(
            xml.contains("A &amp; B &lt; C &gt; D 'Q' \"Q\" © 🙂"),
            "{xml}"
        );
    }

    #[test]
    fn merge_accepts_xml_references_and_reparses() {
        let packet = build_packet(
            r#"<rdf:Description xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:title>A &amp; B &#169; &#x1F642;</dc:title></rdf:Description>"#,
        );
        let output = merge_preserved_descriptions(&packet, &[]).expect("references should merge");
        let xml = String::from_utf8(output).expect("UTF-8");
        assert!(xml.contains("A &amp; B © 🙂"), "{xml}");
        let mut reader = NsReader::from_str(&xml);
        let mut buf = Vec::new();
        loop {
            if matches!(
                reader.read_event_into(&mut buf).expect("reparse"),
                Event::Eof
            ) {
                break;
            }
            buf.clear();
        }
    }

    #[test]
    fn unknown_named_entity_is_rejected() {
        let packet = build_packet(
            r#"<rdf:Description xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:title>A &companyName;</dc:title></rdf:Description>"#,
        );
        assert!(filter_xmp_packet(&packet).is_err());
        assert!(merge_preserved_descriptions(&packet, &[]).is_err());
    }

    #[test]
    fn invalid_numeric_reference_is_rejected() {
        for reference in ["&#0;", "&#x0;", "&#x110000;", "&#xZZ;"] {
            let packet = build_packet(&format!(
                r#"<rdf:Description xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:title>A {reference}</dc:title></rdf:Description>"#
            ));
            assert!(
                filter_xmp_packet(&packet).is_err(),
                "filter accepted {reference}"
            );
            assert!(
                merge_preserved_descriptions(&packet, &[]).is_err(),
                "merge accepted {reference}"
            );
        }
    }

    #[test]
    fn merge_attribute_references_are_decoded_before_escaping() {
        let packet = build_packet(
            r#"<rdf:Description xmlns:dc="http://purl.org/dc/elements/1.1/" dc:title="A &amp; B &#169; &#x1F642;"/>"#,
        );
        let output = merge_preserved_descriptions(&packet, &[]).expect("attribute should merge");
        let xml = String::from_utf8(output).expect("UTF-8");
        assert!(xml.contains("dc:title=\"A &amp; B © 🙂\""));
        assert!(!xml.contains("&amp;amp;"));
    }

    #[test]
    fn owned_nested_rdf_description_does_not_close_outer_description() {
        let packet = build_packet(
            r#"<rdf:Description xmlns:plus="http://ns.useplus.org/ldf/xmp/1.0/" xmlns:dc="http://purl.org/dc/elements/1.1/"><plus:OtherConstraints><rdf:Description><rdf:value>owned &amp; nested</rdf:value><!-- owned comment --><?owned test?></rdf:Description></plus:OtherConstraints><dc:title>must survive</dc:title></rdf:Description>"#,
        );
        let result = filter_xmp_packet(&packet).expect("nested owned RDF should parse");
        assert_eq!(result.len(), 1);
        let xml = String::from_utf8(result[0].xml.clone()).expect("UTF-8");
        assert!(xml.contains("must survive"));
        assert!(!xml.contains("owned"));
        assert!(!xml.contains("rdf:value"));
        let mut reader = NsReader::from_str(&xml);
        let mut buf = Vec::new();
        loop {
            if matches!(
                reader.read_event_into(&mut buf).expect("reparse"),
                Event::Eof
            ) {
                break;
            }
            buf.clear();
        }
    }

    #[test]
    fn unrelated_comments_and_processing_instructions_are_preserved() {
        let packet = build_packet(
            r#"<rdf:Description xmlns:plus="http://ns.useplus.org/ldf/xmp/1.0/" xmlns:dc="http://purl.org/dc/elements/1.1/"><plus:OtherConstraints><!-- owned comment --><?owned test?><rdf:Description/></plus:OtherConstraints><!-- unrelated comment --><?unrelated test?><dc:title>survives</dc:title></rdf:Description>"#,
        );
        let result = filter_xmp_packet(&packet).expect("comments and PIs should parse");
        let xml = String::from_utf8(result[0].xml.clone()).expect("UTF-8");
        assert!(xml.contains("unrelated comment"));
        assert!(xml.contains("unrelated test"));
        assert!(!xml.contains("owned comment"));
        assert!(!xml.contains("owned test"));
    }
}
