use std::fmt;

use oxdoc_core::{XlsxCellValue, XlsxRow, XlsxRowControl};
use serde::Serialize;

pub(crate) const XLSX_SCHEMA_VERSION: u8 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum XlsxSchemaScanMode {
    Full,
    Sampled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum XlsxLogicalType {
    Bool,
    Date,
    Datetime,
    Float64,
    Int64,
    Null,
    Time,
    Utf8,
}

impl fmt::Display for XlsxLogicalType {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            Self::Bool => "bool",
            Self::Date => "date",
            Self::Datetime => "datetime",
            Self::Float64 => "float64",
            Self::Int64 => "int64",
            Self::Null => "null",
            Self::Time => "time",
            Self::Utf8 => "utf8",
        };
        formatter.write_str(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum XlsxSchemaWarningCode {
    SampledResult,
    TypeConflict,
    Utf8Fallback,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct XlsxSchemaWarning {
    pub(crate) code: XlsxSchemaWarningCode,
    pub(crate) message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) column_index: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct XlsxSchemaColumn {
    pub(crate) column_index: usize,
    pub(crate) name: String,
    pub(crate) logical_type: XlsxLogicalType,
    pub(crate) nullable: bool,
    pub(crate) observed_types: Vec<XlsxLogicalType>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct XlsxSchemaScan {
    pub(crate) mode: XlsxSchemaScanMode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) sample_rows: Option<usize>,
    pub(crate) examined_rows: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct XlsxSchemaReport {
    pub(crate) schema_version: u8,
    pub(crate) experimental: bool,
    pub(crate) file: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) sheet_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) sheet_index: Option<usize>,
    pub(crate) scan: XlsxSchemaScan,
    pub(crate) header_policy: &'static str,
    pub(crate) columns: Vec<XlsxSchemaColumn>,
    pub(crate) warnings: Vec<XlsxSchemaWarning>,
}

#[derive(Debug)]
pub(crate) struct XlsxSchemaInferrer {
    sample_rows: Option<usize>,
    examined_rows: usize,
    columns: Vec<ColumnState>,
    warnings: Vec<XlsxSchemaWarning>,
}

impl XlsxSchemaInferrer {
    pub(crate) fn full_scan() -> Self {
        Self::new(None)
    }

    pub(crate) fn sampled(sample_rows: usize) -> Self {
        Self::new(Some(sample_rows))
    }

    fn new(sample_rows: Option<usize>) -> Self {
        Self {
            sample_rows,
            examined_rows: 0,
            columns: Vec::new(),
            warnings: Vec::new(),
        }
    }

    pub(crate) fn observe_row(&mut self, row: &XlsxRow) -> XlsxRowControl {
        if self
            .sample_rows
            .is_some_and(|limit| self.examined_rows >= limit)
        {
            return XlsxRowControl::Stop;
        }

        if let Some(last_cell) = row.cells.last() {
            let required_columns = last_cell.column_index.saturating_add(1);
            if required_columns > self.columns.len() {
                self.columns.resize_with(required_columns, || {
                    ColumnState::new(self.examined_rows > 0)
                });
            }
        }

        let mut cell_index = 0;
        for column_index in 0..self.columns.len() {
            while row
                .cells
                .get(cell_index)
                .is_some_and(|cell| cell.column_index < column_index)
            {
                cell_index += 1;
            }

            let cell = row
                .cells
                .get(cell_index)
                .filter(|cell| cell.column_index == column_index);
            let classification = cell
                .map(|cell| classify_cell(&cell.value))
                .unwrap_or(Classification::plain(XlsxLogicalType::Null));
            let column_name = excel_column_name(column_index);
            let column = &mut self.columns[column_index];

            if let Some(reason) = classification.fallback
                && !column.warned_fallback
            {
                self.warnings.push(XlsxSchemaWarning {
                    code: XlsxSchemaWarningCode::Utf8Fallback,
                    message: format!(
                        "{column_name}: {reason} at worksheet row {}; inferred utf8",
                        row.row_index
                    ),
                    column_index: Some(column_index),
                });
                column.warned_fallback = true;
            }

            if !types_are_compatible(column.logical_type, classification.logical_type)
                && !column.warned_conflict
            {
                self.warnings.push(XlsxSchemaWarning {
                    code: XlsxSchemaWarningCode::TypeConflict,
                    message: format!(
                        "{column_name}: observed {} after {} at worksheet row {}; promoted to utf8",
                        classification.logical_type, column.logical_type, row.row_index
                    ),
                    column_index: Some(column_index),
                });
                column.warned_conflict = true;
            }

            column.observe(classification.logical_type);
        }

        self.examined_rows += 1;
        if self
            .sample_rows
            .is_some_and(|limit| self.examined_rows >= limit)
        {
            XlsxRowControl::Stop
        } else {
            XlsxRowControl::Continue
        }
    }

    pub(crate) fn finish(
        mut self,
        file: String,
        sheet_name: Option<String>,
        sheet_index: Option<usize>,
    ) -> XlsxSchemaReport {
        let mode = if self.sample_rows.is_some() {
            XlsxSchemaScanMode::Sampled
        } else {
            XlsxSchemaScanMode::Full
        };
        if self.sample_rows.is_some() {
            self.warnings.push(XlsxSchemaWarning {
                code: XlsxSchemaWarningCode::SampledResult,
                message: "schema inference used a row sample; results are approximate".to_owned(),
                column_index: None,
            });
        }
        let columns = self
            .columns
            .into_iter()
            .enumerate()
            .map(|(index, column)| XlsxSchemaColumn {
                column_index: index,
                name: excel_column_name(index),
                logical_type: column.logical_type,
                nullable: column.observed_types.contains(&XlsxLogicalType::Null),
                observed_types: column.observed_types,
            })
            .collect();

        XlsxSchemaReport {
            schema_version: XLSX_SCHEMA_VERSION,
            experimental: true,
            file,
            sheet_name,
            sheet_index,
            scan: XlsxSchemaScan {
                mode,
                sample_rows: self.sample_rows,
                examined_rows: self.examined_rows,
            },
            header_policy: "none",
            columns,
            warnings: self.warnings,
        }
    }
}

#[derive(Debug)]
struct ColumnState {
    logical_type: XlsxLogicalType,
    observed_types: Vec<XlsxLogicalType>,
    warned_conflict: bool,
    warned_fallback: bool,
}

impl ColumnState {
    fn new(has_prior_nulls: bool) -> Self {
        let mut observed_types = Vec::new();
        if has_prior_nulls {
            observed_types.push(XlsxLogicalType::Null);
        }
        Self {
            logical_type: XlsxLogicalType::Null,
            observed_types,
            warned_conflict: false,
            warned_fallback: false,
        }
    }

    fn observe(&mut self, observed: XlsxLogicalType) {
        self.logical_type = promote_types(self.logical_type, observed);
        if !self.observed_types.contains(&observed) {
            self.observed_types.push(observed);
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Classification {
    logical_type: XlsxLogicalType,
    fallback: Option<&'static str>,
}

impl Classification {
    fn plain(logical_type: XlsxLogicalType) -> Self {
        Self {
            logical_type,
            fallback: None,
        }
    }

    fn fallback(reason: &'static str) -> Self {
        Self {
            logical_type: XlsxLogicalType::Utf8,
            fallback: Some(reason),
        }
    }
}

fn classify_cell(value: &XlsxCellValue) -> Classification {
    match value {
        XlsxCellValue::Blank => Classification::plain(XlsxLogicalType::Null),
        XlsxCellValue::String { .. } => Classification::plain(XlsxLogicalType::Utf8),
        XlsxCellValue::Boolean { value: Some(_), .. } => {
            Classification::plain(XlsxLogicalType::Bool)
        }
        XlsxCellValue::Boolean { value: None, .. } => {
            Classification::fallback("invalid boolean encoding")
        }
        XlsxCellValue::Number {
            formatted: Some(formatted),
            ..
        } if is_iso_datetime(formatted) => Classification::plain(XlsxLogicalType::Datetime),
        XlsxCellValue::Number {
            formatted: Some(formatted),
            ..
        } if is_iso_date(formatted) => Classification::plain(XlsxLogicalType::Date),
        XlsxCellValue::Number {
            formatted: Some(formatted),
            ..
        } if is_iso_time(formatted) => Classification::plain(XlsxLogicalType::Time),
        XlsxCellValue::Number { raw, .. } if raw.parse::<i64>().is_ok() => {
            Classification::plain(XlsxLogicalType::Int64)
        }
        XlsxCellValue::Number { raw, .. }
            if raw.parse::<f64>().is_ok_and(|number| number.is_finite()) =>
        {
            Classification::plain(XlsxLogicalType::Float64)
        }
        XlsxCellValue::Number { .. } => Classification::fallback("invalid numeric encoding"),
        XlsxCellValue::Error { .. } => Classification::fallback("worksheet error value"),
        _ => Classification::fallback("unsupported cell value"),
    }
}

fn promote_types(left: XlsxLogicalType, right: XlsxLogicalType) -> XlsxLogicalType {
    if left == XlsxLogicalType::Null {
        return right;
    }
    if right == XlsxLogicalType::Null || left == right {
        return left;
    }
    if matches!(
        (left, right),
        (XlsxLogicalType::Int64, XlsxLogicalType::Float64)
            | (XlsxLogicalType::Float64, XlsxLogicalType::Int64)
    ) {
        return XlsxLogicalType::Float64;
    }
    XlsxLogicalType::Utf8
}

fn types_are_compatible(left: XlsxLogicalType, right: XlsxLogicalType) -> bool {
    left == XlsxLogicalType::Null
        || right == XlsxLogicalType::Null
        || left == right
        || matches!(
            (left, right),
            (XlsxLogicalType::Int64, XlsxLogicalType::Float64)
                | (XlsxLogicalType::Float64, XlsxLogicalType::Int64)
        )
}

fn is_iso_date(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| matches!(index, 4 | 7) || byte.is_ascii_digit())
}

fn is_iso_time(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 8
        && bytes[2] == b':'
        && bytes[5] == b':'
        && bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| matches!(index, 2 | 5) || byte.is_ascii_digit())
}

fn is_iso_datetime(value: &str) -> bool {
    value.len() == 19
        && value.as_bytes()[10] == b'T'
        && is_iso_date(&value[..10])
        && is_iso_time(&value[11..])
}

fn excel_column_name(mut index: usize) -> String {
    let mut reversed = Vec::new();
    loop {
        reversed.push(b'A' + (index % 26) as u8);
        if index < 26 {
            break;
        }
        index = index / 26 - 1;
    }
    reversed.reverse();
    String::from_utf8(reversed).expect("Excel column names are ASCII")
}

#[cfg(test)]
mod tests {
    use oxdoc_core::XlsxCell;
    use serde_json::json;

    use super::*;

    #[test]
    fn infers_all_logical_types_and_numeric_promotion() {
        let mut inferrer = XlsxSchemaInferrer::full_scan();
        let first = row(vec![
            blank(0),
            boolean(1, Some(true)),
            number(2, "42", None),
            number(3, "1.5", None),
            number(4, "44927", Some("2023-01-01")),
            number(5, "0.5", Some("12:00:00")),
            number(6, "44927.5", Some("2023-01-01T12:00:00")),
            string(7, "hello"),
        ]);
        let second = row(vec![number(2, "42.5", None)]);

        assert_eq!(inferrer.observe_row(&first), XlsxRowControl::Continue);
        assert_eq!(inferrer.observe_row(&second), XlsxRowControl::Continue);

        let report = finish(inferrer);
        assert_eq!(report.scan.examined_rows, 2);
        assert_eq!(
            report
                .columns
                .iter()
                .map(|column| column.logical_type)
                .collect::<Vec<_>>(),
            vec![
                XlsxLogicalType::Null,
                XlsxLogicalType::Bool,
                XlsxLogicalType::Float64,
                XlsxLogicalType::Float64,
                XlsxLogicalType::Date,
                XlsxLogicalType::Time,
                XlsxLogicalType::Datetime,
                XlsxLogicalType::Utf8,
            ]
        );
        assert_eq!(
            report.columns[2].observed_types,
            vec![XlsxLogicalType::Int64, XlsxLogicalType::Float64]
        );
        assert!(report.columns[0].nullable);
        assert!(report.columns[1].nullable);
        assert!(!report.columns[2].nullable);
        assert!(report.columns[3..].iter().all(|column| column.nullable));
        assert!(report.warnings.is_empty());
    }

    #[test]
    fn backfills_sparse_and_late_columns_with_null() {
        let mut inferrer = XlsxSchemaInferrer::full_scan();
        inferrer.observe_row(&row(vec![number(0, "1", None)]));
        inferrer.observe_row(&XlsxRow {
            row_index: 1,
            cells: vec![],
        });
        inferrer.observe_row(&row_with_index(2, vec![boolean(2, Some(false))]));

        let report = finish(inferrer);
        assert_eq!(
            report
                .columns
                .iter()
                .map(|column| column.name.as_str())
                .collect::<Vec<_>>(),
            vec!["A", "B", "C"]
        );
        assert_eq!(
            report.columns[1].observed_types,
            vec![XlsxLogicalType::Null]
        );
        assert_eq!(
            report.columns[2].observed_types,
            vec![XlsxLogicalType::Null, XlsxLogicalType::Bool]
        );
        assert!(report.columns.iter().all(|column| column.nullable));
    }

    #[test]
    fn reports_late_conflict_and_promotes_to_utf8() {
        let mut inferrer = XlsxSchemaInferrer::full_scan();
        inferrer.observe_row(&row(vec![number(0, "1", None)]));
        inferrer.observe_row(&row_with_index(4, vec![boolean(0, Some(true))]));

        let report = finish(inferrer);
        assert_eq!(report.columns[0].logical_type, XlsxLogicalType::Utf8);
        assert_eq!(
            report.columns[0].observed_types,
            vec![XlsxLogicalType::Int64, XlsxLogicalType::Bool]
        );
        assert_eq!(report.warnings.len(), 1);
        assert_eq!(report.warnings[0].code, XlsxSchemaWarningCode::TypeConflict);
        assert_eq!(report.warnings[0].column_index, Some(0));
        assert!(report.warnings[0].message.contains("worksheet row 4"));
    }

    #[test]
    fn warns_once_per_column_for_utf8_fallbacks() {
        let mut inferrer = XlsxSchemaInferrer::full_scan();
        inferrer.observe_row(&row(vec![
            number(0, "NaN", None),
            XlsxCell {
                column_index: 1,
                value: XlsxCellValue::Boolean {
                    raw: "maybe".to_owned(),
                    value: None,
                },
                has_formula: false,
            },
        ]));
        inferrer.observe_row(&row_with_index(1, vec![number(0, "Infinity", None)]));

        let report = finish(inferrer);
        assert_eq!(report.warnings.len(), 2);
        assert!(
            report
                .warnings
                .iter()
                .all(|warning| warning.code == XlsxSchemaWarningCode::Utf8Fallback)
        );
        assert!(
            report
                .columns
                .iter()
                .all(|column| column.logical_type == XlsxLogicalType::Utf8)
        );
    }

    #[test]
    fn ignores_non_iso_formatted_numbers_for_date_classification() {
        let mut inferrer = XlsxSchemaInferrer::full_scan();
        inferrer.observe_row(&row(vec![
            number(0, "42", Some("$42.00")),
            number(1, "0.5", Some("12:00")),
            number(2, "44927", Some("01/01/2023")),
        ]));

        let report = finish(inferrer);
        assert_eq!(report.columns[0].logical_type, XlsxLogicalType::Int64);
        assert_eq!(report.columns[1].logical_type, XlsxLogicalType::Float64);
        assert_eq!(report.columns[2].logical_type, XlsxLogicalType::Int64);
    }

    #[test]
    fn stops_after_the_requested_number_of_examined_rows() {
        let mut inferrer = XlsxSchemaInferrer::sampled(2);
        assert_eq!(
            inferrer.observe_row(&row_with_index(10, vec![number(0, "1", None)])),
            XlsxRowControl::Continue
        );
        assert_eq!(
            inferrer.observe_row(&row_with_index(11, vec![number(0, "2", None)])),
            XlsxRowControl::Stop
        );
        assert_eq!(
            inferrer.observe_row(&row_with_index(12, vec![string(0, "late")])),
            XlsxRowControl::Stop
        );

        let report = finish(inferrer);
        assert_eq!(report.scan.mode, XlsxSchemaScanMode::Sampled);
        assert_eq!(report.scan.sample_rows, Some(2));
        assert_eq!(report.scan.examined_rows, 2);
        assert_eq!(report.columns[0].logical_type, XlsxLogicalType::Int64);
        assert_eq!(
            report.warnings.last().unwrap().code,
            XlsxSchemaWarningCode::SampledResult
        );
        assert_eq!(report.warnings.last().unwrap().column_index, None);
    }

    #[test]
    fn zero_row_sample_stops_without_examining_input() {
        let mut inferrer = XlsxSchemaInferrer::sampled(0);
        assert_eq!(
            inferrer.observe_row(&row(vec![number(0, "1", None)])),
            XlsxRowControl::Stop
        );

        let report = finish(inferrer);
        assert_eq!(report.scan.examined_rows, 0);
        assert!(report.columns.is_empty());
    }

    #[test]
    fn serializes_the_documented_report_contract() {
        let mut inferrer = XlsxSchemaInferrer::full_scan();
        inferrer.observe_row(&row(vec![number(0, "1", None)]));
        inferrer.observe_row(&row_with_index(1, vec![number(0, "1.5", None)]));

        let serialized = serde_json::to_value(inferrer.finish(
            "book.xlsx".to_owned(),
            Some("Data".to_owned()),
            None,
        ))
        .unwrap();
        assert_eq!(serialized["schema_version"], json!(1));
        assert_eq!(serialized["experimental"], json!(true));
        assert_eq!(serialized["file"], json!("book.xlsx"));
        assert_eq!(serialized["sheet_name"], json!("Data"));
        assert!(serialized.get("sheet_index").is_none());
        assert_eq!(serialized["scan"]["mode"], json!("full"));
        assert_eq!(serialized["scan"]["examined_rows"], json!(2));
        assert!(serialized["scan"].get("sample_rows").is_none());
        assert_eq!(serialized["header_policy"], json!("none"));
        assert_eq!(serialized["columns"][0]["column_index"], json!(0));
        assert_eq!(
            serialized["columns"][0]["observed_types"],
            json!(["int64", "float64"])
        );
    }

    #[test]
    fn generates_excel_column_names() {
        assert_eq!(excel_column_name(0), "A");
        assert_eq!(excel_column_name(25), "Z");
        assert_eq!(excel_column_name(26), "AA");
        assert_eq!(excel_column_name(51), "AZ");
        assert_eq!(excel_column_name(52), "BA");
        assert_eq!(excel_column_name(16_383), "XFD");
    }

    fn row(cells: Vec<XlsxCell>) -> XlsxRow {
        row_with_index(0, cells)
    }

    fn finish(inferrer: XlsxSchemaInferrer) -> XlsxSchemaReport {
        inferrer.finish("book.xlsx".to_owned(), None, None)
    }

    fn row_with_index(row_index: usize, cells: Vec<XlsxCell>) -> XlsxRow {
        XlsxRow { row_index, cells }
    }

    fn blank(column_index: usize) -> XlsxCell {
        XlsxCell {
            column_index,
            value: XlsxCellValue::Blank,
            has_formula: false,
        }
    }

    fn string(column_index: usize, value: &str) -> XlsxCell {
        XlsxCell {
            column_index,
            value: XlsxCellValue::String {
                raw: value.to_owned(),
                value: value.to_owned(),
            },
            has_formula: false,
        }
    }

    fn boolean(column_index: usize, value: Option<bool>) -> XlsxCell {
        XlsxCell {
            column_index,
            value: XlsxCellValue::Boolean {
                raw: value
                    .map_or_else(|| "invalid".to_owned(), |value| u8::from(value).to_string()),
                value,
            },
            has_formula: false,
        }
    }

    fn number(column_index: usize, raw: &str, formatted: Option<&str>) -> XlsxCell {
        XlsxCell {
            column_index,
            value: XlsxCellValue::Number {
                raw: raw.to_owned(),
                formatted: formatted.map(str::to_owned),
            },
            has_formula: false,
        }
    }
}
