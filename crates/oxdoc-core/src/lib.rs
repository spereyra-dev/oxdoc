//! Core OOXML extraction APIs for `oxdoc`.
//!
//! This crate reads Office Open XML packages without rendering them. It exposes
//! path-based helpers for DOCX/PPTX text extraction, XLSX-to-CSV extraction,
//! and package metadata. Extraction returns useful output plus recoverable warnings,
//! while unrecoverable package and parser failures are returned as typed errors.
//!
//! ```no_run
//! # fn demo() -> oxdoc_core::Result<()> {
//! let extraction = oxdoc_core::extract_docx_text("contract.docx")?;
//! println!("{}", extraction.value);
//! # Ok(())
//! # }
//! ```
//!
//! The public API is pre-1.0 and may change while the project hardens parser
//! behavior and streaming boundaries.

mod error;
pub mod models;
mod parsers;
pub mod vfs;

use std::fs::File;
use std::io::{BufReader, Read, Seek, Write};
use std::path::Path;

pub use error::{OxdocError, Result};
pub use models::{DocumentInfo, Extraction, OutputWarning, XlsxCsvOptions};
#[doc(hidden)]
pub use parsers::docx::fuzz_extract_text as fuzz_docx_text;
#[doc(hidden)]
pub use parsers::fuzz_parse_relationships as fuzz_relationships;
#[doc(hidden)]
pub use parsers::metadata::fuzz_parse_metadata as fuzz_metadata;
#[doc(hidden)]
pub use parsers::pptx::fuzz_extract_text as fuzz_pptx_text;
#[doc(hidden)]
pub use parsers::xlsx::{fuzz_parse_shared_strings, fuzz_parse_sheet};
use parsers::{docx, metadata, pptx, xlsx};
use vfs::OoxmlPackage;

pub fn extract_docx_text(path: impl AsRef<Path>) -> Result<Extraction<String>> {
    let file = File::open(path)?;
    extract_docx_text_from_reader(file)
}

pub fn extract_docx_text_from_reader<R: Read + Seek>(reader: R) -> Result<Extraction<String>> {
    let mut package = OoxmlPackage::new(reader)?;
    let document_path = parsers::find_office_document_path(&mut package, "word/document.xml")?;

    package.with_entry(&document_path, |entry| {
        let reader = BufReader::new(entry);
        docx::extract_text(reader, &document_path)
    })
}

pub fn extract_pptx_text(path: impl AsRef<Path>) -> Result<Extraction<String>> {
    let file = File::open(path)?;
    extract_pptx_text_from_reader(file)
}

pub fn extract_pptx_text_from_reader<R: Read + Seek>(reader: R) -> Result<Extraction<String>> {
    let mut package = OoxmlPackage::new(reader)?;
    pptx::extract_text(&mut package)
}

pub fn extract_xlsx_csv<W: Write>(
    path: impl AsRef<Path>,
    options: XlsxCsvOptions<'_>,
    writer: W,
) -> Result<Extraction<()>> {
    let file = File::open(path)?;
    extract_xlsx_csv_from_reader(file, options, writer)
}

pub fn extract_xlsx_csv_from_reader<R: Read + Seek, W: Write>(
    reader: R,
    options: XlsxCsvOptions<'_>,
    writer: W,
) -> Result<Extraction<()>> {
    let mut package = OoxmlPackage::new(reader)?;
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
    read_info_from_reader(file, file_name)
}

pub fn read_info_from_reader<R: Read + Seek>(
    reader: R,
    file_name: impl Into<String>,
) -> Result<Extraction<DocumentInfo>> {
    let mut package = OoxmlPackage::new(reader)?;
    metadata::read_info(&mut package, file_name.into())
}
