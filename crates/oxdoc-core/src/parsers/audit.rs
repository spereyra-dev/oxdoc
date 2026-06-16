use std::io::{Cursor, Read, Seek};

use quick_xml::Reader;
use quick_xml::events::Event;

use crate::models::{AuditSignal, DocumentAudit, DocumentType, Extraction, OutputWarning};
use crate::parsers::{
    attr_value, name_eq, parent_dir, parse_relationships, resolve_relationship_target,
};
use crate::vfs::OoxmlPackage;
use crate::{OxdocError, Result};

use super::{detect_document_type, metadata};

pub(crate) fn read_audit<R: Read + Seek>(
    package: &mut OoxmlPackage<R>,
    file_name: String,
) -> Result<Extraction<DocumentAudit>> {
    let document_type = detect_document_type(package)?;
    let info = metadata::read_info(package, file_name.clone())?;
    let mut warnings = info.warnings;
    let mut signals = Vec::new();

    if info.value.has_macros {
        signals.push(AuditSignal::new(
            "macros",
            "high",
            "[Content_Types].xml",
            "VBA macro content is present or declared",
        ));
    }

    if let Some(custom_properties) = &info.value.custom_properties
        && !custom_properties.is_empty()
    {
        signals.push(AuditSignal::new(
            "custom_properties",
            "info",
            "docProps/custom.xml",
            format!(
                "{} custom document properties are present",
                custom_properties.len()
            ),
        ));
    }

    if matches!(document_type, DocumentType::Xlsx)
        && let Some(hidden) = audit_hidden_xlsx_sheets(package)?
    {
        warnings.extend(hidden.warnings);
        signals.extend(hidden.value);
    }

    let relationship_signals = audit_relationship_targets(package)?;
    warnings.extend(relationship_signals.warnings);
    signals.extend(relationship_signals.value);

    signals.extend(warnings.iter().map(warning_signal));

    Ok(Extraction::with_warnings(
        DocumentAudit {
            file: file_name,
            document_type: document_type_name(document_type).to_owned(),
            metadata: info.value,
            signals,
        },
        warnings,
    ))
}

fn audit_hidden_xlsx_sheets<R: Read + Seek>(
    package: &mut OoxmlPackage<R>,
) -> Result<Option<Extraction<Vec<AuditSignal>>>> {
    if !package.contains("xl/workbook.xml") {
        return Ok(None);
    }

    let workbook_xml = package.read_to_string("xl/workbook.xml")?;
    parse_hidden_xlsx_sheets(&workbook_xml, "xl/workbook.xml").map(Some)
}

fn parse_hidden_xlsx_sheets(xml: &str, path: &str) -> Result<Extraction<Vec<AuditSignal>>> {
    let mut reader = Reader::from_reader(Cursor::new(xml.as_bytes()));
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut signals = Vec::new();
    let mut warnings = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(element)) | Ok(Event::Empty(element))
                if name_eq(element.name().as_ref(), b"sheet") =>
            {
                let state = attr_value(&element, b"state").unwrap_or_default();
                if state.eq_ignore_ascii_case("hidden") || state.eq_ignore_ascii_case("veryHidden")
                {
                    let name = attr_value(&element, b"name").unwrap_or_else(|| "<unnamed>".into());
                    signals.push(AuditSignal::new(
                        "hidden_sheet",
                        "warning",
                        path,
                        format!("worksheet '{name}' is {state}"),
                    ));
                }
            }
            Ok(Event::Start(element)) | Ok(Event::Empty(element))
                if name_eq(element.name().as_ref(), b"workbookProtection") =>
            {
                signals.push(AuditSignal::new(
                    "workbook_protection",
                    "warning",
                    path,
                    "workbook protection settings are present",
                ));
            }
            Ok(Event::Eof) => break,
            Err(source) => {
                warnings.push(OutputWarning::malformed_xml(path, source));
                break;
            }
            _ => {}
        }
        buf.clear();
    }

    Ok(Extraction::with_warnings(signals, warnings))
}

fn audit_relationship_targets<R: Read + Seek>(
    package: &mut OoxmlPackage<R>,
) -> Result<Extraction<Vec<AuditSignal>>> {
    let mut signals = Vec::new();
    let mut warnings = Vec::new();
    let mut relationship_paths = package
        .part_names()
        .into_iter()
        .filter(|path| path.ends_with(".rels"))
        .collect::<Vec<_>>();
    relationship_paths.sort();

    for relationship_path in relationship_paths {
        let xml = package.read_to_string(&relationship_path)?;
        let relationships = match parse_relationships(&xml, &relationship_path) {
            Ok(relationships) => relationships,
            Err(OxdocError::MalformedXmlNode { source, .. }) => {
                warnings.push(OutputWarning::malformed_xml(&relationship_path, source));
                continue;
            }
            Err(err) => return Err(err),
        };

        for relationship in relationships {
            match resolve_relationship_target(
                &relationship_base_dir(&relationship_path),
                &relationship,
                &relationship_path,
            ) {
                Ok(_) => {
                    if let Some((kind, message)) = internal_relationship_signal(
                        relationship.relationship_type.as_deref(),
                        &relationship.target,
                    ) {
                        signals.push(AuditSignal::new(
                            kind,
                            "warning",
                            &relationship_path,
                            message,
                        ));
                    }
                }
                Err(err) => match err {
                    OxdocError::SuspiciousRelationshipTarget { target, reason, .. } => {
                        let kind =
                            external_relationship_kind(relationship.relationship_type.as_deref())
                                .unwrap_or("relationship_target");
                        signals.push(AuditSignal::new(
                            kind,
                            "warning",
                            &relationship_path,
                            format!("relationship target '{target}' is suspicious: {reason}"),
                        ));
                    }
                    err => return Err(err),
                },
            }
        }
    }

    Ok(Extraction::with_warnings(signals, warnings))
}

fn external_relationship_kind(relationship_type: Option<&str>) -> Option<&'static str> {
    match relationship_type?.rsplit('/').next()? {
        kind if kind.eq_ignore_ascii_case("hyperlink") => Some("hyperlink"),
        kind if kind.eq_ignore_ascii_case("externalLink") => Some("external_link"),
        kind if kind.eq_ignore_ascii_case("attachedTemplate") => Some("attached_template"),
        _ => None,
    }
}

fn internal_relationship_signal(
    relationship_type: Option<&str>,
    target: &str,
) -> Option<(&'static str, String)> {
    match relationship_type?.rsplit('/').next()? {
        kind if kind.eq_ignore_ascii_case("oleObject") => Some((
            "ole_object",
            format!("internal OLE object relationship targets '{target}'"),
        )),
        kind if kind.eq_ignore_ascii_case("package") => Some((
            "embedded_package",
            format!("internal embedded package relationship targets '{target}'"),
        )),
        _ => None,
    }
}

fn relationship_base_dir(relationship_path: &str) -> String {
    if relationship_path == "_rels/.rels" {
        return String::new();
    }

    if let Some((prefix, file)) = relationship_path.rsplit_once("/_rels/") {
        let source = file.trim_end_matches(".rels");
        if prefix.is_empty() {
            return parent_dir(source).to_owned();
        }
        if source.is_empty() {
            return prefix.to_owned();
        }
        return parent_dir(&format!("{prefix}/{source}")).to_owned();
    }

    parent_dir(relationship_path).to_owned()
}

fn warning_signal(warning: &OutputWarning) -> AuditSignal {
    AuditSignal::new(
        "parser_warning",
        "warning",
        &warning.path,
        format!(
            "warning[{}/{}]: {}",
            warning.category().as_str(),
            warning.code().as_str(),
            warning.message
        ),
    )
}

fn document_type_name(document_type: DocumentType) -> &'static str {
    match document_type {
        DocumentType::Docx => "docx",
        DocumentType::Pptx => "pptx",
        DocumentType::Xlsx => "xlsx",
        DocumentType::Unknown => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::relationship_base_dir;

    #[test]
    fn derives_relationship_base_dirs() {
        assert_eq!(relationship_base_dir("_rels/.rels"), "");
        assert_eq!(
            relationship_base_dir("word/_rels/document.xml.rels"),
            "word"
        );
        assert_eq!(
            relationship_base_dir("ppt/slides/_rels/slide1.xml.rels"),
            "ppt/slides"
        );
    }
}
