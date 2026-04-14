use thiserror::Error;

pub type Result<T> = std::result::Result<T, OxdocError>;

#[derive(Debug, Error)]
pub enum OxdocError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("corrupted or unsupported ZIP container: {0}")]
    CorruptedZip(#[from] zip::result::ZipError),

    #[error("missing required OOXML part: {0}")]
    MissingPart(String),

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
