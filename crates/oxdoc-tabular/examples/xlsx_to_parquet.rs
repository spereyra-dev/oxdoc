use std::fs::File;

use oxdoc_core::XlsxSheetOptions;
use oxdoc_tabular::{Column, TabularSchema, TabularType, write_xlsx_parquet};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args_os().skip(1);
    let input = args
        .next()
        .ok_or("usage: xlsx_to_parquet INPUT.xlsx OUTPUT.parquet")?;
    let output = args
        .next()
        .ok_or("usage: xlsx_to_parquet INPUT.xlsx OUTPUT.parquet")?;
    if args.next().is_some() {
        return Err("usage: xlsx_to_parquet INPUT.xlsx OUTPUT.parquet".into());
    }

    let schema = TabularSchema::new(vec![
        Column::new("id", 0, TabularType::Int64, false),
        Column::new("name", 1, TabularType::Utf8, true),
        Column::new("active", 2, TabularType::Bool, true),
    ])?;
    let output = File::create(output)?;
    let extraction =
        write_xlsx_parquet(input, XlsxSheetOptions::default(), &schema, 8_192, output)?;

    eprintln!(
        "wrote {} rows in {} batches and {} row groups",
        extraction.value.rows, extraction.value.batches, extraction.value.row_groups
    );
    for warning in extraction.warnings {
        eprintln!("warning: {warning:?}");
    }
    Ok(())
}
