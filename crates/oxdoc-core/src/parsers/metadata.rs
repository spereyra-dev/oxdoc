use std::collections::BTreeMap;
use std::io::{BufRead, Cursor, Read, Seek};

use quick_xml::Reader;
use quick_xml::events::Event;

use crate::Result;
use crate::models::{DocumentInfo, Extraction, OutputWarning};
use crate::parsers::{
    append_decoded_xml_reference, append_decoded_xml_text, attr_value, local_name, name_eq,
};
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

    if package.contains("[Content_Types].xml") {
        let content_types_xml = package.read_to_string("[Content_Types].xml")?;
        let result = parse_content_types(
            Cursor::new(content_types_xml.as_bytes()),
            "[Content_Types].xml",
        )?;
        info.has_macros = info.has_macros || result.value;
        warnings.extend(result.warnings);
    }

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

    if package.contains("docProps/custom.xml") {
        let custom_xml = package.read_to_string("docProps/custom.xml")?;
        let result = parse_custom(Cursor::new(custom_xml.as_bytes()), "docProps/custom.xml")?;
        apply_custom(&mut info, result.value);
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
    title: Option<String>,
    subject: Option<String>,
    description: Option<String>,
    keywords: Option<String>,
    category: Option<String>,
    content_status: Option<String>,
    last_printed: Option<String>,
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

#[derive(Debug, Default)]
struct CustomProps {
    values: BTreeMap<String, String>,
}

fn parse_content_types<R: BufRead>(source: R, path: &str) -> Result<Extraction<bool>> {
    let mut reader = Reader::from_reader(source);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut has_macros = false;
    let mut warnings = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(element)) | Ok(Event::Empty(element))
                if matches!(
                    crate::parsers::local_name(element.name().as_ref()),
                    "Default" | "Override"
                ) && attr_value(&element, "ContentType")
                    .as_deref()
                    .is_some_and(is_vba_project_content_type) =>
            {
                has_macros = true;
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

    Ok(Extraction::with_warnings(has_macros, warnings))
}

fn parse_core<R: BufRead>(source: R, path: &str) -> Result<Extraction<CoreProps>> {
    let mut reader = Reader::from_reader(source);
    reader.config_mut().trim_text(false);
    let mut buf = Vec::new();
    let mut props = CoreProps::default();
    let mut warnings = Vec::new();
    let mut current_field: Option<String> = None;
    let mut decoded = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(element)) => {
                let local = element.name().as_ref().to_owned();
                if matches!(
                    crate::parsers::local_name(&local),
                    "creator"
                        | "lastModifiedBy"
                        | "created"
                        | "modified"
                        | "revision"
                        | "title"
                        | "subject"
                        | "description"
                        | "keywords"
                        | "category"
                        | "contentStatus"
                        | "lastPrinted"
                ) {
                    current_field = Some(local);
                }
            }
            Ok(Event::Text(value)) => {
                if let Some(field) = &current_field {
                    decoded.clear();
                    append_decoded_xml_text(value.as_ref(), &mut decoded);
                    assign_core_value(&mut props, field, &decoded);
                }
            }
            Ok(Event::CData(value)) => {
                if let Some(field) = &current_field {
                    decoded.clear();
                    append_decoded_xml_text(value.as_ref(), &mut decoded);
                    assign_core_value(&mut props, field, &decoded);
                }
            }
            Ok(Event::GeneralRef(value)) => {
                if let Some(field) = &current_field {
                    decoded.clear();
                    append_decoded_xml_reference(value.as_ref(), &mut decoded);
                    assign_core_value(&mut props, field, &decoded);
                }
            }
            Ok(Event::End(element))
                if current_field
                    .as_deref()
                    .is_some_and(|field| name_eq(element.name().as_ref(), local_name(field))) =>
            {
                current_field = None;
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

    Ok(Extraction::with_warnings(props, warnings))
}

fn parse_app<R: BufRead>(source: R, path: &str) -> Result<Extraction<AppProps>> {
    let mut reader = Reader::from_reader(source);
    reader.config_mut().trim_text(false);
    let mut buf = Vec::new();
    let mut props = AppProps::default();
    let mut warnings = Vec::new();
    let mut current_field: Option<String> = None;
    let mut decoded = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(element)) => {
                let local = element.name().as_ref().to_owned();
                if matches!(
                    crate::parsers::local_name(&local),
                    "Application" | "Company" | "Words" | "Pages" | "Slides" | "Worksheets"
                ) {
                    current_field = Some(local);
                }
            }
            Ok(Event::Text(value)) => {
                if let Some(field) = &current_field {
                    decoded.clear();
                    append_decoded_xml_text(value.as_ref(), &mut decoded);
                    assign_app_value(&mut props, field, &decoded);
                }
            }
            Ok(Event::CData(value)) => {
                if let Some(field) = &current_field {
                    decoded.clear();
                    append_decoded_xml_text(value.as_ref(), &mut decoded);
                    assign_app_value(&mut props, field, &decoded);
                }
            }
            Ok(Event::GeneralRef(value)) => {
                if let Some(field) = &current_field {
                    decoded.clear();
                    append_decoded_xml_reference(value.as_ref(), &mut decoded);
                    assign_app_value(&mut props, field, &decoded);
                }
            }
            Ok(Event::End(element))
                if current_field
                    .as_deref()
                    .is_some_and(|field| name_eq(element.name().as_ref(), local_name(field))) =>
            {
                current_field = None;
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

    Ok(Extraction::with_warnings(props, warnings))
}

fn parse_custom<R: BufRead>(source: R, path: &str) -> Result<Extraction<CustomProps>> {
    let mut reader = Reader::from_reader(source);
    reader.config_mut().trim_text(false);
    let mut buf = Vec::new();
    let mut props = CustomProps::default();
    let mut warnings = Vec::new();
    let mut current_property: Option<CustomPropertyValue> = None;
    let mut decoded = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(element)) => {
                if name_eq(element.name().as_ref(), "property")
                    && let Some(name) = attr_value(&element, "name")
                {
                    current_property = Some(CustomPropertyValue {
                        name,
                        value: String::new(),
                        saw_value: false,
                        value_depth: 0,
                    });
                } else if let Some(property) = &mut current_property {
                    property.value_depth += 1;
                    property.saw_value = true;
                }
            }
            Ok(Event::Empty(element)) => {
                if let Some(property) = &mut current_property
                    && !name_eq(element.name().as_ref(), "property")
                {
                    property.saw_value = true;
                }
            }
            Ok(Event::Text(value)) => {
                if let Some(property) = &mut current_property
                    && property.value_depth > 0
                {
                    append_decoded_xml_text(value.as_ref(), &mut property.value);
                    property.saw_value = true;
                }
            }
            Ok(Event::CData(value)) => {
                if let Some(property) = &mut current_property
                    && property.value_depth > 0
                {
                    append_decoded_xml_text(value.as_ref(), &mut property.value);
                    property.saw_value = true;
                }
            }
            Ok(Event::GeneralRef(value)) => {
                if let Some(property) = &mut current_property
                    && property.value_depth > 0
                {
                    decoded.clear();
                    append_decoded_xml_reference(value.as_ref(), &mut decoded);
                    property.value.push_str(&decoded);
                    property.saw_value = true;
                }
            }
            Ok(Event::End(element)) => {
                if name_eq(element.name().as_ref(), "property")
                    && let Some(property) = current_property.take()
                    && property.saw_value
                    && !property.name.is_empty()
                {
                    props.values.insert(property.name, property.value);
                } else if let Some(property) = &mut current_property {
                    property.value_depth = property.value_depth.saturating_sub(1);
                }
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

    Ok(Extraction::with_warnings(props, warnings))
}

struct CustomPropertyValue {
    name: String,
    value: String,
    saw_value: bool,
    value_depth: usize,
}

fn apply_core(info: &mut DocumentInfo, props: CoreProps) {
    info.author = props.author;
    info.last_modified_by = props.last_modified_by;
    info.created_at = props.created_at;
    info.modified_at = props.modified_at;
    info.revision = props.revision;
    info.title = props.title;
    info.subject = props.subject;
    info.description = props.description;
    info.keywords = props.keywords;
    info.category = props.category;
    info.content_status = props.content_status;
    info.last_printed = props.last_printed;
}

fn assign_core_value(props: &mut CoreProps, field: &str, value: &str) {
    match crate::parsers::local_name(field) {
        "creator" => append_option(&mut props.author, value),
        "lastModifiedBy" => {
            append_option(&mut props.last_modified_by, value);
        }
        "created" => append_option(&mut props.created_at, value),
        "modified" => append_option(&mut props.modified_at, value),
        "revision" => append_option(&mut props.revision, value),
        "title" => append_option(&mut props.title, value),
        "subject" => append_option(&mut props.subject, value),
        "description" => append_option(&mut props.description, value),
        "keywords" => append_option(&mut props.keywords, value),
        "category" => append_option(&mut props.category, value),
        "contentStatus" => append_option(&mut props.content_status, value),
        "lastPrinted" => append_option(&mut props.last_printed, value),
        _ => {}
    }
}

fn assign_app_value(props: &mut AppProps, field: &str, value: &str) {
    match crate::parsers::local_name(field) {
        "Application" => append_option(&mut props.application, value),
        "Company" => append_option(&mut props.company, value),
        "Words" => props.word_count = value.parse().ok(),
        "Pages" => props.page_count = value.parse().ok(),
        "Slides" => props.slide_count = value.parse().ok(),
        "Worksheets" => props.worksheet_count = value.parse().ok(),
        _ => {}
    }
}

fn append_option(target: &mut Option<String>, value: &str) {
    match target {
        Some(existing) => existing.push_str(value),
        None => *target = Some(value.to_owned()),
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

fn apply_custom(info: &mut DocumentInfo, props: CustomProps) {
    if !props.values.is_empty() {
        info.custom_properties = Some(props.values);
    }
}

fn is_vba_project_content_type(content_type: &str) -> bool {
    content_type.eq_ignore_ascii_case("application/vnd.ms-office.vbaProject")
}

#[doc(hidden)]
pub fn fuzz_parse_metadata(xml: &[u8]) -> Result<()> {
    let _ = parse_core(Cursor::new(xml), "docProps/core.xml")?;
    let _ = parse_app(Cursor::new(xml), "docProps/app.xml")?;
    let _ = parse_custom(Cursor::new(xml), "docProps/custom.xml")?;
    let _ = parse_content_types(Cursor::new(xml), "[Content_Types].xml")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::{parse_app, parse_content_types, parse_core, parse_custom};

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
        assert_eq!(props.title.as_deref(), None);
        assert_eq!(props.subject.as_deref(), None);
        assert_eq!(props.description.as_deref(), None);
        assert_eq!(props.keywords.as_deref(), None);
        assert_eq!(props.category.as_deref(), None);
        assert_eq!(props.content_status.as_deref(), None);
        assert_eq!(props.last_printed.as_deref(), None);
    }

    #[test]
    fn parses_extended_core_properties() {
        let xml = r#"
            <cp:coreProperties xmlns:cp="cp" xmlns:dc="dc">
              <dc:title>Quarterly Report</dc:title>
              <dc:subject>Financials</dc:subject>
              <dc:description>Numbers &amp; charts</dc:description>
              <cp:keywords>finance; report; q1</cp:keywords>
              <cp:category>Reports</cp:category>
              <cp:contentStatus>Final</cp:contentStatus>
              <cp:lastPrinted>2024-03-14T09:30:00Z</cp:lastPrinted>
            </cp:coreProperties>
        "#;

        let props = parse_core(Cursor::new(xml.as_bytes()), "docProps/core.xml")
            .unwrap()
            .value;

        assert_eq!(props.title.as_deref(), Some("Quarterly Report"));
        assert_eq!(props.subject.as_deref(), Some("Financials"));
        assert_eq!(props.description.as_deref(), Some("Numbers & charts"));
        assert_eq!(props.keywords.as_deref(), Some("finance; report; q1"));
        assert_eq!(props.category.as_deref(), Some("Reports"));
        assert_eq!(props.content_status.as_deref(), Some("Final"));
        assert_eq!(props.last_printed.as_deref(), Some("2024-03-14T09:30:00Z"));
    }

    #[test]
    fn parses_extended_core_properties_cdata() {
        let xml = r#"
            <cp:coreProperties xmlns:cp="cp" xmlns:dc="dc">
              <dc:title><![CDATA[Report < Q1]]></dc:title>
              <cp:keywords><![CDATA[a < b]]></cp:keywords>
            </cp:coreProperties>
        "#;

        let props = parse_core(Cursor::new(xml.as_bytes()), "docProps/core.xml")
            .unwrap()
            .value;

        assert_eq!(props.title.as_deref(), Some("Report < Q1"));
        assert_eq!(props.keywords.as_deref(), Some("a < b"));
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

    #[test]
    fn appends_split_core_text_and_warns_on_malformed_xml() {
        let xml = r#"<cp:coreProperties xmlns:cp="cp" xmlns:dc="dc"><dc:creator>Ada &amp; Linus</dc:creator><"#;

        let result = parse_core(Cursor::new(xml.as_bytes()), "docProps/core.xml").unwrap();

        assert_eq!(result.value.author.as_deref(), Some("Ada & Linus"));
        assert_eq!(result.warnings.len(), 1);
    }

    #[test]
    fn ignores_invalid_numeric_app_properties_and_warns() {
        let xml = r#"<Properties><Words>many</Words><Slides>3</Slides><"#;

        let result = parse_app(Cursor::new(xml.as_bytes()), "docProps/app.xml").unwrap();

        assert_eq!(result.value.word_count, None);
        assert_eq!(result.value.slide_count, Some(3));
        assert_eq!(result.warnings.len(), 1);
    }

    #[test]
    fn parses_metadata_cdata_values() {
        let core_xml = r#"
            <cp:coreProperties xmlns:cp="cp" xmlns:dc="dc">
              <dc:creator><![CDATA[Ada < Linus]]></dc:creator>
            </cp:coreProperties>
        "#;
        let app_xml = r#"
            <Properties>
              <Application><![CDATA[LibreOffice < Writer]]></Application>
            </Properties>
        "#;

        let core = parse_core(Cursor::new(core_xml.as_bytes()), "docProps/core.xml")
            .unwrap()
            .value;
        let app = parse_app(Cursor::new(app_xml.as_bytes()), "docProps/app.xml")
            .unwrap()
            .value;

        assert_eq!(core.author.as_deref(), Some("Ada < Linus"));
        assert_eq!(app.application.as_deref(), Some("LibreOffice < Writer"));
    }

    #[test]
    fn parses_custom_properties() {
        let xml = r#"
            <Properties>
              <property name="Department">
                <vt:lpwstr>Research &amp; Development</vt:lpwstr>
              </property>
              <property name="Reviewed">
                <vt:bool>true</vt:bool>
              </property>
              <property name="Empty">
                <vt:lpwstr></vt:lpwstr>
              </property>
              <property name="">
                <vt:lpwstr>ignored</vt:lpwstr>
              </property>
            </Properties>
        "#;

        let props = parse_custom(Cursor::new(xml.as_bytes()), "docProps/custom.xml")
            .unwrap()
            .value;

        assert_eq!(
            props.values.get("Department").map(String::as_str),
            Some("Research & Development")
        );
        assert_eq!(
            props.values.get("Reviewed").map(String::as_str),
            Some("true")
        );
        assert_eq!(props.values.get("Empty").map(String::as_str), Some(""));
        assert!(!props.values.contains_key(""));
    }

    #[test]
    fn detects_macro_content_types() {
        let xml = r#"
            <Types>
              <Default Extension="bin" ContentType="application/vnd.ms-office.vbaProject"/>
            </Types>
        "#;

        let result =
            parse_content_types(Cursor::new(xml.as_bytes()), "[Content_Types].xml").unwrap();

        assert!(result.value);
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn parses_metadata_general_refs() {
        let core_xml = r#"<cp:coreProperties xmlns:cp="cp" xmlns:dc="dc"><dc:creator>Ada &amp; Linus</dc:creator></cp:coreProperties>"#;
        let app_xml = r#"<Properties><Application>A &amp; B</Application></Properties>"#;

        let core = parse_core(Cursor::new(core_xml.as_bytes()), "docProps/core.xml")
            .unwrap()
            .value;
        let app = parse_app(Cursor::new(app_xml.as_bytes()), "docProps/app.xml")
            .unwrap()
            .value;

        assert_eq!(core.author.as_deref(), Some("Ada & Linus"));
        assert_eq!(app.application.as_deref(), Some("A & B"));
    }

    #[test]
    fn parses_custom_cdata_and_general_ref() {
        let xml = r#"
            <Properties>
              <property name="GeneralRef">
                <vt:lpwstr>foo &amp; bar</vt:lpwstr>
              </property>
              <property name="CData">
                <vt:lpwstr><![CDATA[foo < bar]]></vt:lpwstr>
              </property>
              <property name="NoValue" />
            </Properties>
        "#;
        let props = parse_custom(Cursor::new(xml.as_bytes()), "docProps/custom.xml")
            .unwrap()
            .value;
        assert_eq!(
            props.values.get("GeneralRef").map(String::as_str),
            Some("foo & bar")
        );
        assert_eq!(
            props.values.get("CData").map(String::as_str),
            Some("foo < bar")
        );
    }

    #[test]
    fn detects_macro_content_types_start_end() {
        let xml = r#"
            <Types>
              <Override Extension="bin" ContentType="application/vnd.ms-office.vbaProject"></Override>
            </Types>
        "#;
        let result =
            parse_content_types(Cursor::new(xml.as_bytes()), "[Content_Types].xml").unwrap();
        assert!(result.value);
    }

    #[test]
    fn test_fuzz_parse_metadata() {
        use super::fuzz_parse_metadata;
        assert!(fuzz_parse_metadata(b"<xml/>").is_ok());
    }
}
