use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};
use oxdoc_core::{DocumentInfo, OutputWarning, OxdocError, XlsxCsvOptions};

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
            eprintln!("error[{}]: {err}", err.code());
            ExitCode::from(1)
        }
    }
}

fn run() -> Result<(), CliError> {
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

#[derive(Debug)]
enum CliError {
    Core(OxdocError),
    InvalidArgument(String),
    Io(std::io::Error),
    Json(serde_json::Error),
}

impl CliError {
    fn code(&self) -> &'static str {
        match self {
            CliError::Core(err) => err.code().as_str(),
            CliError::InvalidArgument(_) => "E010",
            CliError::Io(_) => "E011",
            CliError::Json(_) => "E012",
        }
    }
}

impl std::fmt::Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CliError::Core(err) => write!(f, "{err}"),
            CliError::InvalidArgument(message) => write!(f, "{message}"),
            CliError::Io(err) => write!(f, "{err}"),
            CliError::Json(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for CliError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            CliError::Core(err) => Some(err),
            CliError::InvalidArgument(_) => None,
            CliError::Io(err) => Some(err),
            CliError::Json(err) => Some(err),
        }
    }
}

impl From<OxdocError> for CliError {
    fn from(value: OxdocError) -> Self {
        CliError::Core(value)
    }
}

impl From<serde_json::Error> for CliError {
    fn from(value: serde_json::Error) -> Self {
        CliError::Json(value)
    }
}

impl From<std::io::Error> for CliError {
    fn from(value: std::io::Error) -> Self {
        CliError::Io(value)
    }
}

#[derive(Debug, serde::Serialize)]
struct TextPayload {
    file: String,
    text: String,
}

fn parse_delimiter(value: &str) -> Result<u8, CliError> {
    let bytes = value.as_bytes();
    if bytes.len() == 1 {
        Ok(bytes[0])
    } else {
        Err(CliError::InvalidArgument(
            "delimiter must be a single-byte character".to_owned(),
        ))
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
        eprintln!(
            "warning[{}/{}]: {}: {}",
            warning.category().as_str(),
            warning.code().as_str(),
            warning.path,
            warning.message
        );
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

#[cfg(test)]
mod tests {
    use std::error::Error;

    use super::CliError;

    #[test]
    fn cli_errors_expose_stable_codes_and_sources() {
        let core = CliError::from(oxdoc_core::OxdocError::MissingPart(
            "word/document.xml".to_owned(),
        ));
        let invalid = CliError::InvalidArgument("bad delimiter".to_owned());
        let io = CliError::from(std::io::Error::other("disk"));
        let json = CliError::from(serde_json::from_str::<serde_json::Value>("{").unwrap_err());

        assert_eq!(core.code(), "E003");
        assert!(core.source().is_some());
        assert_eq!(invalid.code(), "E010");
        assert_eq!(format!("{invalid}"), "bad delimiter");
        assert!(invalid.source().is_none());
        assert_eq!(io.code(), "E011");
        assert_eq!(format!("{io}"), "disk");
        assert!(io.source().is_some());
        assert_eq!(json.code(), "E012");
        assert!(format!("{json}").contains("EOF"));
        assert!(json.source().is_some());
    }
}
