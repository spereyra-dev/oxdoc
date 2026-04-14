use std::io::BufRead;

use quick_xml::Reader;
use quick_xml::events::Event;

use crate::Result;
use crate::models::{Extraction, OutputWarning};
use crate::parsers::{decode_xml_reference, decode_xml_text, name_eq};

pub(crate) fn extract_text<R: BufRead>(source: R, path: &str) -> Result<Extraction<String>> {
    let mut reader = Reader::from_reader(source);
    reader.config_mut().trim_text(false);
    let mut buf = Vec::new();
    let mut text = String::new();
    let mut warnings = Vec::new();
    let mut in_text_node = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(element)) => {
                if name_eq(element.name().as_ref(), b"t") {
                    in_text_node = true;
                }
            }
            Ok(Event::Empty(element)) => {
                if name_eq(element.name().as_ref(), b"tab") {
                    text.push('\t');
                } else if name_eq(element.name().as_ref(), b"br")
                    || name_eq(element.name().as_ref(), b"cr")
                {
                    push_newline(&mut text);
                }
            }
            Ok(Event::Text(value)) if in_text_node => {
                text.push_str(&decode_xml_text(value.as_ref()));
            }
            Ok(Event::CData(value)) if in_text_node => {
                text.push_str(&decode_xml_text(value.as_ref()));
            }
            Ok(Event::GeneralRef(value)) if in_text_node => {
                text.push_str(&decode_xml_reference(value.as_ref()));
            }
            Ok(Event::End(element)) => {
                if name_eq(element.name().as_ref(), b"t") {
                    in_text_node = false;
                } else if name_eq(element.name().as_ref(), b"p") {
                    push_newline(&mut text);
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

    Ok(Extraction::with_warnings(text, warnings))
}

fn push_newline(text: &mut String) {
    if !text.ends_with('\n') {
        text.push('\n');
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::extract_text;

    #[test]
    fn extracts_word_text_with_logical_breaks() {
        let xml = r#"
            <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
              <w:body>
                <w:p><w:r><w:t>Hola</w:t></w:r><w:r><w:tab/><w:t>Mundo</w:t></w:r></w:p>
                <w:p><w:r><w:t>Segundo &amp; final</w:t></w:r></w:p>
              </w:body>
            </w:document>
        "#;

        let result = extract_text(Cursor::new(xml.as_bytes()), "word/document.xml").unwrap();

        assert_eq!(result.value, "Hola\tMundo\nSegundo & final\n");
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn returns_partial_text_after_malformed_xml() {
        let xml = br#"<w:document><w:p><w:r><w:t>Hola</w:t></w:r></w:p><"#;

        let result = extract_text(Cursor::new(xml.as_slice()), "word/document.xml").unwrap();

        assert_eq!(result.value, "Hola\n");
        assert_eq!(result.warnings.len(), 1);
    }

    #[test]
    fn handles_cdata_breaks_and_empty_document() {
        let xml = r#"
            <w:document xmlns:w="w">
              <w:body>
                <w:p><w:r><w:t><![CDATA[A < B]]></w:t><w:br/><w:t>&#67;</w:t></w:r></w:p>
                <w:p><w:r><w:cr/></w:r></w:p>
              </w:body>
            </w:document>
        "#;

        let result = extract_text(Cursor::new(xml.as_bytes()), "word/document.xml").unwrap();
        let empty = extract_text(Cursor::new(b"<w:document/>"), "word/document.xml").unwrap();

        assert_eq!(result.value, "A < B\nC\n");
        assert!(empty.value.is_empty());
    }
}
