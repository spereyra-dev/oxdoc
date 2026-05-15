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
//! The public API follows semantic versioning from 1.0 onward.

mod error;
pub mod models;
mod parsers;
pub mod vfs;

use std::fs::File;
use std::io::{Cursor, Read, Seek, Write};
use std::path::Path;

pub use error::{OxdocError, Result};
pub use models::{
    AuditSignal, DocumentAudit, DocumentInfo, DocumentType, Extraction, OutputWarning,
    StructuredText, TextBlock, XlsxCsvOptions, XlsxSheet, XlsxSheetVisibility, XlsxValueMode,
};
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
    docx::extract_text(&mut package)
}

pub fn extract_docx_structured_text(path: impl AsRef<Path>) -> Result<Extraction<StructuredText>> {
    let file = File::open(path)?;
    extract_docx_structured_text_from_reader(file)
}

pub fn extract_docx_structured_text_from_reader<R: Read + Seek>(
    reader: R,
) -> Result<Extraction<StructuredText>> {
    let mut package = OoxmlPackage::new(reader)?;
    docx::extract_structured_text(&mut package)
}

pub fn extract_pptx_text(path: impl AsRef<Path>) -> Result<Extraction<String>> {
    let file = File::open(path)?;
    extract_pptx_text_from_reader(file)
}

pub fn extract_pptx_text_from_reader<R: Read + Seek>(reader: R) -> Result<Extraction<String>> {
    let mut package = OoxmlPackage::new(reader)?;
    pptx::extract_text(&mut package)
}

pub fn extract_pptx_structured_text(path: impl AsRef<Path>) -> Result<Extraction<StructuredText>> {
    let file = File::open(path)?;
    extract_pptx_structured_text_from_reader(file)
}

pub fn extract_pptx_structured_text_from_reader<R: Read + Seek>(
    reader: R,
) -> Result<Extraction<StructuredText>> {
    let mut package = OoxmlPackage::new(reader)?;
    pptx::extract_structured_text(&mut package)
}

pub fn extract_xlsx_csv<W: Write>(
    path: impl AsRef<Path>,
    options: XlsxCsvOptions<'_>,
    writer: W,
) -> Result<Extraction<()>> {
    let file = File::open(path)?;
    extract_xlsx_csv_from_reader(file, options, writer)
}

pub fn extract_xlsx_csv_with_value_mode<W: Write>(
    path: impl AsRef<Path>,
    options: XlsxCsvOptions<'_>,
    value_mode: XlsxValueMode,
    writer: W,
) -> Result<Extraction<()>> {
    let file = File::open(path)?;
    extract_xlsx_csv_from_reader_with_value_mode(file, options, value_mode, writer)
}

pub fn extract_xlsx_csv_from_reader<R: Read + Seek, W: Write>(
    reader: R,
    options: XlsxCsvOptions<'_>,
    writer: W,
) -> Result<Extraction<()>> {
    extract_xlsx_csv_from_reader_with_value_mode(reader, options, XlsxValueMode::Raw, writer)
}

pub fn extract_xlsx_csv_from_reader_with_value_mode<R: Read + Seek, W: Write>(
    reader: R,
    options: XlsxCsvOptions<'_>,
    value_mode: XlsxValueMode,
    writer: W,
) -> Result<Extraction<()>> {
    let mut package = OoxmlPackage::new(reader)?;
    xlsx::write_csv(&mut package, options, value_mode, writer)
}

pub fn list_xlsx_sheets(path: impl AsRef<Path>) -> Result<Extraction<Vec<XlsxSheet>>> {
    let file = File::open(path)?;
    list_xlsx_sheets_from_reader(file)
}

pub fn list_xlsx_sheets_from_reader<R: Read + Seek>(
    reader: R,
) -> Result<Extraction<Vec<XlsxSheet>>> {
    list_xlsx_sheets_from_reader_with_hidden(reader, false)
}

pub fn list_xlsx_sheets_with_hidden(
    path: impl AsRef<Path>,
    include_hidden: bool,
) -> Result<Extraction<Vec<XlsxSheet>>> {
    let file = File::open(path)?;
    list_xlsx_sheets_from_reader_with_hidden(file, include_hidden)
}

pub fn list_xlsx_sheets_from_reader_with_hidden<R: Read + Seek>(
    reader: R,
    include_hidden: bool,
) -> Result<Extraction<Vec<XlsxSheet>>> {
    let mut package = OoxmlPackage::new(reader)?;
    xlsx::list_sheets(&mut package, include_hidden)
}

pub fn detect_document_type(path: impl AsRef<Path>) -> Result<DocumentType> {
    let file = File::open(path)?;
    detect_document_type_from_reader(file)
}

pub fn detect_document_type_from_reader<R: Read>(mut reader: R) -> Result<DocumentType> {
    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes)?;
    let mut package = OoxmlPackage::new(Cursor::new(bytes))?;
    parsers::detect_document_type(&mut package)
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

pub fn read_audit(path: impl AsRef<Path>) -> Result<Extraction<DocumentAudit>> {
    let path = path.as_ref();
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_owned();

    let file = File::open(path)?;
    read_audit_from_reader(file, file_name)
}

pub fn read_audit_from_reader<R: Read + Seek>(
    reader: R,
    file_name: impl Into<String>,
) -> Result<Extraction<DocumentAudit>> {
    let mut package = OoxmlPackage::new(reader)?;
    parsers::audit::read_audit(&mut package, file_name.into())
}
