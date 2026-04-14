use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputWarning {
    pub path: String,
    pub message: String,
}

impl OutputWarning {
    pub fn new(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            message: message.into(),
        }
    }
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
    pub delimiter: u8,
}

impl Default for XlsxCsvOptions<'_> {
    fn default() -> Self {
        Self {
            sheet_name: None,
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
