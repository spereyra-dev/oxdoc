//! Experimental Arrow and Parquet conversion for typed XLSX rows.
//!
//! This crate deliberately keeps the columnar dependency stack outside
//! `oxdoc-core`. Callers supply a schema and explicit zero-based worksheet
//! column indexes; no schema inference or value coercion is performed.

use std::io::{Read, Seek, Write};
use std::path::Path;
use std::sync::Arc;

use arrow_array::{
    ArrayRef, BooleanArray, Date32Array, Float64Array, Int64Array, NullArray, RecordBatch,
    StringArray, Time64MicrosecondArray, TimestampMicrosecondArray,
};
use arrow_schema::{DataType, Field, Schema, SchemaRef, TimeUnit};
use oxdoc_core::{
    Extraction, OxdocError, XlsxCellValue, XlsxRow, XlsxRowControl, XlsxSheetOptions, XlsxValueMode,
};
use parquet::arrow::ArrowWriter;
use thiserror::Error;

/// Logical types accepted by the experimental XLSX-to-Arrow converter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabularType {
    Null,
    Bool,
    Int64,
    Float64,
    Date,
    Time,
    DateTime,
    Utf8,
}

impl TabularType {
    fn arrow_type(self) -> DataType {
        match self {
            Self::Null => DataType::Null,
            Self::Bool => DataType::Boolean,
            Self::Int64 => DataType::Int64,
            Self::Float64 => DataType::Float64,
            Self::Date => DataType::Date32,
            Self::Time => DataType::Time64(TimeUnit::Microsecond),
            Self::DateTime => DataType::Timestamp(TimeUnit::Microsecond, None),
            Self::Utf8 => DataType::Utf8,
        }
    }
}

/// One output column mapped to a zero-based XLSX worksheet column.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Column {
    pub name: String,
    pub index: usize,
    pub data_type: TabularType,
    pub nullable: bool,
}

impl Column {
    pub fn new(
        name: impl Into<String>,
        index: usize,
        data_type: TabularType,
        nullable: bool,
    ) -> Self {
        Self {
            name: name.into(),
            index,
            data_type,
            nullable,
        }
    }
}

/// Explicit schema used for XLSX conversion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TabularSchema {
    columns: Vec<Column>,
}

impl TabularSchema {
    pub fn new(columns: Vec<Column>) -> Result<Self> {
        if columns.is_empty() {
            return Err(Error::EmptySchema);
        }
        for (position, column) in columns.iter().enumerate() {
            if column.name.is_empty() {
                return Err(Error::EmptyColumnName { position });
            }
            if columns[..position]
                .iter()
                .any(|other| other.index == column.index)
            {
                return Err(Error::DuplicateColumnIndex {
                    index: column.index,
                });
            }
            if column.data_type == TabularType::Null && !column.nullable {
                return Err(Error::NonNullableNullColumn {
                    name: column.name.clone(),
                });
            }
        }
        Ok(Self { columns })
    }

    pub fn columns(&self) -> &[Column] {
        &self.columns
    }

    pub fn arrow_schema(&self) -> SchemaRef {
        Arc::new(Schema::new(
            self.columns
                .iter()
                .map(|column| {
                    Field::new(&column.name, column.data_type.arrow_type(), column.nullable)
                })
                .collect::<Vec<_>>(),
        ))
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("tabular schema must contain at least one column")]
    EmptySchema,
    #[error("column at schema position {position} has an empty name")]
    EmptyColumnName { position: usize },
    #[error("worksheet column index {index} appears more than once in the schema")]
    DuplicateColumnIndex { index: usize },
    #[error("null column {name:?} must be nullable")]
    NonNullableNullColumn { name: String },
    #[error("batch_rows must be greater than zero")]
    InvalidBatchRows,
    #[error(
        "row {row_index}, worksheet column {column_index} ({column_name:?}) expected {expected:?}, found {actual}"
    )]
    SchemaMismatch {
        row_index: usize,
        column_index: usize,
        column_name: String,
        expected: TabularType,
        actual: String,
    },
    #[error(transparent)]
    Core(#[from] OxdocError),
    #[error(transparent)]
    Arrow(#[from] arrow_schema::ArrowError),
    #[error(transparent)]
    Parquet(#[from] parquet::errors::ParquetError),
}

pub type Result<T> = std::result::Result<T, Error>;

/// Counts emitted rows and bounded Arrow batches.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BatchStats {
    pub rows: usize,
    pub batches: usize,
}

/// Counts rows, Arrow batches, and flushed Parquet row groups.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ParquetStats {
    pub rows: usize,
    pub batches: usize,
    pub row_groups: usize,
}

/// Visit bounded Arrow record batches converted from one worksheet.
///
/// Date, time, and datetime columns require recognized XLSX number formats,
/// so this function requests [`XlsxValueMode::Formatted`] from `oxdoc-core`.
pub fn visit_xlsx_record_batches<F>(
    path: impl AsRef<Path>,
    options: XlsxSheetOptions<'_>,
    schema: &TabularSchema,
    batch_rows: usize,
    visitor: F,
) -> Result<Extraction<BatchStats>>
where
    F: FnMut(&RecordBatch) -> Result<()>,
{
    let file = std::fs::File::open(path).map_err(OxdocError::from)?;
    visit_xlsx_record_batches_from_reader(file, options, schema, batch_rows, visitor)
}

/// Reader-based counterpart to [`visit_xlsx_record_batches`].
pub fn visit_xlsx_record_batches_from_reader<R, F>(
    reader: R,
    options: XlsxSheetOptions<'_>,
    schema: &TabularSchema,
    batch_rows: usize,
    mut visitor: F,
) -> Result<Extraction<BatchStats>>
where
    R: Read + Seek,
    F: FnMut(&RecordBatch) -> Result<()>,
{
    if batch_rows == 0 {
        return Err(Error::InvalidBatchRows);
    }

    let arrow_schema = schema.arrow_schema();
    let mut converter = BatchConverter::new(schema, Arc::clone(&arrow_schema), batch_rows);
    let mut callback_error = None;

    let extraction =
        oxdoc_core::visit_xlsx_rows_from_reader(reader, options, XlsxValueMode::Formatted, |row| {
            let result = converter.push(row).and_then(|batch| {
                if let Some(batch) = batch {
                    visitor(&batch)?;
                }
                Ok(())
            });
            match result {
                Ok(()) => Ok(XlsxRowControl::Continue),
                Err(error) => {
                    callback_error = Some(error);
                    Err(OxdocError::InvalidArgument(
                        "tabular conversion aborted".to_owned(),
                    ))
                }
            }
        });

    if let Some(error) = callback_error {
        return Err(error);
    }
    let warnings = extraction?.warnings;
    if let Some(batch) = converter.finish()? {
        visitor(&batch)?;
    }

    Ok(Extraction::with_warnings(converter.stats, warnings))
}

/// Convert one worksheet to Parquet, flushing each bounded batch as a row group.
pub fn write_xlsx_parquet<W>(
    path: impl AsRef<Path>,
    options: XlsxSheetOptions<'_>,
    schema: &TabularSchema,
    batch_rows: usize,
    writer: W,
) -> Result<Extraction<ParquetStats>>
where
    W: Write + Send,
{
    let file = std::fs::File::open(path).map_err(OxdocError::from)?;
    write_xlsx_parquet_from_reader(file, options, schema, batch_rows, writer)
}

/// Reader-based counterpart to [`write_xlsx_parquet`].
pub fn write_xlsx_parquet_from_reader<R, W>(
    reader: R,
    options: XlsxSheetOptions<'_>,
    schema: &TabularSchema,
    batch_rows: usize,
    writer: W,
) -> Result<Extraction<ParquetStats>>
where
    R: Read + Seek,
    W: Write + Send,
{
    let mut parquet = ArrowWriter::try_new(writer, schema.arrow_schema(), None)?;
    let extraction =
        visit_xlsx_record_batches_from_reader(reader, options, schema, batch_rows, |batch| {
            parquet.write(batch)?;
            parquet.flush()?;
            Ok(())
        })?;
    let row_groups = parquet.flushed_row_groups().len();
    parquet.close()?;

    Ok(extraction.map(|stats| ParquetStats {
        rows: stats.rows,
        batches: stats.batches,
        row_groups,
    }))
}

struct BatchConverter<'a> {
    schema: &'a TabularSchema,
    arrow_schema: SchemaRef,
    batch_rows: usize,
    buffers: Vec<ColumnBuffer>,
    stats: BatchStats,
}

impl<'a> BatchConverter<'a> {
    fn new(schema: &'a TabularSchema, arrow_schema: SchemaRef, batch_rows: usize) -> Self {
        Self {
            schema,
            arrow_schema,
            batch_rows,
            buffers: schema
                .columns
                .iter()
                .map(|column| ColumnBuffer::new(column.data_type, batch_rows))
                .collect(),
            stats: BatchStats::default(),
        }
    }

    fn push(&mut self, row: &XlsxRow) -> Result<Option<RecordBatch>> {
        for (column, buffer) in self.schema.columns.iter().zip(&mut self.buffers) {
            let value = row
                .cells
                .binary_search_by_key(&column.index, |cell| cell.column_index)
                .ok()
                .map(|position| &row.cells[position].value);
            buffer.push(row.row_index, column, value)?;
        }
        self.stats.rows += 1;

        if self.stats.rows.is_multiple_of(self.batch_rows) {
            self.take_batch().map(Some)
        } else {
            Ok(None)
        }
    }

    fn finish(&mut self) -> Result<Option<RecordBatch>> {
        if self.buffers.first().is_none_or(ColumnBuffer::is_empty) {
            Ok(None)
        } else {
            self.take_batch().map(Some)
        }
    }

    fn take_batch(&mut self) -> Result<RecordBatch> {
        let arrays = self
            .buffers
            .iter_mut()
            .map(ColumnBuffer::take_array)
            .collect();
        let batch = RecordBatch::try_new(Arc::clone(&self.arrow_schema), arrays)?;
        self.stats.batches += 1;
        Ok(batch)
    }
}

enum ColumnBuffer {
    Null(usize),
    Bool(Vec<Option<bool>>),
    Int64(Vec<Option<i64>>),
    Float64(Vec<Option<f64>>),
    Date(Vec<Option<i32>>),
    Time(Vec<Option<i64>>),
    DateTime(Vec<Option<i64>>),
    Utf8(Vec<Option<String>>),
}

impl ColumnBuffer {
    fn new(data_type: TabularType, capacity: usize) -> Self {
        match data_type {
            TabularType::Null => Self::Null(0),
            TabularType::Bool => Self::Bool(Vec::with_capacity(capacity)),
            TabularType::Int64 => Self::Int64(Vec::with_capacity(capacity)),
            TabularType::Float64 => Self::Float64(Vec::with_capacity(capacity)),
            TabularType::Date => Self::Date(Vec::with_capacity(capacity)),
            TabularType::Time => Self::Time(Vec::with_capacity(capacity)),
            TabularType::DateTime => Self::DateTime(Vec::with_capacity(capacity)),
            TabularType::Utf8 => Self::Utf8(Vec::with_capacity(capacity)),
        }
    }

    fn is_empty(&self) -> bool {
        match self {
            Self::Null(len) => *len == 0,
            Self::Bool(values) => values.is_empty(),
            Self::Int64(values) => values.is_empty(),
            Self::Float64(values) => values.is_empty(),
            Self::Date(values) => values.is_empty(),
            Self::Time(values) => values.is_empty(),
            Self::DateTime(values) => values.is_empty(),
            Self::Utf8(values) => values.is_empty(),
        }
    }

    fn push(
        &mut self,
        row_index: usize,
        column: &Column,
        value: Option<&XlsxCellValue>,
    ) -> Result<()> {
        if value.is_none() || matches!(value, Some(XlsxCellValue::Blank)) {
            if !column.nullable {
                return Err(mismatch(row_index, column, "null"));
            }
            self.push_null();
            return Ok(());
        }

        let value = value.expect("non-null value checked above");
        match (self, value) {
            (Self::Null(_), _) => Err(mismatch(row_index, column, value_kind(value))),
            (
                Self::Bool(values),
                XlsxCellValue::Boolean {
                    value: Some(value), ..
                },
            ) => {
                values.push(Some(*value));
                Ok(())
            }
            (Self::Int64(values), XlsxCellValue::Number { raw, .. }) => {
                values
                    .push(Some(raw.parse().map_err(|_| {
                        mismatch(row_index, column, value_kind(value))
                    })?));
                Ok(())
            }
            (Self::Float64(values), XlsxCellValue::Number { raw, .. }) => {
                let parsed: f64 = raw
                    .parse()
                    .map_err(|_| mismatch(row_index, column, value_kind(value)))?;
                if !parsed.is_finite() {
                    return Err(mismatch(row_index, column, "non-finite number"));
                }
                values.push(Some(parsed));
                Ok(())
            }
            (
                Self::Date(values),
                XlsxCellValue::Number {
                    formatted: Some(text),
                    ..
                },
            ) => {
                values
                    .push(Some(parse_date(text).ok_or_else(|| {
                        mismatch(row_index, column, value_kind(value))
                    })?));
                Ok(())
            }
            (
                Self::Time(values),
                XlsxCellValue::Number {
                    formatted: Some(text),
                    ..
                },
            ) => {
                values
                    .push(Some(parse_time(text).ok_or_else(|| {
                        mismatch(row_index, column, value_kind(value))
                    })?));
                Ok(())
            }
            (
                Self::DateTime(values),
                XlsxCellValue::Number {
                    formatted: Some(text),
                    ..
                },
            ) => {
                values
                    .push(Some(parse_datetime(text).ok_or_else(|| {
                        mismatch(row_index, column, value_kind(value))
                    })?));
                Ok(())
            }
            (Self::Utf8(values), XlsxCellValue::String { value, .. }) => {
                values.push(Some(value.clone()));
                Ok(())
            }
            (_, value) => Err(mismatch(row_index, column, value_kind(value))),
        }
    }

    fn push_null(&mut self) {
        match self {
            Self::Null(len) => *len += 1,
            Self::Bool(values) => values.push(None),
            Self::Int64(values) => values.push(None),
            Self::Float64(values) => values.push(None),
            Self::Date(values) => values.push(None),
            Self::Time(values) => values.push(None),
            Self::DateTime(values) => values.push(None),
            Self::Utf8(values) => values.push(None),
        }
    }

    fn take_array(&mut self) -> ArrayRef {
        match self {
            Self::Null(len) => Arc::new(NullArray::new(std::mem::take(len))),
            Self::Bool(values) => Arc::new(BooleanArray::from(std::mem::take(values))),
            Self::Int64(values) => Arc::new(Int64Array::from(std::mem::take(values))),
            Self::Float64(values) => Arc::new(Float64Array::from(std::mem::take(values))),
            Self::Date(values) => Arc::new(Date32Array::from(std::mem::take(values))),
            Self::Time(values) => Arc::new(Time64MicrosecondArray::from(std::mem::take(values))),
            Self::DateTime(values) => {
                Arc::new(TimestampMicrosecondArray::from(std::mem::take(values)))
            }
            Self::Utf8(values) => Arc::new(StringArray::from(std::mem::take(values))),
        }
    }
}

fn mismatch(row_index: usize, column: &Column, actual: impl Into<String>) -> Error {
    Error::SchemaMismatch {
        row_index,
        column_index: column.index,
        column_name: column.name.clone(),
        expected: column.data_type,
        actual: actual.into(),
    }
}

fn value_kind(value: &XlsxCellValue) -> &'static str {
    match value {
        XlsxCellValue::Blank => "null",
        XlsxCellValue::String { .. } => "string",
        XlsxCellValue::Boolean { value: Some(_), .. } => "boolean",
        XlsxCellValue::Boolean { value: None, .. } => "invalid boolean",
        XlsxCellValue::Number {
            formatted: Some(_), ..
        } => "formatted number",
        XlsxCellValue::Number { .. } => "number",
        XlsxCellValue::Error { .. } => "worksheet error",
        _ => "unsupported XLSX value",
    }
}

fn parse_date(value: &str) -> Option<i32> {
    let mut parts = value.split('-');
    let year: i32 = parts.next()?.parse().ok()?;
    let month: u32 = parts.next()?.parse().ok()?;
    let day: u32 = parts.next()?.parse().ok()?;
    if parts.next().is_some() || !valid_date(year, month, day) {
        return None;
    }
    i32::try_from(days_from_civil(year, month, day)).ok()
}

fn parse_time(value: &str) -> Option<i64> {
    let mut parts = value.split(':');
    let hour: i64 = parts.next()?.parse().ok()?;
    let minute: i64 = parts.next()?.parse().ok()?;
    let seconds = parts.next()?;
    if parts.next().is_some() || hour > 23 || minute > 59 {
        return None;
    }
    let (second_text, fraction_text) = seconds.split_once('.').unwrap_or((seconds, ""));
    let second: i64 = second_text.parse().ok()?;
    if second > 59 || fraction_text.len() > 6 || !fraction_text.bytes().all(|b| b.is_ascii_digit())
    {
        return None;
    }
    let fraction = if fraction_text.is_empty() {
        0
    } else {
        fraction_text.parse::<i64>().ok()? * 10_i64.pow(6 - fraction_text.len() as u32)
    };
    Some(((hour * 3_600 + minute * 60 + second) * 1_000_000) + fraction)
}

fn parse_datetime(value: &str) -> Option<i64> {
    let (date, time) = value.split_once('T')?;
    let days = i64::from(parse_date(date)?);
    days.checked_mul(86_400_000_000)?
        .checked_add(parse_time(time)?)
}

fn valid_date(year: i32, month: u32, day: u32) -> bool {
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let max_day = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return false,
    };
    (1..=max_day).contains(&day)
}

fn days_from_civil(year: i32, month: u32, day: u32) -> i64 {
    let year = year - i32::from(month <= 2);
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let yoe = year - era * 400;
    let month = month as i32;
    let doy = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day as i32 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    i64::from(era * 146_097 + doe - 719_468)
}

#[cfg(test)]
mod tests {
    use std::io::{Cursor, Write};

    use arrow_array::{
        Array, BooleanArray, Date32Array, Float64Array, Int64Array, StringArray,
        Time64MicrosecondArray, TimestampMicrosecondArray,
    };
    use oxdoc_core::XlsxCell;
    use parquet::file::reader::{FileReader, SerializedFileReader};
    use zip::write::SimpleFileOptions;

    use super::*;

    #[test]
    fn emits_multiple_bounded_batches() {
        let schema = test_schema();
        let mut sizes = Vec::new();
        let extraction = visit_xlsx_record_batches_from_reader(
            workbook(&[
                ("1", "alpha"),
                ("2", "beta"),
                ("3", "gamma"),
                ("4", "delta"),
                ("5", "epsilon"),
            ]),
            XlsxSheetOptions::default(),
            &schema,
            2,
            |batch| {
                sizes.push(batch.num_rows());
                let ids = batch
                    .column(0)
                    .as_any()
                    .downcast_ref::<Int64Array>()
                    .unwrap();
                let labels = batch
                    .column(1)
                    .as_any()
                    .downcast_ref::<StringArray>()
                    .unwrap();
                assert_eq!(ids.len(), labels.len());
                Ok(())
            },
        )
        .unwrap();

        assert_eq!(sizes, vec![2, 2, 1]);
        assert_eq!(
            extraction.value,
            BatchStats {
                rows: 5,
                batches: 3
            }
        );
    }

    #[test]
    fn writes_one_parquet_row_group_per_batch() {
        let schema = test_schema();
        let mut output = Vec::new();
        let extraction = write_xlsx_parquet_from_reader(
            workbook(&[
                ("1", "alpha"),
                ("2", "beta"),
                ("3", "gamma"),
                ("4", "delta"),
                ("5", "epsilon"),
            ]),
            XlsxSheetOptions::default(),
            &schema,
            2,
            &mut output,
        )
        .unwrap();

        assert_eq!(
            extraction.value,
            ParquetStats {
                rows: 5,
                batches: 3,
                row_groups: 3
            }
        );
        let reader = SerializedFileReader::new(bytes::Bytes::from(output)).unwrap();
        assert_eq!(reader.metadata().num_row_groups(), 3);
        assert_eq!(reader.metadata().file_metadata().num_rows(), 5);
    }

    #[test]
    fn rejects_schema_mismatch_with_row_and_column_context() {
        let schema =
            TabularSchema::new(vec![Column::new("id", 1, TabularType::Int64, false)]).unwrap();
        let error = visit_xlsx_record_batches_from_reader(
            workbook(&[("1", "not-an-integer")]),
            XlsxSheetOptions::default(),
            &schema,
            10,
            |_| Ok(()),
        )
        .unwrap_err();

        assert!(matches!(
            error,
            Error::SchemaMismatch {
                row_index: 0,
                column_index: 1,
                expected: TabularType::Int64,
                ..
            }
        ));
    }

    #[test]
    fn rejects_zero_batch_size() {
        let error = visit_xlsx_record_batches_from_reader(
            workbook(&[]),
            XlsxSheetOptions::default(),
            &test_schema(),
            0,
            |_| Ok(()),
        )
        .unwrap_err();
        assert!(matches!(error, Error::InvalidBatchRows));
    }

    #[test]
    fn converts_all_supported_types_and_sparse_nulls() {
        let schema = TabularSchema::new(vec![
            Column::new("null", 0, TabularType::Null, true),
            Column::new("bool", 1, TabularType::Bool, false),
            Column::new("int", 2, TabularType::Int64, false),
            Column::new("float", 3, TabularType::Float64, false),
            Column::new("date", 4, TabularType::Date, false),
            Column::new("time", 5, TabularType::Time, false),
            Column::new("datetime", 6, TabularType::DateTime, false),
            Column::new("text", 7, TabularType::Utf8, true),
        ])
        .unwrap();
        let mut converter = BatchConverter::new(&schema, schema.arrow_schema(), 2);
        converter
            .push(&XlsxRow {
                row_index: 4,
                cells: vec![
                    cell(
                        1,
                        XlsxCellValue::Boolean {
                            raw: "1".into(),
                            value: Some(true),
                        },
                    ),
                    number_cell(2, "42", None),
                    number_cell(3, "2.5", None),
                    number_cell(4, "45292", Some("2024-01-01")),
                    number_cell(5, "0.5", Some("12:00:00")),
                    number_cell(6, "45292.5", Some("2024-01-01T12:00:00")),
                    cell(
                        7,
                        XlsxCellValue::String {
                            raw: "hello".into(),
                            value: "hello".into(),
                        },
                    ),
                ],
            })
            .unwrap();
        let batch = converter.finish().unwrap().unwrap();

        assert_eq!(batch.column(0).logical_null_count(), 1);
        assert!(
            batch
                .column(1)
                .as_any()
                .downcast_ref::<BooleanArray>()
                .unwrap()
                .value(0)
        );
        assert_eq!(
            batch
                .column(2)
                .as_any()
                .downcast_ref::<Int64Array>()
                .unwrap()
                .value(0),
            42
        );
        assert_eq!(
            batch
                .column(3)
                .as_any()
                .downcast_ref::<Float64Array>()
                .unwrap()
                .value(0),
            2.5
        );
        assert_eq!(
            batch
                .column(4)
                .as_any()
                .downcast_ref::<Date32Array>()
                .unwrap()
                .value(0),
            19_723
        );
        assert_eq!(
            batch
                .column(5)
                .as_any()
                .downcast_ref::<Time64MicrosecondArray>()
                .unwrap()
                .value(0),
            43_200_000_000
        );
        assert_eq!(
            batch
                .column(6)
                .as_any()
                .downcast_ref::<TimestampMicrosecondArray>()
                .unwrap()
                .value(0),
            1_704_110_400_000_000
        );
        assert_eq!(
            batch
                .column(7)
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap()
                .value(0),
            "hello"
        );
    }

    fn test_schema() -> TabularSchema {
        TabularSchema::new(vec![
            Column::new("id", 0, TabularType::Int64, false),
            Column::new("label", 1, TabularType::Utf8, false),
        ])
        .unwrap()
    }

    fn workbook(rows: &[(&str, &str)]) -> Cursor<Vec<u8>> {
        let mut output = Cursor::new(Vec::new());
        {
            let mut zip = zip::ZipWriter::new(&mut output);
            let options = SimpleFileOptions::default();
            let files = [
                (
                    "[Content_Types].xml",
                    r#"<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>
  <Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>
</Types>"#
                        .to_owned(),
                ),
                (
                    "_rels/.rels",
                    r#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/>
</Relationships>"#
                        .to_owned(),
                ),
                (
                    "xl/workbook.xml",
                    r#"<?xml version="1.0" encoding="UTF-8"?>
<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <sheets><sheet name="Sheet1" sheetId="1" r:id="rId1"/></sheets>
</workbook>"#
                        .to_owned(),
                ),
                (
                    "xl/_rels/workbook.xml.rels",
                    r#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/>
</Relationships>"#
                        .to_owned(),
                ),
                ("xl/worksheets/sheet1.xml", sheet_xml(rows)),
            ];
            for (name, contents) in files {
                zip.start_file(name, options).unwrap();
                zip.write_all(contents.as_bytes()).unwrap();
            }
            zip.finish().unwrap();
        }
        output.set_position(0);
        output
    }

    fn sheet_xml(rows: &[(&str, &str)]) -> String {
        let rows = rows
            .iter()
            .enumerate()
            .map(|(index, (id, label))| {
                let row = index + 1;
                format!(
                    r#"<row r="{row}"><c r="A{row}"><v>{id}</v></c><c r="B{row}" t="inlineStr"><is><t>{label}</t></is></c></row>"#
                )
            })
            .collect::<String>();
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData>{rows}</sheetData></worksheet>"#
        )
    }

    fn cell(column_index: usize, value: XlsxCellValue) -> XlsxCell {
        XlsxCell {
            column_index,
            value,
            has_formula: false,
        }
    }

    fn number_cell(column_index: usize, raw: &str, formatted: Option<&str>) -> XlsxCell {
        cell(
            column_index,
            XlsxCellValue::Number {
                raw: raw.to_owned(),
                formatted: formatted.map(str::to_owned),
            },
        )
    }
}
