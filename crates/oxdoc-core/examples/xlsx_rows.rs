use std::env;

use oxdoc_core::{XlsxRowControl, XlsxSheetOptions, XlsxValueMode};

fn main() -> oxdoc_core::Result<()> {
    let path = env::args_os()
        .nth(1)
        .expect("usage: cargo run -p oxdoc-core --example xlsx_rows -- <file.xlsx>");

    let extraction = oxdoc_core::visit_xlsx_rows(
        path,
        XlsxSheetOptions::default(),
        XlsxValueMode::Raw,
        |row| {
            println!("row {}: {} present cells", row.row_index, row.cells.len());
            Ok(XlsxRowControl::Continue)
        },
    )?;

    for warning in extraction.warnings {
        eprintln!(
            "warning[{}/{}]: {}: {}",
            warning.category().as_str(),
            warning.code().as_str(),
            warning.path,
            warning.message
        );
    }

    Ok(())
}
