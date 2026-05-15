use std::collections::BTreeMap;

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputWarning {
    pub path: String,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WarningCategory {
    Parser,
    Data,
    Custom,
}

impl WarningCategory {
    pub fn as_str(self) -> &'static str {
        match self {
            WarningCategory::Parser => "parser",
            WarningCategory::Data => "data",
            WarningCategory::Custom => "custom",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WarningCode {
    MalformedXml,
    IgnoredWorkbookSheet,
    SharedStringIndexOutOfBounds,
    InvalidSharedStringIndex,
    Custom,
}

impl WarningCode {
    pub fn as_str(self) -> &'static str {
        match self {
            WarningCode::MalformedXml => "W001",
            WarningCode::IgnoredWorkbookSheet => "W002",
            WarningCode::SharedStringIndexOutOfBounds => "W003",
            WarningCode::InvalidSharedStringIndex => "W004",
            WarningCode::Custom => "W999",
        }
    }
}

impl OutputWarning {
    pub fn new(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            message: message.into(),
        }
    }

    pub fn malformed_xml(path: impl Into<String>, source: impl std::fmt::Display) -> Self {
        Self::new(path, format!("stopped after malformed XML: {source}"))
    }

    pub fn ignored_workbook_sheet(path: impl Into<String>) -> Self {
        Self::new(
            path,
            "ignored workbook sheet without name or relationship id",
        )
    }

    pub fn shared_string_index_out_of_bounds(path: impl Into<String>, index: usize) -> Self {
        Self::new(
            path,
            format!("shared string index {index} is out of bounds"),
        )
    }

    pub fn invalid_shared_string_index(path: impl Into<String>, value: impl Into<String>) -> Self {
        Self::new(
            path,
            format!("invalid shared string index '{}'", value.into()),
        )
    }

    pub fn category(&self) -> WarningCategory {
        match self.code() {
            WarningCode::MalformedXml => WarningCategory::Parser,
            WarningCode::IgnoredWorkbookSheet
            | WarningCode::SharedStringIndexOutOfBounds
            | WarningCode::InvalidSharedStringIndex => WarningCategory::Data,
            WarningCode::Custom => WarningCategory::Custom,
        }
    }

    pub fn code(&self) -> WarningCode {
        match self.message.as_str() {
            message if message.starts_with("stopped after malformed XML: ") => {
                WarningCode::MalformedXml
            }
            "ignored workbook sheet without name or relationship id" => {
                WarningCode::IgnoredWorkbookSheet
            }
            message
                if message.starts_with("shared string index ")
                    && message.ends_with(" is out of bounds") =>
            {
                WarningCode::SharedStringIndexOutOfBounds
            }
            message if message.starts_with("invalid shared string index '") => {
                WarningCode::InvalidSharedStringIndex
            }
            _ => WarningCode::Custom,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentType {
    Docx,
    Pptx,
    Xlsx,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XlsxSheet {
    pub index: usize,
    pub name: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum XlsxValueMode {
    #[default]
    Raw,
    Formatted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Extraction<T> {
    pub value: T,
    pub warnings: Vec<OutputWarning>,
}

impl<T> Extraction<T> {
    pub fn new(value: T) -> Self {
        Self {
            value,
            warnings: Vec::new(),
        }
    }

    pub fn with_warnings(value: T, warnings: Vec<OutputWarning>) -> Self {
        Self { value, warnings }
    }

    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Extraction<U> {
        Extraction {
            value: f(self.value),
            warnings: self.warnings,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct XlsxCsvOptions<'a> {
    pub sheet_name: Option<&'a str>,
    pub sheet_index: Option<usize>,
    pub delimiter: u8,
}

impl Default for XlsxCsvOptions<'_> {
    fn default() -> Self {
        Self {
            sheet_name: None,
            sheet_index: None,
            delimiter: b',',
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct DocumentInfo {
    pub file: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_modified_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub application: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub company: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_properties: Option<BTreeMap<String, String>>,
    pub has_macros: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub word_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slide_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub worksheet_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revision: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct DocumentAudit {
    pub file: String,
    pub document_type: String,
    pub metadata: DocumentInfo,
    pub signals: Vec<AuditSignal>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AuditSignal {
    pub kind: String,
    pub severity: String,
    pub path: String,
    pub message: String,
}

impl AuditSignal {
    pub fn new(
        kind: impl Into<String>,
        severity: impl Into<String>,
        path: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            kind: kind.into(),
            severity: severity.into(),
            path: path.into(),
            message: message.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Extraction, OutputWarning, WarningCategory, WarningCode, XlsxCsvOptions};

    #[test]
    fn builds_and_maps_extractions() {
        let warning = OutputWarning::malformed_xml("word/document.xml", "parse error");
        let extraction = Extraction::with_warnings("hello".to_owned(), vec![warning.clone()]);

        let mapped = extraction.map(|value| value.len());

        assert_eq!(mapped.value, 5);
        assert_eq!(mapped.warnings, vec![warning]);
        assert!(Extraction::new(()).warnings.is_empty());
    }

    #[test]
    fn classifies_warning_codes_and_categories() {
        let malformed = OutputWarning::malformed_xml("word/document.xml", "parse error");
        let sheet = OutputWarning::ignored_workbook_sheet("xl/workbook.xml");
        let shared = OutputWarning::shared_string_index_out_of_bounds("xl/sheet.xml", 7);
        let invalid = OutputWarning::invalid_shared_string_index("xl/sheet.xml", "abc");

        assert_eq!(malformed.category(), WarningCategory::Parser);
        assert_eq!(malformed.code(), WarningCode::MalformedXml);
        assert_eq!(sheet.category(), WarningCategory::Data);
        assert_eq!(sheet.code(), WarningCode::IgnoredWorkbookSheet);
        assert_eq!(shared.code(), WarningCode::SharedStringIndexOutOfBounds);
        assert_eq!(invalid.code(), WarningCode::InvalidSharedStringIndex);
        assert_eq!(WarningCategory::Parser.as_str(), "parser");
        assert_eq!(WarningCategory::Data.as_str(), "data");
        assert_eq!(WarningCode::MalformedXml.as_str(), "W001");
        assert_eq!(WarningCode::IgnoredWorkbookSheet.as_str(), "W002");
        assert_eq!(WarningCode::SharedStringIndexOutOfBounds.as_str(), "W003");
        assert_eq!(WarningCode::InvalidSharedStringIndex.as_str(), "W004");
    }

    #[test]
    fn classifies_unknown_warnings_as_custom() {
        let warning = OutputWarning::new("custom.xml", "partial extraction");

        assert_eq!(warning.category(), WarningCategory::Custom);
        assert_eq!(warning.code(), WarningCode::Custom);
        assert_eq!(warning.category().as_str(), "custom");
        assert_eq!(warning.code().as_str(), "W999");
    }

    #[test]
    fn defaults_xlsx_csv_options() {
        let options = XlsxCsvOptions::default();

        assert_eq!(options.sheet_name, None);
        assert_eq!(options.sheet_index, None);
        assert_eq!(options.delimiter, b',');
    }
}
