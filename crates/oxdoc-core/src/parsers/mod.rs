pub(crate) mod docx;
pub(crate) mod metadata;
pub(crate) mod pptx;
pub(crate) mod xlsx;

use std::collections::HashMap;
use std::io::Cursor;

use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};

use crate::models::OutputWarning;
use crate::vfs::OoxmlPackage;
use crate::{OxdocError, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Relationship {
    pub id: Option<String>,
    pub target: String,
    pub relationship_type: Option<String>,
    pub target_mode: Option<String>,
}

pub(crate) fn find_office_document_path<R: std::io::Read + std::io::Seek>(
    package: &mut OoxmlPackage<R>,
    fallback: &str,
) -> Result<String> {
    match package.read_to_string("_rels/.rels") {
        Ok(xml) => {
            for relationship in parse_relationships(&xml, "_rels/.rels")? {
                if relationship
                    .relationship_type
                    .as_deref()
                    .is_some_and(|kind| kind.ends_with("/officeDocument"))
                {
                    return resolve_relationship_target("", &relationship, "_rels/.rels");
                }
            }
            Ok(fallback.to_owned())
        }
        Err(OxdocError::MissingPart(_)) => Ok(fallback.to_owned()),
        Err(err) => Err(err),
    }
}

pub(crate) fn parse_relationship_map(
    xml: &str,
    path: &str,
) -> Result<HashMap<String, Relationship>> {
    let mut map = HashMap::new();
    for relationship in parse_relationships(xml, path)? {
        if let Some(id) = relationship.id.clone() {
            map.insert(id, relationship);
        }
    }
    Ok(map)
}

#[doc(hidden)]
pub fn fuzz_parse_relationships(xml: &[u8]) -> Result<()> {
    let xml = String::from_utf8_lossy(xml);
    let _ = parse_relationship_map(&xml, "_rels/.rels")?;
    Ok(())
}

fn parse_relationships(xml: &str, path: &str) -> Result<Vec<Relationship>> {
    let mut reader = Reader::from_reader(Cursor::new(xml.as_bytes()));
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut relationships = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(element)) | Ok(Event::Empty(element)) => {
                if name_eq(element.name().as_ref(), b"Relationship")
                    && let Some(target) = attr_value(&element, b"Target")
                {
                    relationships.push(Relationship {
                        id: attr_value(&element, b"Id"),
                        target,
                        relationship_type: attr_value(&element, b"Type"),
                        target_mode: attr_value(&element, b"TargetMode"),
                    });
                }
            }
            Ok(Event::Eof) => break,
            Err(source) => {
                return Err(OxdocError::MalformedXmlNode {
                    path: path.to_owned(),
                    source,
                });
            }
            _ => {}
        }
        buf.clear();
    }

    Ok(relationships)
}

pub(crate) fn resolve_relationship_target(
    base_dir: &str,
    relationship: &Relationship,
    relationship_path: &str,
) -> Result<String> {
    if relationship
        .target_mode
        .as_deref()
        .is_some_and(|mode| mode.eq_ignore_ascii_case("External"))
    {
        return Err(suspicious_relationship_target(
            relationship_path,
            &relationship.target,
            "external relationship targets are not supported",
        ));
    }

    resolve_part_path(base_dir, &relationship.target, relationship_path)
}

pub(crate) fn resolve_part_path(
    base_dir: &str,
    target: &str,
    relationship_path: &str,
) -> Result<String> {
    let target = target.trim();
    if target.is_empty() {
        return Err(suspicious_relationship_target(
            relationship_path,
            target,
            "target is empty",
        ));
    }

    if target.contains('\0') {
        return Err(suspicious_relationship_target(
            relationship_path,
            target,
            "target contains a NUL byte",
        ));
    }

    if target.contains('\\') {
        return Err(suspicious_relationship_target(
            relationship_path,
            target,
            "backslashes are not valid OOXML part separators",
        ));
    }

    if target.starts_with("//") || has_uri_scheme(target) {
        return Err(suspicious_relationship_target(
            relationship_path,
            target,
            "target is external or absolute outside the package",
        ));
    }

    let mut parts = Vec::new();
    if !target.starts_with('/') {
        parts.extend(base_dir.split('/').filter(|segment| !segment.is_empty()));
    }

    for segment in target.trim_start_matches('/').split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                if parts.pop().is_none() {
                    return Err(suspicious_relationship_target(
                        relationship_path,
                        target,
                        "target escapes the OOXML package root",
                    ));
                }
            }
            other => parts.push(other),
        }
    }

    if parts.is_empty() {
        return Err(suspicious_relationship_target(
            relationship_path,
            target,
            "target does not resolve to an OOXML part",
        ));
    }

    Ok(parts.join("/"))
}

fn suspicious_relationship_target(path: &str, target: &str, reason: &str) -> OxdocError {
    OxdocError::SuspiciousRelationshipTarget {
        path: path.to_owned(),
        target: target.to_owned(),
        reason: reason.to_owned(),
    }
}

fn has_uri_scheme(target: &str) -> bool {
    let Some((scheme, _)) = target.split_once(':') else {
        return false;
    };
    let mut bytes = scheme.bytes();
    bytes.next().is_some_and(|byte| byte.is_ascii_alphabetic())
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'-' | b'.'))
}

pub(crate) fn parent_dir(path: &str) -> &str {
    path.rsplit_once('/').map_or("", |(parent, _)| parent)
}

pub(crate) fn rels_path_for(part_path: &str) -> String {
    match part_path.rsplit_once('/') {
        Some((dir, file)) => format!("{dir}/_rels/{file}.rels"),
        None => format!("_rels/{part_path}.rels"),
    }
}

pub(crate) fn name_eq(name: &[u8], expected_local: &[u8]) -> bool {
    local_name(name) == expected_local
}

pub(crate) fn local_name(name: &[u8]) -> &[u8] {
    name.rsplit(|byte| *byte == b':').next().unwrap_or(name)
}

pub(crate) fn attr_value(element: &BytesStart<'_>, expected_local: &[u8]) -> Option<String> {
    element
        .attributes()
        .with_checks(false)
        .flatten()
        .find(|attr| local_name(attr.key.as_ref()) == expected_local)
        .map(|attr| decode_xml_text(attr.value.as_ref()))
}

pub(crate) fn decode_xml_text(bytes: &[u8]) -> String {
    let raw = String::from_utf8_lossy(bytes);
    if !raw.contains('&') {
        return raw.into_owned();
    }

    let mut decoded = String::with_capacity(raw.len());
    let mut chars = raw.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != '&' {
            decoded.push(ch);
            continue;
        }

        let mut entity = String::new();
        while let Some(&next) = chars.peek() {
            chars.next();
            if next == ';' {
                break;
            }
            entity.push(next);
            if entity.len() > 16 {
                break;
            }
        }

        match entity.as_str() {
            "amp" => decoded.push('&'),
            "lt" => decoded.push('<'),
            "gt" => decoded.push('>'),
            "quot" => decoded.push('"'),
            "apos" => decoded.push('\''),
            _ if entity.starts_with("#x") => {
                if let Ok(value) = u32::from_str_radix(&entity[2..], 16)
                    && let Some(decoded_char) = char::from_u32(value)
                {
                    decoded.push(decoded_char);
                }
            }
            _ if entity.starts_with('#') => {
                if let Ok(value) = entity[1..].parse::<u32>()
                    && let Some(decoded_char) = char::from_u32(value)
                {
                    decoded.push(decoded_char);
                }
            }
            _ => {
                decoded.push('&');
                decoded.push_str(&entity);
                decoded.push(';');
            }
        }
    }

    decoded
}

pub(crate) fn decode_xml_reference(bytes: &[u8]) -> String {
    match bytes {
        b"amp" => "&".to_owned(),
        b"lt" => "<".to_owned(),
        b"gt" => ">".to_owned(),
        b"quot" => "\"".to_owned(),
        b"apos" => "'".to_owned(),
        _ => {
            let entity = String::from_utf8_lossy(bytes);
            if let Some(hex) = entity.strip_prefix("#x")
                && let Ok(value) = u32::from_str_radix(hex, 16)
                && let Some(decoded_char) = char::from_u32(value)
            {
                return decoded_char.to_string();
            }
            if let Some(decimal) = entity.strip_prefix('#')
                && let Ok(value) = decimal.parse::<u32>()
                && let Some(decoded_char) = char::from_u32(value)
            {
                return decoded_char.to_string();
            }
            format!("&{entity};")
        }
    }
}

pub(crate) fn merge_warnings(
    mut left: Vec<OutputWarning>,
    right: Vec<OutputWarning>,
) -> Vec<OutputWarning> {
    left.extend(right);
    left
}

#[cfg(test)]
mod tests {
    use super::{
        decode_xml_reference, decode_xml_text, merge_warnings, parent_dir, parse_relationship_map,
        rels_path_for, resolve_part_path,
    };
    use crate::OxdocError;
    use crate::models::OutputWarning;

    #[test]
    fn decodes_text_and_general_references() {
        assert_eq!(decode_xml_text(b"plain"), "plain");
        assert_eq!(
            decode_xml_text(b"&amp;&lt;&gt;&quot;&apos;&#65;&#x42;&unknown;"),
            "&<>\"'AB&unknown;"
        );
        assert_eq!(decode_xml_reference(b"amp"), "&");
        assert_eq!(decode_xml_reference(b"#65"), "A");
        assert_eq!(decode_xml_reference(b"#x42"), "B");
        assert_eq!(decode_xml_reference(b"custom"), "&custom;");
    }

    #[test]
    fn normalizes_part_and_relationship_paths() {
        assert_eq!(
            resolve_part_path("xl", "worksheets/sheet1.xml", "xl/_rels/workbook.xml.rels").unwrap(),
            "xl/worksheets/sheet1.xml"
        );
        assert_eq!(
            resolve_part_path(
                "xl/worksheets",
                "../sharedStrings.xml",
                "xl/worksheets/_rels/sheet1.xml.rels"
            )
            .unwrap(),
            "xl/sharedStrings.xml"
        );
        assert_eq!(
            resolve_part_path("", "/word/document.xml", "_rels/.rels").unwrap(),
            "word/document.xml"
        );
        assert_eq!(parent_dir("xl/workbook.xml"), "xl");
        assert_eq!(parent_dir("workbook.xml"), "");
        assert_eq!(
            rels_path_for("xl/workbook.xml"),
            "xl/_rels/workbook.xml.rels"
        );
        assert_eq!(rels_path_for("workbook.xml"), "_rels/workbook.xml.rels");

        let err =
            resolve_part_path("xl", "../../outside.xml", "xl/_rels/workbook.xml.rels").unwrap_err();
        assert!(matches!(
            err,
            OxdocError::SuspiciousRelationshipTarget { .. }
        ));

        for target in [
            "",
            "word\0document.xml",
            "word\\document.xml",
            "https://example.invalid/document.xml",
            "/",
        ] {
            let err =
                resolve_part_path("word", target, "word/_rels/document.xml.rels").unwrap_err();
            assert!(matches!(
                err,
                OxdocError::SuspiciousRelationshipTarget { .. }
            ));
        }
    }

    #[test]
    fn parses_relationship_maps_and_errors_on_malformed_xml() {
        let xml = r#"
            <Relationships>
              <Relationship Id="rId1" Type="officeDocument" Target="word/document.xml"/>
              <Relationship Type="ignored-without-id" Target="ignored.xml"/>
              <Relationship Id="rId2" Type="empty-without-target"/>
            </Relationships>
        "#;

        let map = parse_relationship_map(xml, "_rels/.rels").unwrap();

        assert_eq!(
            map.get("rId1")
                .map(|relationship| relationship.target.as_str()),
            Some("word/document.xml")
        );
        assert_eq!(map.len(), 1);

        let err = parse_relationship_map("<Relationships><", "_rels/.rels").unwrap_err();
        assert!(matches!(err, OxdocError::MalformedXmlNode { .. }));
    }

    #[test]
    fn merges_warnings_in_order() {
        let warnings = merge_warnings(
            vec![OutputWarning::new("a.xml", "first")],
            vec![OutputWarning::new("b.xml", "second")],
        );

        assert_eq!(warnings[0].path, "a.xml");
        assert_eq!(warnings[1].message, "second");
    }
}
