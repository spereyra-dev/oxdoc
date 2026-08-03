use std::fs::File;
use std::time::Instant;

use oxdoc_core::{XlsxRowControl, XlsxSheetOptions, XlsxValueMode};
use oxdoc_tabular::{
    Column, TabularSchema, TabularType, visit_xlsx_record_batches, write_xlsx_parquet,
};
use serde::Serialize;

#[derive(Serialize)]
struct GateReport {
    rows: usize,
    typed_rows_per_second: f64,
    arrow_rows_per_second: f64,
    parquet_rows_per_second: f64,
    arrow_ratio: f64,
    parquet_ratio: f64,
    batches: usize,
    row_groups: usize,
    gates_passed: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args_os().skip(1);
    let input = args
        .next()
        .ok_or("usage: tabular_gate INPUT.xlsx OUTPUT.parquet EXPECTED_ROWS")?;
    let output = args
        .next()
        .ok_or("usage: tabular_gate INPUT.xlsx OUTPUT.parquet EXPECTED_ROWS")?;
    let expected_rows: usize = args
        .next()
        .ok_or("usage: tabular_gate INPUT.xlsx OUTPUT.parquet EXPECTED_ROWS")?
        .to_string_lossy()
        .parse()?;
    if args.next().is_some() {
        return Err("usage: tabular_gate INPUT.xlsx OUTPUT.parquet EXPECTED_ROWS".into());
    }

    let schema = TabularSchema::new(vec![
        Column::new("id", 0, TabularType::Int64, false),
        Column::new("name", 1, TabularType::Utf8, false),
        Column::new("active", 2, TabularType::Bool, false),
    ])?;

    let mut typed_rows = 0;
    let started = Instant::now();
    oxdoc_core::visit_xlsx_rows(
        &input,
        XlsxSheetOptions::default(),
        XlsxValueMode::Formatted,
        |_| {
            typed_rows += 1;
            Ok(XlsxRowControl::Continue)
        },
    )?;
    let typed_seconds = started.elapsed().as_secs_f64();

    let started = Instant::now();
    let arrow =
        visit_xlsx_record_batches(&input, XlsxSheetOptions::default(), &schema, 8_192, |_| {
            Ok(())
        })?;
    let arrow_seconds = started.elapsed().as_secs_f64();

    let started = Instant::now();
    let parquet = write_xlsx_parquet(
        &input,
        XlsxSheetOptions::default(),
        &schema,
        8_192,
        File::create(output)?,
    )?;
    let parquet_seconds = started.elapsed().as_secs_f64();

    if typed_rows != expected_rows
        || arrow.value.rows != expected_rows
        || parquet.value.rows != expected_rows
    {
        return Err(format!(
            "row count mismatch: expected {expected_rows}, typed={typed_rows}, arrow={}, parquet={}",
            arrow.value.rows, parquet.value.rows
        )
        .into());
    }

    let typed_rate = expected_rows as f64 / typed_seconds;
    let arrow_rate = expected_rows as f64 / arrow_seconds;
    let parquet_rate = expected_rows as f64 / parquet_seconds;
    let arrow_ratio = arrow_rate / typed_rate;
    let parquet_ratio = parquet_rate / typed_rate;
    let report = GateReport {
        rows: expected_rows,
        typed_rows_per_second: typed_rate,
        arrow_rows_per_second: arrow_rate,
        parquet_rows_per_second: parquet_rate,
        arrow_ratio,
        parquet_ratio,
        batches: parquet.value.batches,
        row_groups: parquet.value.row_groups,
        gates_passed: arrow_ratio >= 0.70 && parquet_ratio >= 0.40,
    };
    println!("{}", serde_json::to_string(&report)?);
    if !report.gates_passed {
        return Err("tabular throughput ratio gate failed".into());
    }
    Ok(())
}
