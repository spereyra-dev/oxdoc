use std::env;
use std::io::{self, Write};

use oxdoc_core::XlsxCsvOptions;

fn main() -> oxdoc_core::Result<()> {
    let path = env::args_os()
        .nth(1)
        .expect("usage: cargo run -p oxdoc-core --example xlsx_csv -- <file.xlsx>");
    let mut stdout = io::stdout().lock();

    let extraction = oxdoc_core::extract_xlsx_csv(path, XlsxCsvOptions::default(), &mut stdout)?;
    stdout.flush()?;

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
