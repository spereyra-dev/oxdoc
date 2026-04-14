use std::env;

fn main() -> oxdoc_core::Result<()> {
    let path = env::args_os()
        .nth(1)
        .expect("usage: cargo run -p oxdoc-core --example extract_text -- <file.docx>");

    let extraction = oxdoc_core::extract_docx_text(path)?;
    print!("{}", extraction.value);

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
