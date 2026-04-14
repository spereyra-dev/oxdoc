mod error;
pub mod models;
mod parsers;
pub mod vfs;

use std::fs::File;
use std::io::{BufReader, Write};
use std::path::Path;

pub use error::{OxdocError, Result};
pub use models::{DocumentInfo, Extraction, OutputWarning, XlsxCsvOptions};
use parsers::{docx, metadata, xlsx};
use vfs::OoxmlPackage;

pub fn extract_docx_text(path: impl AsRef<Path>) -> Result<Extraction<String>> {
    let file = File::open(path)?;
    let mut package = OoxmlPackage::new(file)?;
    let document_path = parsers::find_office_document_path(&mut package, "word/document.xml")?;

    package.with_entry(&document_path, |entry| {
        let reader = BufReader::new(entry);
        docx::extract_text(reader, &document_path)
    })
}

pub fn extract_xlsx_csv<W: Write>(
    path: impl AsRef<Path>,
    options: XlsxCsvOptions<'_>,
    writer: W,
) -> Result<Extraction<()>> {
    let file = File::open(path)?;
    let mut package = OoxmlPackage::new(file)?;
    xlsx::write_csv(&mut package, options, writer)
}

pub fn read_info(path: impl AsRef<Path>) -> Result<Extraction<DocumentInfo>> {
    let path = path.as_ref();
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_owned();

    let file = File::open(path)?;
    let mut package = OoxmlPackage::new(file)?;
    metadata::read_info(&mut package, file_name)
}
