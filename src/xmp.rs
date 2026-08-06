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
    reader.config_mut().trim_text(true);
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
                    append_description_open(&mut out, &start, &ns_stack, &reader)?;
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
                    let owned = start_element_is_owned(start.name().as_ref(), &ns_stack);
                    if owned {
                        owned_depth += 1;
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
                    append_description_open(&mut out, &empty, &ns_stack, &reader)?;
                    if has_unrelated_attr {
                        out.extend_from_slice(b"/>");
                        descriptions.push(PreservedDescription {
                            xml: out,
                            has_unrelated: true,
                        });
                    }
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
                let local = end.local_name();
                let is_rdf_desc = local_name_eq(local.as_ref(), "Description");

                if is_rdf_desc {
                    let mut out = current_out
                        .take()
                        .ok_or_else(|| xmp_internal_error("writer missing"))?;
                    out.extend_from_slice(b"</Description>");
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
                if let Some(out) = current_out.as_mut() {
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
                if let Some(out) = current_out.as_mut() {
                    out.push(b'<');
                    out.push(b'?');
                    out.extend_from_slice(pi.as_ref());
                    out.extend_from_slice(b"?>");
                }
            }
            Event::GeneralRef(_) => {
                return Err(Error::Metadata(
                    "Entity references are not supported in XMP".to_string(),
                ));
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
) -> Result<()> {
    out.push(b'<');
    out.extend_from_slice(b"Description");

    for attr_res in start.attributes() {
        let attr = attr_res.map_err(xmp_attr_error)?;
        let key = attr.key.as_ref();
        if !is_xmlns_attr(key) && !is_xml_lang_attr(key) && attribute_is_owned(&attr, ns_stack) {
            continue;
        }
        let value = attr_raw_value(&attr, reader)?;
        append_attr(out, key, &value);
    }

    let rdf_uri = RDF_NAMESPACE.as_bytes();
    let has_rdf = start.attributes().any(|a| {
        a.as_ref()
            .is_ok_and(|attr| attr.key.as_ref() == b"xmlns:rdf" && attr.value.as_ref() == rdf_uri)
    });
    if !has_rdf {
        append_attr(out, b"xmlns:rdf", rdf_uri);
    }

    for (prefix, uri) in ns_stack.snapshot() {
        if prefix.is_empty() {
            continue;
        }
        let mut attr_name = Vec::with_capacity(prefix.len() + 6);
        attr_name.extend_from_slice(b"xmlns:");
        attr_name.extend_from_slice(&prefix);
        let already = start.attributes().any(|a| {
            a.as_ref().is_ok_and(|attr| {
                attr.key.as_ref() == attr_name.as_slice() && attr.value.as_ref() == uri.as_slice()
            })
        });
        if already {
            continue;
        }
        append_attr(out, &attr_name, &uri);
    }

    out.push(b'>');
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

/// Check for namespace prefix conflicts between existing and new XMP metadata.
pub(crate) fn check_namespace_conflict(existing_xmp: &[u8], new_xmp: &[u8]) -> Result<()> {
    let existing_descs = filter_xmp_packet(existing_xmp)?;
    let new_descs = filter_xmp_packet(new_xmp)?;

    let existing_map = merge_prefix_maps(&existing_descs);
    let new_map = merge_prefix_maps(&new_descs);

    for (prefix, new_uri) in &new_map {
        if let Some(existing_uri) = existing_map.get(prefix) {
            if existing_uri != new_uri {
                return Err(Error::Metadata(format!(
                    "XMP namespace conflict: prefix '{}' maps to '{}' in existing metadata but '{}' in new metadata",
                    String::from_utf8_lossy(prefix),
                    String::from_utf8_lossy(existing_uri),
                    String::from_utf8_lossy(new_uri)
                )));
            }
        }
    }

    let existing_raw_map = extract_prefix_declarations(existing_xmp);
    let new_raw_map = extract_prefix_declarations(new_xmp);

    for (prefix, new_uri) in &new_raw_map {
        if let Some(existing_uri) = existing_raw_map.get(prefix) {
            if existing_uri != new_uri {
                return Err(Error::Metadata(format!(
                    "XMP namespace conflict: prefix '{}' maps to '{}' in existing metadata but '{}' in new metadata",
                    String::from_utf8_lossy(prefix),
                    String::from_utf8_lossy(existing_uri),
                    String::from_utf8_lossy(new_uri)
                )));
            }
        }
    }
    Ok(())
}

fn extract_prefix_declarations(packet: &[u8]) -> std::collections::BTreeMap<Vec<u8>, Vec<u8>> {
    let mut map = std::collections::BTreeMap::new();
    let packet_str = match std::str::from_utf8(packet) {
        Ok(s) => s,
        Err(_) => return map,
    };
    let mut reader = NsReader::from_str(packet_str);
    let mut buf = Vec::new();
    while let Ok(event) = reader.read_event_into(&mut buf) {
        match event {
            Event::Start(start) => {
                for attr_res in start.attributes() {
                    let Ok(attr) = attr_res else { continue };
                    let key = attr.key.as_ref();
                    if !key.starts_with(b"xmlns:") {
                        continue;
                    }
                    let prefix = key[6..].to_vec();
                    if prefix.is_empty() {
                        continue;
                    }
                    let value = match attr.decoded_and_normalized_value(
                        quick_xml::XmlVersion::Implicit1_0,
                        reader.decoder(),
                    ) {
                        Ok(v) => v.into_owned().into_bytes(),
                        Err(_) => continue,
                    };
                    map.entry(prefix).or_insert(value);
                }
            }
            Event::Empty(start) => {
                for attr_res in start.attributes() {
                    let Ok(attr) = attr_res else { continue };
                    let key = attr.key.as_ref();
                    if !key.starts_with(b"xmlns:") {
                        continue;
                    }
                    let prefix = key[6..].to_vec();
                    if prefix.is_empty() {
                        continue;
                    }
                    let value = match attr.decoded_and_normalized_value(
                        quick_xml::XmlVersion::Implicit1_0,
                        reader.decoder(),
                    ) {
                        Ok(v) => v.into_owned().into_bytes(),
                        Err(_) => continue,
                    };
                    map.entry(prefix).or_insert(value);
                }
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    map
}

fn merge_prefix_maps(
    descs: &[PreservedDescription],
) -> std::collections::BTreeMap<Vec<u8>, Vec<u8>> {
    let mut out = std::collections::BTreeMap::new();
    for desc in descs {
        let xml_str = std::str::from_utf8(&desc.xml).unwrap_or("");
        let reader = NsReader::from_str(xml_str);
        let bindings: Vec<_> = reader.resolver().bindings().collect();
        for (prefix_decl, ns) in bindings {
            if let quick_xml::name::PrefixDeclaration::Named(prefix) = prefix_decl {
                if prefix.is_empty() {
                    continue;
                }
                out.entry(prefix.to_vec())
                    .or_insert_with(|| ns.as_ref().to_vec());
            }
        }
    }
    out
}

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
}
