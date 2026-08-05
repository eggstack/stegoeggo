//! XMP parsing and namespace-aware field filtering using `quick-xml`.
//!
//! Provides ownership-based XMP field stripping by namespace URI and local name,
//! and namespace conflict detection for XMP packet merging.

use std::collections::HashMap;

use quick_xml::events::Event;
use quick_xml::name::QName;
use quick_xml::Reader;

use crate::error::{Error, Result};

pub(crate) const PLUS_NAMESPACE: &str = "http://ns.useplus.org/ldf/xmp/1.0/";
pub(crate) const STEGOEGGO_NAMESPACE: &str = "https://github.com/eggstack/stegoeggo";

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

fn is_owned_attribute(
    prefix: &str,
    local_name: &str,
    prefix_map: &HashMap<String, String>,
) -> bool {
    let uri = match prefix_map.get(prefix) {
        Some(u) => u.as_str(),
        None => return false,
    };
    OWNED_FIELDS
        .iter()
        .any(|f| f.namespace_uri == uri && f.local_name == local_name)
}

fn is_owned_element(prefix: &str, local_name: &str, prefix_map: &HashMap<String, String>) -> bool {
    let uri = match prefix_map.get(prefix) {
        Some(u) => u.as_str(),
        None => return false,
    };
    OWNED_FIELDS
        .iter()
        .any(|f| f.namespace_uri == uri && f.local_name == local_name)
}

fn split_qname(name: QName<'_>) -> (Option<String>, String) {
    let raw = String::from_utf8_lossy(name.as_ref()).into_owned();
    if let Some(colon_pos) = raw.find(':') {
        let prefix = raw[..colon_pos].to_string();
        let local = raw[colon_pos + 1..].to_string();
        (Some(prefix), local)
    } else {
        (None, raw)
    }
}

pub(crate) fn build_prefix_map(xmp_data: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let mut search_from = 0;
    while let Some(pos) = xmp_data[search_from..].find("xmlns:") {
        let abs = search_from + pos;
        let after_prefix = &xmp_data[abs + 6..];
        if let Some(eq_pos) = after_prefix.find('=') {
            let prefix = after_prefix[..eq_pos].to_string();
            let rest = &after_prefix[eq_pos + 1..];
            if let Some(stripped) = rest.strip_prefix('"') {
                if let Some(end) = stripped.find('"') {
                    map.insert(prefix, stripped[..end].to_string());
                    search_from = abs + 6 + eq_pos + 1 + end + 1;
                    continue;
                }
            } else if let Some(stripped) = rest.strip_prefix('\'') {
                if let Some(end) = stripped.find('\'') {
                    map.insert(prefix, stripped[..end].to_string());
                    search_from = abs + 6 + eq_pos + 1 + end + 1;
                    continue;
                }
            }
        }
        search_from = abs + 6;
    }
    map
}

pub(crate) fn strip_owned_fields_from_description(
    desc_raw: &str,
    prefix_map: &HashMap<String, String>,
) -> Option<String> {
    let mut reader = Reader::from_str(desc_raw);
    let mut buf = Vec::new();
    let mut result = String::new();
    let mut has_unrelated = false;
    let mut open_tags: Vec<String> = Vec::new();

    let mut desc_start = None;
    loop {
        buf.clear();
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let (prefix, local) = split_qname(e.name());
                let p = prefix.as_deref().unwrap_or("");
                if desc_start.is_none() && local == "rdf:Description" {
                    desc_start = Some(0);
                    let name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                    result.push('<');
                    result.push_str(&name);
                    for attr in e.attributes().flatten() {
                        let attr_key = String::from_utf8_lossy(attr.key.as_ref());
                        let (attr_prefix, attr_local) = if let Some(colon) = attr_key.find(':') {
                            (
                                Some(attr_key[..colon].to_string()),
                                attr_key[colon + 1..].to_string(),
                            )
                        } else {
                            (None, attr_key.to_string())
                        };
                        let ap = attr_prefix.as_deref().unwrap_or("");
                        if !is_owned_attribute(ap, &attr_local, prefix_map) {
                            let val = String::from_utf8_lossy(&attr.value);
                            result.push_str(&format!(" {}=\"{}\"", attr_key, val));
                        }
                    }
                    result.push('>');
                } else if desc_start.is_some() {
                    if is_owned_element(p, &local, prefix_map) {
                        open_tags.push(local);
                    } else {
                        has_unrelated = true;
                        let name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                        result.push('<');
                        result.push_str(&name);
                        for attr in e.attributes().flatten() {
                            let key = String::from_utf8_lossy(attr.key.as_ref());
                            let val = String::from_utf8_lossy(&attr.value);
                            result.push_str(&format!(" {}=\"{}\"", key, val));
                        }
                        result.push('>');
                    }
                }
            }
            Ok(Event::Text(ref t)) => {
                if desc_start.is_some() && open_tags.is_empty() {
                    let text = String::from_utf8_lossy(t.as_ref());
                    result.push_str(&text);
                }
            }
            Ok(Event::End(_)) => {
                if desc_start.is_some() {
                    if !open_tags.is_empty() {
                        open_tags.pop();
                        has_unrelated = true;
                    } else {
                        break;
                    }
                }
            }
            Ok(Event::Empty(ref e)) => {
                if desc_start.is_some() {
                    let (p, local) = split_qname(e.name());
                    let pp = p.as_deref().unwrap_or("");
                    if !is_owned_element(pp, &local, prefix_map) {
                        has_unrelated = true;
                        let name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                        result.push('<');
                        result.push_str(&name);
                        for attr in e.attributes().flatten() {
                            let key = String::from_utf8_lossy(attr.key.as_ref());
                            let val = String::from_utf8_lossy(&attr.value);
                            result.push_str(&format!(" {}=\"{}\"", key, val));
                        }
                        result.push_str("/>");
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => return None,
            _ => {}
        }
    }

    result.push_str("</rdf:Description>");

    if !has_unrelated && !result.contains("plus:DataMining") && !result.contains("stegoeggo:") {
        return Some(desc_raw.to_string());
    }
    if result.contains("plus:DataMining") || result.contains("stegoeggo:") {
        return None;
    }
    if has_unrelated {
        Some(result)
    } else {
        None
    }
}

/// Check for namespace prefix conflicts between existing and new XMP metadata.
///
/// Returns `Err` if the same prefix maps to different namespace URIs in the two sections.
pub fn check_namespace_conflict(existing_xmp: &str, new_xmp: &str) -> Result<()> {
    let existing_map = build_prefix_map(existing_xmp);
    let new_map = build_prefix_map(new_xmp);

    for (prefix, new_uri) in &new_map {
        if let Some(existing_uri) = existing_map.get(prefix) {
            if existing_uri != new_uri {
                return Err(Error::Metadata(format!(
                    "XMP namespace conflict: prefix '{}' maps to '{}' in existing metadata but '{}' in new metadata",
                    prefix, existing_uri, new_uri
                )));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_prefix_map_extracts_namespaces() {
        let xmp = r#"<?xml version="1.0"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/" xmlns:plus="http://ns.useplus.org/ldf/xmp/1.0/">
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
</rdf:RDF>
</x:xmpmeta>"#;
        let map = build_prefix_map(xmp);
        assert_eq!(
            map.get("plus").unwrap(),
            "http://ns.useplus.org/ldf/xmp/1.0/"
        );
        assert_eq!(map.get("x").unwrap(), "adobe:ns:meta/");
    }

    #[test]
    fn owned_field_matches_by_uri() {
        let mut map = HashMap::new();
        map.insert("plus".to_string(), PLUS_NAMESPACE.to_string());
        assert!(is_owned_attribute("plus", "DataMining", &map));
        assert!(!is_owned_attribute("plus", "License", &map));
    }

    #[test]
    fn unrelated_plus_field_not_owned() {
        let mut map = HashMap::new();
        map.insert("plus".to_string(), PLUS_NAMESPACE.to_string());
        assert!(!is_owned_attribute("plus", "License", &map));
        assert!(!is_owned_attribute("plus", "SomeOther", &map));
    }

    #[test]
    fn check_namespace_conflict_detects_mismatch() {
        let existing = r#"xmlns:dc="http://purl.org/dc/elements/1.1/""#;
        let conflicting = r#"xmlns:dc="http://example.com/different""#;
        assert!(check_namespace_conflict(existing, conflicting).is_err());
    }

    #[test]
    fn check_namespace_conflict_allows_compatible() {
        let existing = r#"xmlns:dc="http://purl.org/dc/elements/1.1/""#;
        let compatible = r#"xmlns:dc="http://purl.org/dc/elements/1.1/""#;
        assert!(check_namespace_conflict(existing, compatible).is_ok());
    }
}
