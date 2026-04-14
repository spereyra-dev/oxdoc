use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};
use oxdoc_core::{DocumentInfo, OutputWarning, XlsxCsvOptions};

#[derive(Debug, Parser)]
#[command(
    name = "oxdoc",
    version,
    about = "Fast OOXML text, CSV, and metadata extractor"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Extract {
        #[command(subcommand)]
        command: ExtractCommand,
    },
    Info {
        file: PathBuf,
        #[arg(long, value_enum, default_value_t = InfoFormat::Json)]
        format: InfoFormat,
    },
}

#[derive(Debug, Subcommand)]
enum ExtractCommand {
    Text {
        file: PathBuf,
        #[arg(long, value_enum, default_value_t = TextFormat::Text)]
        format: TextFormat,
    },
    Csv {
        file: PathBuf,
        #[arg(long)]
        sheet: Option<String>,
        #[arg(long, default_value = ",")]
        delimiter: String,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum TextFormat {
    Text,
    Json,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum InfoFormat {
    Text,
    Json,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Command::Extract { command } => match command {
            ExtractCommand::Text { file, format } => {
                let result = oxdoc_core::extract_docx_text(&file)?;
                emit_warnings(&result.warnings);
                match format {
                    TextFormat::Text => {
                        print!("{}", result.value);
                    }
                    TextFormat::Json => {
                        let payload = TextPayload {
                            file: display_file_name(&file),
                            text: result.value,
                        };
                        serde_json::to_writer_pretty(io::stdout().lock(), &payload)?;
                        println!();
                    }
                }
            }
            ExtractCommand::Csv {
                file,
                sheet,
                delimiter,
            } => {
                let delimiter = parse_delimiter(&delimiter)?;
                let mut stdout = io::stdout().lock();
                let result = oxdoc_core::extract_xlsx_csv(
                    &file,
                    XlsxCsvOptions {
                        sheet_name: sheet.as_deref(),
                        delimiter,
                    },
                    &mut stdout,
                )?;
                stdout.flush()?;
                emit_warnings(&result.warnings);
            }
        },
        Command::Info { file, format } => {
            let result = oxdoc_core::read_info(&file)?;
            emit_warnings(&result.warnings);
            match format {
                InfoFormat::Json => {
                    serde_json::to_writer_pretty(io::stdout().lock(), &result.value)?;
                    println!();
                }
                InfoFormat::Text => print_info(&result.value),
            }
        }
    }

    Ok(())
}

#[derive(Debug, serde::Serialize)]
struct TextPayload {
    file: String,
    text: String,
}

fn parse_delimiter(value: &str) -> Result<u8, String> {
    let bytes = value.as_bytes();
    if bytes.len() == 1 {
        Ok(bytes[0])
    } else {
        Err("delimiter must be a single-byte character".to_owned())
    }
}

fn display_file_name(path: &std::path::Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_owned()
}

fn emit_warnings(warnings: &[OutputWarning]) {
    for warning in warnings {
        eprintln!("warning: {}: {}", warning.path, warning.message);
    }
}

fn print_info(info: &DocumentInfo) {
    println!("file: {}", info.file);
    print_optional("author", info.author.as_deref());
    print_optional("last_modified_by", info.last_modified_by.as_deref());
    print_optional("created_at", info.created_at.as_deref());
    print_optional("modified_at", info.modified_at.as_deref());
    print_optional("application", info.application.as_deref());
    print_optional("company", info.company.as_deref());
    println!("has_macros: {}", info.has_macros);
    print_optional_u64("word_count", info.word_count);
    print_optional_u64("page_count", info.page_count);
    print_optional_u64("slide_count", info.slide_count);
    print_optional_u64("worksheet_count", info.worksheet_count);
    print_optional("revision", info.revision.as_deref());
}

fn print_optional(label: &str, value: Option<&str>) {
    if let Some(value) = value {
        println!("{label}: {value}");
    }
}

fn print_optional_u64(label: &str, value: Option<u64>) {
    if let Some(value) = value {
        println!("{label}: {value}");
    }
}
