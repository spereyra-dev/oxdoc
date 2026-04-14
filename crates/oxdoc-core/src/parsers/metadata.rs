use std::io::{BufRead, Cursor, Read, Seek};

use quick_xml::Reader;
use quick_xml::events::Event;

use crate::Result;
use crate::models::{DocumentInfo, Extraction, OutputWarning};
use crate::parsers::{decode_xml_reference, decode_xml_text, name_eq};
use crate::vfs::OoxmlPackage;

const MACRO_PARTS: &[&str] = &[
    "word/vbaProject.bin",
    "xl/vbaProject.bin",
    "ppt/vbaProject.bin",
];

pub(crate) fn read_info<R: Read + Seek>(
    package: &mut OoxmlPackage<R>,
    file_name: String,
) -> Result<Extraction<DocumentInfo>> {
    let mut info = DocumentInfo {
        file: file_name,
        has_macros: package.contains_any(MACRO_PARTS),
        ..DocumentInfo::default()
    };
    let mut warnings = Vec::new();

    if package.contains("docProps/core.xml") {
        let core_xml = package.read_to_string("docProps/core.xml")?;
        let result = parse_core(Cursor::new(core_xml.as_bytes()), "docProps/core.xml")?;
        apply_core(&mut info, result.value);
        warnings.extend(result.warnings);
    }

    if package.contains("docProps/app.xml") {
        let app_xml = package.read_to_string("docProps/app.xml")?;
        let result = parse_app(Cursor::new(app_xml.as_bytes()), "docProps/app.xml")?;
        apply_app(&mut info, result.value);
        warnings.extend(result.warnings);
    }

    Ok(Extraction::with_warnings(info, warnings))
}

#[derive(Debug, Default)]
struct CoreProps {
    author: Option<String>,
    last_modified_by: Option<String>,
    created_at: Option<String>,
    modified_at: Option<String>,
    revision: Option<String>,
}

#[derive(Debug, Default)]
struct AppProps {
    application: Option<String>,
    company: Option<String>,
    word_count: Option<u64>,
    page_count: Option<u64>,
    slide_count: Option<u64>,
    worksheet_count: Option<u64>,
}

fn parse_core<R: BufRead>(source: R, path: &str) -> Result<Extraction<CoreProps>> {
    let mut reader = Reader::from_reader(source);
    reader.config_mut().trim_text(false);
    let mut buf = Vec::new();
    let mut props = CoreProps::default();
    let mut warnings = Vec::new();
    let mut current_field: Option<Vec<u8>> = None;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(element)) => {
                let local = element.name().as_ref().to_vec();
                if matches!(
                    crate::parsers::local_name(&local),
                    b"creator" | b"lastModifiedBy" | b"created" | b"modified" | b"revision"
                ) {
                    current_field = Some(local);
                }
            }
            Ok(Event::Text(value)) => {
                if let Some(field) = &current_field {
                    let value = decode_xml_text(value.as_ref());
                    assign_core_value(&mut props, field, value);
                }
            }
            Ok(Event::GeneralRef(value)) => {
                if let Some(field) = &current_field {
                    assign_core_value(&mut props, field, decode_xml_reference(value.as_ref()));
                }
            }
            Ok(Event::End(element)) => {
                if current_field.as_deref().is_some_and(|field| {
                    name_eq(element.name().as_ref(), crate::parsers::local_name(field))
                }) {
                    current_field = None;
                }
            }
            Ok(Event::Eof) => break,
            Err(source) => {
                warnings.push(OutputWarning::new(
                    path,
                    format!("stopped after malformed XML: {source}"),
                ));
                break;
            }
            _ => {}
        }
        buf.clear();
    }

    Ok(Extraction::with_warnings(props, warnings))
}

fn parse_app<R: BufRead>(source: R, path: &str) -> Result<Extraction<AppProps>> {
    let mut reader = Reader::from_reader(source);
    reader.config_mut().trim_text(false);
    let mut buf = Vec::new();
    let mut props = AppProps::default();
    let mut warnings = Vec::new();
    let mut current_field: Option<Vec<u8>> = None;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(element)) => {
                let local = element.name().as_ref().to_vec();
                if matches!(
                    crate::parsers::local_name(&local),
                    b"Application" | b"Company" | b"Words" | b"Pages" | b"Slides" | b"Worksheets"
                ) {
                    current_field = Some(local);
                }
            }
            Ok(Event::Text(value)) => {
                if let Some(field) = &current_field {
                    let value = decode_xml_text(value.as_ref());
                    assign_app_value(&mut props, field, value);
                }
            }
            Ok(Event::GeneralRef(value)) => {
                if let Some(field) = &current_field {
                    assign_app_value(&mut props, field, decode_xml_reference(value.as_ref()));
                }
            }
            Ok(Event::End(element)) => {
                if current_field.as_deref().is_some_and(|field| {
                    name_eq(element.name().as_ref(), crate::parsers::local_name(field))
                }) {
                    current_field = None;
                }
            }
            Ok(Event::Eof) => break,
            Err(source) => {
                warnings.push(OutputWarning::new(
                    path,
                    format!("stopped after malformed XML: {source}"),
                ));
                break;
            }
            _ => {}
        }
        buf.clear();
    }

    Ok(Extraction::with_warnings(props, warnings))
}

fn apply_core(info: &mut DocumentInfo, props: CoreProps) {
    info.author = props.author;
    info.last_modified_by = props.last_modified_by;
    info.created_at = props.created_at;
    info.modified_at = props.modified_at;
    info.revision = props.revision;
}

fn assign_core_value(props: &mut CoreProps, field: &[u8], value: String) {
    match crate::parsers::local_name(field) {
        b"creator" => props.author = Some(append_or_new(props.author.take(), value)),
        b"lastModifiedBy" => {
            props.last_modified_by = Some(append_or_new(props.last_modified_by.take(), value));
        }
        b"created" => props.created_at = Some(append_or_new(props.created_at.take(), value)),
        b"modified" => props.modified_at = Some(append_or_new(props.modified_at.take(), value)),
        b"revision" => props.revision = Some(append_or_new(props.revision.take(), value)),
        _ => {}
    }
}

fn assign_app_value(props: &mut AppProps, field: &[u8], value: String) {
    match crate::parsers::local_name(field) {
        b"Application" => props.application = Some(append_or_new(props.application.take(), value)),
        b"Company" => props.company = Some(append_or_new(props.company.take(), value)),
        b"Words" => props.word_count = value.parse().ok(),
        b"Pages" => props.page_count = value.parse().ok(),
        b"Slides" => props.slide_count = value.parse().ok(),
        b"Worksheets" => props.worksheet_count = value.parse().ok(),
        _ => {}
    }
}

fn append_or_new(existing: Option<String>, value: String) -> String {
    match existing {
        Some(mut existing) => {
            existing.push_str(&value);
            existing
        }
        None => value,
    }
}

fn apply_app(info: &mut DocumentInfo, props: AppProps) {
    info.application = props.application;
    info.company = props.company;
    info.word_count = props.word_count;
    info.page_count = props.page_count;
    info.slide_count = props.slide_count;
    info.worksheet_count = props.worksheet_count;
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::{parse_app, parse_core};

    #[test]
    fn parses_core_properties() {
        let xml = r#"
            <cp:coreProperties xmlns:cp="cp" xmlns:dc="dc" xmlns:dcterms="dcterms">
              <dc:creator>Ada</dc:creator>
              <cp:lastModifiedBy>Linus</cp:lastModifiedBy>
              <dcterms:created>2024-03-12T10:00:00Z</dcterms:created>
              <cp:revision>7</cp:revision>
            </cp:coreProperties>
        "#;

        let props = parse_core(Cursor::new(xml.as_bytes()), "docProps/core.xml")
            .unwrap()
            .value;

        assert_eq!(props.author.as_deref(), Some("Ada"));
        assert_eq!(props.last_modified_by.as_deref(), Some("Linus"));
        assert_eq!(props.created_at.as_deref(), Some("2024-03-12T10:00:00Z"));
        assert_eq!(props.revision.as_deref(), Some("7"));
    }

    #[test]
    fn parses_app_properties() {
        let xml = r#"
            <Properties>
              <Application>LibreOffice</Application>
              <Words>1542</Words>
              <Pages>3</Pages>
              <Worksheets>2</Worksheets>
            </Properties>
        "#;

        let props = parse_app(Cursor::new(xml.as_bytes()), "docProps/app.xml")
            .unwrap()
            .value;

        assert_eq!(props.application.as_deref(), Some("LibreOffice"));
        assert_eq!(props.word_count, Some(1542));
        assert_eq!(props.page_count, Some(3));
        assert_eq!(props.worksheet_count, Some(2));
    }
}
