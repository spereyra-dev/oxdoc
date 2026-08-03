//! Reusable XLSX schema inference with an optional Arrow/Parquet adapter.
//!
//! Schema inference is dependency-light and always available. Enable the
//! `parquet` feature for bounded Arrow batches and Parquet writing.

pub mod xlsx_schema;

#[cfg(feature = "parquet")]
mod parquet;

#[cfg(feature = "parquet")]
pub use parquet::*;
