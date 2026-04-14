use thiserror::Error;

pub type Result<T> = std::result::Result<T, OxdocError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OxdocErrorCode {
    Io,
    CorruptedZip,
    MissingPart,
    UnsupportedEncryptedPart,
    PartTooLarge,
    SuspiciousZipEntry,
    SuspiciousRelationshipTarget,
    MissingCoreRelations,
    MalformedXmlNode,
    InvalidArgument,
}

impl OxdocErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            OxdocErrorCode::Io => "E001",
            OxdocErrorCode::CorruptedZip => "E002",
            OxdocErrorCode::MissingPart => "E003",
            OxdocErrorCode::UnsupportedEncryptedPart => "E004",
            OxdocErrorCode::PartTooLarge => "E005",
            OxdocErrorCode::SuspiciousZipEntry => "E006",
            OxdocErrorCode::SuspiciousRelationshipTarget => "E007",
            OxdocErrorCode::MissingCoreRelations => "E008",
            OxdocErrorCode::MalformedXmlNode => "E009",
            OxdocErrorCode::InvalidArgument => "E010",
        }
    }
}

#[derive(Debug, Error)]
pub enum OxdocError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("corrupted or unsupported ZIP container: {0}")]
    CorruptedZip(#[from] zip::result::ZipError),

    #[error("missing required OOXML part: {0}")]
    MissingPart(String),

    #[error("unsupported encrypted OOXML part: {0}")]
    UnsupportedEncryptedPart(String),

    #[error("OOXML part is too large: {path} is {size} bytes; limit is {limit} bytes")]
    PartTooLarge { path: String, size: u64, limit: u64 },

    #[error("suspicious OOXML ZIP entry: {path}: {reason}")]
    SuspiciousZipEntry { path: String, reason: String },

    #[error("suspicious OOXML relationship target in {path}: {target}: {reason}")]
    SuspiciousRelationshipTarget {
        path: String,
        target: String,
        reason: String,
    },

    #[error("missing office document relationship")]
    MissingCoreRelations,

    #[error("malformed XML in {path}: {source}")]
    MalformedXmlNode {
        path: String,
        #[source]
        source: quick_xml::Error,
    },

    #[error("invalid argument: {0}")]
    InvalidArgument(String),
}

impl OxdocError {
    pub fn code(&self) -> OxdocErrorCode {
        match self {
            OxdocError::Io(_) => OxdocErrorCode::Io,
            OxdocError::CorruptedZip(_) => OxdocErrorCode::CorruptedZip,
            OxdocError::MissingPart(_) => OxdocErrorCode::MissingPart,
            OxdocError::UnsupportedEncryptedPart(_) => OxdocErrorCode::UnsupportedEncryptedPart,
            OxdocError::PartTooLarge { .. } => OxdocErrorCode::PartTooLarge,
            OxdocError::SuspiciousZipEntry { .. } => OxdocErrorCode::SuspiciousZipEntry,
            OxdocError::SuspiciousRelationshipTarget { .. } => {
                OxdocErrorCode::SuspiciousRelationshipTarget
            }
            OxdocError::MissingCoreRelations => OxdocErrorCode::MissingCoreRelations,
            OxdocError::MalformedXmlNode { .. } => OxdocErrorCode::MalformedXmlNode,
            OxdocError::InvalidArgument(_) => OxdocErrorCode::InvalidArgument,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{OxdocError, OxdocErrorCode};

    #[test]
    fn maps_errors_to_stable_codes() {
        let cases = [
            (
                OxdocError::Io(std::io::Error::new(std::io::ErrorKind::NotFound, "missing")),
                OxdocErrorCode::Io,
                "E001",
            ),
            (
                OxdocError::CorruptedZip(zip::result::ZipError::FileNotFound),
                OxdocErrorCode::CorruptedZip,
                "E002",
            ),
            (
                OxdocError::MissingPart("word/document.xml".to_owned()),
                OxdocErrorCode::MissingPart,
                "E003",
            ),
            (
                OxdocError::UnsupportedEncryptedPart("word/document.xml".to_owned()),
                OxdocErrorCode::UnsupportedEncryptedPart,
                "E004",
            ),
            (
                OxdocError::PartTooLarge {
                    path: "word/document.xml".to_owned(),
                    size: 2,
                    limit: 1,
                },
                OxdocErrorCode::PartTooLarge,
                "E005",
            ),
            (
                OxdocError::SuspiciousZipEntry {
                    path: "word/document.xml".to_owned(),
                    reason: "directory".to_owned(),
                },
                OxdocErrorCode::SuspiciousZipEntry,
                "E006",
            ),
            (
                OxdocError::SuspiciousRelationshipTarget {
                    path: "_rels/.rels".to_owned(),
                    target: "https://example.invalid/document.xml".to_owned(),
                    reason: "external".to_owned(),
                },
                OxdocErrorCode::SuspiciousRelationshipTarget,
                "E007",
            ),
            (
                OxdocError::MissingCoreRelations,
                OxdocErrorCode::MissingCoreRelations,
                "E008",
            ),
            (
                OxdocError::MalformedXmlNode {
                    path: "_rels/.rels".to_owned(),
                    source: quick_xml::Error::Syntax(quick_xml::errors::SyntaxError::UnclosedTag),
                },
                OxdocErrorCode::MalformedXmlNode,
                "E009",
            ),
            (
                OxdocError::InvalidArgument("bad delimiter".to_owned()),
                OxdocErrorCode::InvalidArgument,
                "E010",
            ),
        ];

        for (error, code, label) in cases {
            assert_eq!(error.code(), code);
            assert_eq!(error.code().as_str(), label);
        }
    }
}
