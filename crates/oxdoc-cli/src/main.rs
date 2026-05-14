use std::fs::File;
use std::io::{self, Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};
use oxdoc_core::{DocumentInfo, DocumentType, OutputWarning, OxdocError, XlsxCsvOptions};

mod update;

#[derive(Debug, Parser)]
#[command(
    name = "oxdoc",
    version,
    about = "Fast OOXML text, CSV, and metadata extractor"
)]
struct Cli {
    #[arg(long, short, global = true)]
    quiet: bool,
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
    /// Check for a newer release and install it
    Update {
        /// Only check if an update is available; do not download or install
        #[arg(long)]
        check: bool,
        /// Install a specific version instead of the latest (e.g. v0.2.0)
        #[arg(long, value_name = "VERSION")]
        version: Option<String>,
    },
}

#[derive(Debug, Subcommand)]
enum ExtractCommand {
    Text {
        #[arg(required = true)]
        files: Vec<PathBuf>,
        #[arg(long, value_enum, default_value_t = TextFormat::Text)]
        format: TextFormat,
        #[arg(long, short)]
        output: Option<PathBuf>,
    },
    Csv {
        #[arg(required = true)]
        files: Vec<PathBuf>,
        #[arg(long, help = "Visible workbook sheet name to extract")]
        sheet: Option<String>,
        #[arg(
            long,
            conflicts_with_all = ["sheet", "list_sheets"],
            help = "1-based visible workbook sheet index to extract"
        )]
        sheet_index: Option<usize>,
        #[arg(
            long,
            conflicts_with_all = ["sheet", "sheet_index"],
            help = "List visible workbook sheets and exit"
        )]
        list_sheets: bool,
        #[arg(long, default_value = ",")]
        delimiter: String,
        #[arg(long, short)]
        output: Option<PathBuf>,
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
            ExtractCommand::Text {
                files,
                format,
                output,
            } => {
                extract_text_command(&files, format, output.as_deref(), cli.quiet)?;
            }
            ExtractCommand::Csv {
                files,
                sheet,
                sheet_index,
                list_sheets,
                delimiter,
                output,
            } => {
                let delimiter = parse_delimiter(&delimiter)?;
                extract_csv_command(
                    &files,
                    sheet.as_deref(),
                    sheet_index,
                    list_sheets,
                    delimiter,
                    output.as_deref(),
                    cli.quiet,
                )?;
            }
        },
        Command::Info { file, format } => {
            let result = read_info(&file)?;
            emit_warnings(&result.warnings, cli.quiet);
            match format {
                InfoFormat::Json => {
                    let payload = InfoPayload {
                        oxdoc_version: env!("CARGO_PKG_VERSION"),
                        info: &result.value,
                    };
                    serde_json::to_writer_pretty(io::stdout().lock(), &payload)?;
                    println!();
                }
                InfoFormat::Text => print_info(&result.value),
            }
        }
        Command::Update { check, version } => {
            match update::run(check, version).map_err(CliError::Update)? {
                update::UpdateOutcome::AlreadyUpToDate { version } => {
                    println!("oxdoc {version} is already up to date.");
                }
                update::UpdateOutcome::UpdateAvailable { current, latest } => {
                    println!("Update available: {current} → {latest}");
                    println!("Run `oxdoc update` to install.");
                }
                update::UpdateOutcome::Updated { from, to } => {
                    println!("Updated oxdoc from {from} to {to}.");
                }
            }
        }
    }

    Ok(())
}

fn extract_text_command(
    files: &[PathBuf],
    format: TextFormat,
    output: Option<&Path>,
    quiet: bool,
) -> Result<(), CliError> {
    let multiple = files.len() > 1;
    let mut writer = output_writer(output)?;
    let mut payloads = Vec::new();
    let mut wrote_text = false;

    for file in files {
        match extract_text(file) {
            Ok(result) => {
                emit_warnings(&result.warnings, quiet);
                match format {
                    TextFormat::Text => {
                        if wrote_text {
                            writeln!(writer, "\n---")?;
                        }
                        write!(writer, "{}", result.value)?;
                        wrote_text = true;
                    }
                    TextFormat::Json => payloads.push(TextPayload {
                        file: display_file_name(file),
                        text: result.value,
                    }),
                }
            }
            Err(err) if multiple => emit_skipped_input_warning(file, &err, quiet),
            Err(err) => return Err(err),
        }
    }

    if matches!(format, TextFormat::Json) {
        if multiple {
            serde_json::to_writer_pretty(&mut writer, &payloads)?;
        } else if let Some(payload) = payloads.into_iter().next() {
            serde_json::to_writer_pretty(&mut writer, &payload)?;
        } else {
            return Err(CliError::InvalidArgument(
                "no input files were processed successfully".to_owned(),
            ));
        }
        writeln!(writer)?;
    } else if !wrote_text {
        return Err(CliError::InvalidArgument(
            "no input files were processed successfully".to_owned(),
        ));
    }

    writer.flush()?;
    Ok(())
}

fn extract_csv_command(
    files: &[PathBuf],
    sheet_name: Option<&str>,
    sheet_index: Option<usize>,
    list_sheets: bool,
    delimiter: u8,
    output: Option<&Path>,
    quiet: bool,
) -> Result<(), CliError> {
    if list_sheets && files.len() > 1 {
        return Err(CliError::InvalidArgument(
            "--list-sheets supports a single input file".to_owned(),
        ));
    }

    let multiple = files.len() > 1;
    let mut writer = output_writer(output)?;
    let mut processed = 0usize;

    for file in files {
        let result = if list_sheets {
            match list_xlsx_sheets(file) {
                Ok(sheets) => {
                    for sheet in &sheets.value {
                        writeln!(writer, "{}: {}", sheet.index, sheet.name)?;
                    }
                    Ok(sheets.map(|_| ()))
                }
                Err(err) => Err(err),
            }
        } else {
            extract_csv(
                file,
                XlsxCsvOptions {
                    sheet_name,
                    sheet_index,
                    delimiter,
                },
                &mut writer,
            )
        };

        match result {
            Ok(result) => {
                processed += 1;
                emit_warnings(&result.warnings, quiet);
            }
            Err(err) if multiple => emit_skipped_input_warning(file, &err, quiet),
            Err(err) => return Err(err),
        }
    }

    if processed == 0 {
        return Err(CliError::InvalidArgument(
            "no input files were processed successfully".to_owned(),
        ));
    }

    writer.flush()?;
    Ok(())
}

fn extract_text(file: &Path) -> Result<oxdoc_core::Extraction<String>, CliError> {
    let input = read_input(file)?;
    match document_type_for_input(&input, file)? {
        DocumentType::Pptx => input.extract_pptx_text().map_err(CliError::Core),
        DocumentType::Docx | DocumentType::Unknown => {
            input.extract_docx_text().map_err(CliError::Core)
        }
        DocumentType::Xlsx => Err(CliError::InvalidArgument(
            "cannot extract text from an XLSX workbook".to_owned(),
        )),
    }
}

fn extract_csv<W: Write>(
    file: &Path,
    options: XlsxCsvOptions<'_>,
    writer: W,
) -> Result<oxdoc_core::Extraction<()>, CliError> {
    let input = read_input(file)?;
    input
        .extract_xlsx_csv(options, writer)
        .map_err(CliError::Core)
}

fn list_xlsx_sheets(
    file: &Path,
) -> Result<oxdoc_core::Extraction<Vec<oxdoc_core::XlsxSheet>>, CliError> {
    let input = read_input(file)?;
    input.list_xlsx_sheets().map_err(CliError::Core)
}

fn read_info(file: &Path) -> Result<oxdoc_core::Extraction<DocumentInfo>, CliError> {
    let input = read_input(file)?;
    input
        .read_info(display_file_name(file))
        .map_err(CliError::Core)
}

enum Input {
    Path(PathBuf),
    Stdin(Vec<u8>),
}

impl Input {
    fn extract_docx_text(&self) -> oxdoc_core::Result<oxdoc_core::Extraction<String>> {
        match self {
            Input::Path(path) => oxdoc_core::extract_docx_text(path),
            Input::Stdin(bytes) => oxdoc_core::extract_docx_text_from_reader(Cursor::new(bytes)),
        }
    }

    fn extract_pptx_text(&self) -> oxdoc_core::Result<oxdoc_core::Extraction<String>> {
        match self {
            Input::Path(path) => oxdoc_core::extract_pptx_text(path),
            Input::Stdin(bytes) => oxdoc_core::extract_pptx_text_from_reader(Cursor::new(bytes)),
        }
    }

    fn extract_xlsx_csv<W: Write>(
        &self,
        options: XlsxCsvOptions<'_>,
        writer: W,
    ) -> oxdoc_core::Result<oxdoc_core::Extraction<()>> {
        match self {
            Input::Path(path) => oxdoc_core::extract_xlsx_csv(path, options, writer),
            Input::Stdin(bytes) => {
                oxdoc_core::extract_xlsx_csv_from_reader(Cursor::new(bytes), options, writer)
            }
        }
    }

    fn list_xlsx_sheets(
        &self,
    ) -> oxdoc_core::Result<oxdoc_core::Extraction<Vec<oxdoc_core::XlsxSheet>>> {
        match self {
            Input::Path(path) => oxdoc_core::list_xlsx_sheets(path),
            Input::Stdin(bytes) => oxdoc_core::list_xlsx_sheets_from_reader(Cursor::new(bytes)),
        }
    }

    fn read_info(
        &self,
        file_name: String,
    ) -> oxdoc_core::Result<oxdoc_core::Extraction<DocumentInfo>> {
        match self {
            Input::Path(path) => oxdoc_core::read_info(path),
            Input::Stdin(bytes) => oxdoc_core::read_info_from_reader(Cursor::new(bytes), file_name),
        }
    }

    fn detect_document_type(&self) -> oxdoc_core::Result<DocumentType> {
        match self {
            Input::Path(path) => oxdoc_core::detect_document_type(path),
            Input::Stdin(bytes) => oxdoc_core::detect_document_type_from_reader(Cursor::new(bytes)),
        }
    }
}

fn read_input(file: &Path) -> Result<Input, CliError> {
    if file == Path::new("-") {
        let mut bytes = Vec::new();
        io::stdin().lock().read_to_end(&mut bytes)?;
        Ok(Input::Stdin(bytes))
    } else {
        Ok(Input::Path(file.to_owned()))
    }
}

fn document_type_for_input(input: &Input, file: &Path) -> Result<DocumentType, CliError> {
    let detected = input.detect_document_type()?;
    if detected != DocumentType::Unknown {
        return Ok(detected);
    }

    if file
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("pptx"))
    {
        return Ok(DocumentType::Pptx);
    }

    if file
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("xlsx"))
    {
        return Ok(DocumentType::Xlsx);
    }

    Ok(DocumentType::Docx)
}

enum OutputWriter {
    Stdout(io::Stdout),
    File(File),
}

impl Write for OutputWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            OutputWriter::Stdout(stdout) => stdout.lock().write(buf),
            OutputWriter::File(file) => file.write(buf),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            OutputWriter::Stdout(stdout) => stdout.lock().flush(),
            OutputWriter::File(file) => file.flush(),
        }
    }
}

fn output_writer(path: Option<&Path>) -> Result<OutputWriter, CliError> {
    match path {
        Some(path) => Ok(OutputWriter::File(File::create(path)?)),
        None => Ok(OutputWriter::Stdout(io::stdout())),
    }
}

fn emit_skipped_input_warning(path: &Path, err: &CliError, quiet: bool) {
    if quiet {
        return;
    }

    eprintln!(
        "warning[batch/W998]: {}: skipped after error[{}]: {}",
        display_file_name(path),
        err.code(),
        err
    );
}

#[derive(Debug)]
enum CliError {
    Core(OxdocError),
    InvalidArgument(String),
    Io(std::io::Error),
    Json(serde_json::Error),
    Update(String),
}

impl CliError {
    fn code(&self) -> &'static str {
        match self {
            CliError::Core(err) => err.code().as_str(),
            CliError::InvalidArgument(_) => "E010",
            CliError::Io(_) => "E011",
            CliError::Json(_) => "E012",
            CliError::Update(_) => "E013",
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
            CliError::Update(message) => write!(f, "{message}"),
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
            CliError::Update(_) => None,
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

#[derive(Debug, serde::Serialize)]
struct InfoPayload<'a> {
    oxdoc_version: &'static str,
    #[serde(flatten)]
    info: &'a DocumentInfo,
}

fn parse_delimiter(value: &str) -> Result<u8, CliError> {
    if value == "\\t" {
        return Ok(b'\t');
    }

    if value == "\\n" {
        return Ok(b'\n');
    }

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
    if path == Path::new("-") {
        return "<stdin>".to_owned();
    }

    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_owned()
}

fn emit_warnings(warnings: &[OutputWarning], quiet: bool) {
    if quiet {
        return;
    }

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

    use super::{CliError, parse_delimiter};

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

        let update = CliError::Update("update error".to_owned());
        assert_eq!(update.code(), "E013");
        assert_eq!(format!("{update}"), "update error");
        assert!(update.source().is_none());
    }

    #[test]
    fn test_parse_delimiter() {
        use super::parse_delimiter;
        assert_eq!(parse_delimiter("\\t").unwrap(), b'\t');
        assert_eq!(parse_delimiter(",").unwrap(), b',');
        assert!(parse_delimiter("foo").is_err());
    }

    #[test]
    fn parse_delimiter_handles_supported_and_invalid_values() {
        assert_eq!(parse_delimiter(",").unwrap(), b',');
        assert_eq!(parse_delimiter("\\t").unwrap(), b'\t');
        assert_eq!(parse_delimiter("|").unwrap(), b'|');
        assert_eq!(parse_delimiter(";").unwrap(), b';');

        let empty = parse_delimiter("").unwrap_err();
        assert_eq!(empty.code(), "E010");
        assert_eq!(
            format!("{empty}"),
            "delimiter must be a single-byte character"
        );

        let multi_byte = parse_delimiter("ab").unwrap_err();
        assert_eq!(multi_byte.code(), "E010");
        assert_eq!(
            format!("{multi_byte}"),
            "delimiter must be a single-byte character"
        );
    }

    #[test]
    fn parse_delimiter_supports_newline_escape_sequence() {
        assert_eq!(parse_delimiter("\\n").unwrap(), b'\n');
    }
}
