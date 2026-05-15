use std::fs::{self, File};
use std::io::{self, Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};
use oxdoc_core::{
    AuditSignal, DocumentAudit, DocumentInfo, DocumentType, OutputWarning, OxdocError,
    StructuredText, XlsxCsvOptions, XlsxValueMode,
};

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
    #[arg(long, global = true, value_enum, default_value_t = WarningFormat::Text)]
    warnings: WarningFormat,
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
    Audit {
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
        #[arg(
            long,
            conflicts_with = "all_sheets",
            help = "Visible workbook sheet name to extract"
        )]
        sheet: Option<String>,
        #[arg(
            long,
            conflicts_with_all = ["sheet", "list_sheets", "all_sheets"],
            help = "1-based visible workbook sheet index to extract"
        )]
        sheet_index: Option<usize>,
        #[arg(
            long,
            conflicts_with_all = ["sheet", "sheet_index", "all_sheets", "output_dir"],
            help = "List workbook sheets and exit"
        )]
        list_sheets: bool,
        #[arg(
            long,
            conflicts_with_all = ["sheet", "sheet_index", "list_sheets", "output"],
            requires = "output_dir",
            help = "Export workbook sheets to separate CSV files"
        )]
        all_sheets: bool,
        #[arg(
            long,
            help = "Include hidden and very hidden workbook sheets in listing or extraction"
        )]
        include_hidden: bool,
        #[arg(long, default_value = ",")]
        delimiter: String,
        #[arg(long, value_enum, default_value_t = CliXlsxValueMode::Raw)]
        value_mode: CliXlsxValueMode,
        #[arg(long, short)]
        output: Option<PathBuf>,
        #[arg(long, conflicts_with = "output", requires = "all_sheets")]
        output_dir: Option<PathBuf>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum TextFormat {
    Text,
    Json,
    Jsonl,
    StructuredJson,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum InfoFormat {
    Text,
    Json,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum WarningFormat {
    Text,
    Json,
    None,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum CliXlsxValueMode {
    Raw,
    Formatted,
}

impl From<CliXlsxValueMode> for XlsxValueMode {
    fn from(value: CliXlsxValueMode) -> Self {
        match value {
            CliXlsxValueMode::Raw => XlsxValueMode::Raw,
            CliXlsxValueMode::Formatted => XlsxValueMode::Formatted,
        }
    }
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
    let warning_format = if cli.quiet {
        WarningFormat::None
    } else {
        cli.warnings
    };

    match cli.command {
        Command::Extract { command } => match command {
            ExtractCommand::Text {
                files,
                format,
                output,
            } => {
                extract_text_command(&files, format, output.as_deref(), warning_format)?;
            }
            ExtractCommand::Csv {
                files,
                sheet,
                sheet_index,
                list_sheets,
                all_sheets,
                include_hidden,
                delimiter,
                value_mode,
                output,
                output_dir,
            } => {
                let delimiter = parse_delimiter(&delimiter)?;
                extract_csv_command(
                    CsvCommandOptions {
                        files: &files,
                        sheet_name: sheet.as_deref(),
                        sheet_index,
                        list_sheets,
                        all_sheets,
                        include_hidden,
                        delimiter,
                        value_mode: value_mode.into(),
                        output: output.as_deref(),
                        output_dir: output_dir.as_deref(),
                    },
                    warning_format,
                )?;
            }
        },
        Command::Info { file, format } => {
            let result = read_info(&file)?;
            emit_warnings(&result.warnings, warning_format);
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
        Command::Audit { file, format } => {
            let result = read_audit(&file)?;
            emit_warnings(&result.warnings, warning_format);
            match format {
                InfoFormat::Json => {
                    let payload = AuditPayload {
                        oxdoc_version: env!("CARGO_PKG_VERSION"),
                        audit: &result.value,
                    };
                    serde_json::to_writer_pretty(io::stdout().lock(), &payload)?;
                    println!();
                }
                InfoFormat::Text => print_audit(&result.value),
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
    warning_format: WarningFormat,
) -> Result<(), CliError> {
    let multiple = files.len() > 1;
    let mut writer = output_writer(output)?;
    let mut payloads = Vec::new();
    let mut structured_payloads = Vec::new();
    let mut wrote_text = false;

    for file in files {
        if format == TextFormat::Jsonl {
            let record = extract_text_jsonl_record(file, warning_format);
            serde_json::to_writer(&mut writer, &record)?;
            writeln!(writer)?;
            continue;
        }

        if format == TextFormat::StructuredJson {
            match extract_structured_text(file) {
                Ok(result) => {
                    emit_warnings(&result.warnings, warning_format);
                    structured_payloads.push(TextStructuredPayload {
                        file: display_file_name(file),
                        structured: result.value,
                    });
                }
                Err(err) if multiple => emit_skipped_input_warning(file, &err, warning_format),
                Err(err) => return Err(err),
            }
            continue;
        }

        match extract_text(file) {
            Ok(result) => {
                emit_warnings(&result.warnings, warning_format);
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
                    TextFormat::StructuredJson => unreachable!("handled before extraction"),
                    TextFormat::Jsonl => unreachable!("handled before extraction"),
                }
            }
            Err(err) if multiple => emit_skipped_input_warning(file, &err, warning_format),
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
    } else if matches!(format, TextFormat::StructuredJson) {
        if multiple {
            serde_json::to_writer_pretty(&mut writer, &structured_payloads)?;
        } else if let Some(payload) = structured_payloads.into_iter().next() {
            serde_json::to_writer_pretty(&mut writer, &payload)?;
        } else {
            return Err(CliError::InvalidArgument(
                "no input files were processed successfully".to_owned(),
            ));
        }
        writeln!(writer)?;
    } else if format == TextFormat::Jsonl {
        // JSONL is a batch record contract: each input produces either a success
        // or error line, so per-file extraction failures are represented in stdout.
    } else if !wrote_text {
        return Err(CliError::InvalidArgument(
            "no input files were processed successfully".to_owned(),
        ));
    }

    writer.flush()?;
    Ok(())
}

fn extract_structured_text(
    file: &Path,
) -> Result<oxdoc_core::Extraction<StructuredText>, CliError> {
    let input = read_input(file)?;
    match document_type_for_input(&input, file)? {
        DocumentType::Pptx => input.extract_pptx_structured_text().map_err(CliError::Core),
        DocumentType::Docx | DocumentType::Unknown => {
            input.extract_docx_structured_text().map_err(CliError::Core)
        }
        DocumentType::Xlsx => Err(CliError::InvalidArgument(
            "cannot extract text from an XLSX workbook".to_owned(),
        )),
    }
}

fn extract_text_jsonl_record(file: &Path, warning_format: WarningFormat) -> TextJsonlRecord {
    let file_name = display_file_name(file);
    let input = match read_input(file) {
        Ok(input) => input,
        Err(err) => {
            return TextJsonlRecord::error(file_name, "unknown", err);
        }
    };
    let document_type = match input.detect_document_type() {
        Ok(document_type) => document_type_name(document_type),
        Err(err) => {
            return TextJsonlRecord::error(file_name, "unknown", CliError::Core(err));
        }
    };

    let result = match document_type_for_input(&input, file) {
        Ok(document_type) => match document_type {
            DocumentType::Pptx => input.extract_pptx_text(),
            DocumentType::Docx | DocumentType::Unknown => input.extract_docx_text(),
            DocumentType::Xlsx => Err(OxdocError::InvalidArgument(
                "cannot extract text from an XLSX workbook".to_owned(),
            )),
        },
        Err(err) => {
            return TextJsonlRecord::error(file_name, document_type, err);
        }
    };

    match result {
        Ok(extraction) => {
            emit_warnings(&extraction.warnings, warning_format);
            TextJsonlRecord::success(file_name, document_type, extraction)
        }
        Err(err) => TextJsonlRecord::error(file_name, document_type, CliError::Core(err)),
    }
}

fn document_type_name(document_type: DocumentType) -> &'static str {
    match document_type {
        DocumentType::Docx => "docx",
        DocumentType::Pptx => "pptx",
        DocumentType::Xlsx => "xlsx",
        DocumentType::Unknown => "unknown",
    }
}

struct CsvCommandOptions<'a> {
    files: &'a [PathBuf],
    sheet_name: Option<&'a str>,
    sheet_index: Option<usize>,
    list_sheets: bool,
    all_sheets: bool,
    include_hidden: bool,
    delimiter: u8,
    value_mode: XlsxValueMode,
    output: Option<&'a Path>,
    output_dir: Option<&'a Path>,
}

fn extract_csv_command(
    options: CsvCommandOptions<'_>,
    warning_format: WarningFormat,
) -> Result<(), CliError> {
    if options.list_sheets && options.files.len() > 1 {
        return Err(CliError::InvalidArgument(
            "--list-sheets supports a single input file".to_owned(),
        ));
    }

    if options.all_sheets {
        if options.files.len() > 1 {
            return Err(CliError::InvalidArgument(
                "--all-sheets supports a single input file".to_owned(),
            ));
        }
        let output_dir = options.output_dir.ok_or_else(|| {
            CliError::InvalidArgument("--all-sheets requires --output-dir".to_owned())
        })?;
        return export_all_sheets(
            options.files.first().expect("required by clap"),
            output_dir,
            options.delimiter,
            options.value_mode,
            options.include_hidden,
            warning_format,
        );
    }

    let multiple = options.files.len() > 1;
    let mut writer = output_writer(options.output)?;
    let mut processed = 0usize;

    for file in options.files {
        let result = if options.list_sheets {
            match list_xlsx_sheets(file, options.include_hidden) {
                Ok(sheets) => {
                    for sheet in &sheets.value {
                        if options.include_hidden {
                            writeln!(
                                writer,
                                "{}: {} ({})",
                                sheet.index,
                                sheet.name,
                                sheet.visibility.as_str()
                            )?;
                        } else {
                            writeln!(writer, "{}: {}", sheet.index, sheet.name)?;
                        }
                    }
                    Ok(sheets.map(|_| ()))
                }
                Err(err) => Err(err),
            }
        } else {
            extract_csv(
                file,
                XlsxCsvOptions {
                    sheet_name: options.sheet_name,
                    sheet_index: options.sheet_index,
                    include_hidden: options.include_hidden,
                    delimiter: options.delimiter,
                },
                options.value_mode,
                &mut writer,
            )
        };

        match result {
            Ok(result) => {
                processed += 1;
                emit_warnings(&result.warnings, warning_format);
            }
            Err(err) if multiple => emit_skipped_input_warning(file, &err, warning_format),
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

fn export_all_sheets(
    file: &Path,
    output_dir: &Path,
    delimiter: u8,
    value_mode: XlsxValueMode,
    include_hidden: bool,
    warning_format: WarningFormat,
) -> Result<(), CliError> {
    fs::create_dir_all(output_dir)?;
    let sheets = list_xlsx_sheets(file, include_hidden)?;
    emit_warnings(&sheets.warnings, warning_format);

    let mut manifest = AllSheetsManifest {
        oxdoc_version: env!("CARGO_PKG_VERSION"),
        file: display_file_name(file),
        sheets: Vec::new(),
    };
    let mut failures = 0usize;

    for sheet in sheets.value {
        let csv_file_name = csv_file_name_for_sheet(sheet.index, &sheet.name);
        let csv_path = output_dir.join(&csv_file_name);
        let mut csv_file = File::create(&csv_path)?;
        let result = extract_csv(
            file,
            XlsxCsvOptions {
                sheet_name: None,
                sheet_index: Some(sheet.index),
                include_hidden,
                delimiter,
            },
            value_mode,
            &mut csv_file,
        );

        match result {
            Ok(result) => {
                emit_warnings(&result.warnings, warning_format);
                manifest.sheets.push(ExportedSheetManifest {
                    index: sheet.index,
                    visibility: sheet.visibility.as_str(),
                    name: sheet.name,
                    csv_path: csv_file_name,
                    warnings: result
                        .warnings
                        .iter()
                        .map(ManifestWarning::from_output_warning)
                        .collect(),
                    error: None,
                });
            }
            Err(err) => {
                failures += 1;
                manifest.sheets.push(ExportedSheetManifest {
                    index: sheet.index,
                    visibility: sheet.visibility.as_str(),
                    name: sheet.name,
                    csv_path: csv_file_name,
                    warnings: Vec::new(),
                    error: Some(ManifestError {
                        code: err.code(),
                        message: err.to_string(),
                    }),
                });
            }
        }
    }

    let manifest_path = output_dir.join("manifest.json");
    let mut manifest_file = File::create(manifest_path)?;
    serde_json::to_writer_pretty(&mut manifest_file, &manifest)?;
    writeln!(manifest_file)?;

    if failures > 0 {
        return Err(CliError::InvalidArgument(format!(
            "{failures} sheet export(s) failed; see manifest.json"
        )));
    }

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
    value_mode: XlsxValueMode,
    writer: W,
) -> Result<oxdoc_core::Extraction<()>, CliError> {
    let input = read_input(file)?;
    input
        .extract_xlsx_csv(options, value_mode, writer)
        .map_err(CliError::Core)
}

fn list_xlsx_sheets(
    file: &Path,
    include_hidden: bool,
) -> Result<oxdoc_core::Extraction<Vec<oxdoc_core::XlsxSheet>>, CliError> {
    let input = read_input(file)?;
    input
        .list_xlsx_sheets(include_hidden)
        .map_err(CliError::Core)
}

fn read_info(file: &Path) -> Result<oxdoc_core::Extraction<DocumentInfo>, CliError> {
    let input = read_input(file)?;
    input
        .read_info(display_file_name(file))
        .map_err(CliError::Core)
}

fn read_audit(file: &Path) -> Result<oxdoc_core::Extraction<DocumentAudit>, CliError> {
    let input = read_input(file)?;
    input
        .read_audit(display_file_name(file))
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

    fn extract_docx_structured_text(
        &self,
    ) -> oxdoc_core::Result<oxdoc_core::Extraction<StructuredText>> {
        match self {
            Input::Path(path) => oxdoc_core::extract_docx_structured_text(path),
            Input::Stdin(bytes) => {
                oxdoc_core::extract_docx_structured_text_from_reader(Cursor::new(bytes))
            }
        }
    }

    fn extract_pptx_structured_text(
        &self,
    ) -> oxdoc_core::Result<oxdoc_core::Extraction<StructuredText>> {
        match self {
            Input::Path(path) => oxdoc_core::extract_pptx_structured_text(path),
            Input::Stdin(bytes) => {
                oxdoc_core::extract_pptx_structured_text_from_reader(Cursor::new(bytes))
            }
        }
    }

    fn extract_xlsx_csv<W: Write>(
        &self,
        options: XlsxCsvOptions<'_>,
        value_mode: XlsxValueMode,
        writer: W,
    ) -> oxdoc_core::Result<oxdoc_core::Extraction<()>> {
        match self {
            Input::Path(path) => {
                oxdoc_core::extract_xlsx_csv_with_value_mode(path, options, value_mode, writer)
            }
            Input::Stdin(bytes) => oxdoc_core::extract_xlsx_csv_from_reader_with_value_mode(
                Cursor::new(bytes),
                options,
                value_mode,
                writer,
            ),
        }
    }

    fn list_xlsx_sheets(
        &self,
        include_hidden: bool,
    ) -> oxdoc_core::Result<oxdoc_core::Extraction<Vec<oxdoc_core::XlsxSheet>>> {
        match self {
            Input::Path(path) => oxdoc_core::list_xlsx_sheets_with_hidden(path, include_hidden),
            Input::Stdin(bytes) => oxdoc_core::list_xlsx_sheets_from_reader_with_hidden(
                Cursor::new(bytes),
                include_hidden,
            ),
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

    fn read_audit(
        &self,
        file_name: String,
    ) -> oxdoc_core::Result<oxdoc_core::Extraction<DocumentAudit>> {
        match self {
            Input::Path(path) => oxdoc_core::read_audit(path),
            Input::Stdin(bytes) => {
                oxdoc_core::read_audit_from_reader(Cursor::new(bytes), file_name)
            }
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

fn emit_skipped_input_warning(path: &Path, err: &CliError, format: WarningFormat) {
    let path = display_file_name(path);
    let message = format!("skipped after error[{}]: {}", err.code(), err);
    emit_warning_parts("batch", "W998", &path, &message, format);
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
struct TextStructuredPayload {
    file: String,
    #[serde(flatten)]
    structured: StructuredText,
}

#[derive(Debug, serde::Serialize)]
struct TextJsonlRecord {
    file: String,
    document_type: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonlError>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    warnings: Vec<OwnedWarningPayload>,
}

impl TextJsonlRecord {
    fn success(
        file: String,
        document_type: &'static str,
        extraction: oxdoc_core::Extraction<String>,
    ) -> Self {
        Self {
            file,
            document_type,
            text: Some(extraction.value),
            error: None,
            warnings: extraction
                .warnings
                .iter()
                .map(OwnedWarningPayload::from_output_warning)
                .collect(),
        }
    }

    fn error(file: String, document_type: &'static str, error: CliError) -> Self {
        Self {
            file,
            document_type,
            text: None,
            error: Some(JsonlError {
                code: error.code(),
                message: error.to_string(),
            }),
            warnings: Vec::new(),
        }
    }
}

#[derive(Debug, serde::Serialize)]
struct JsonlError {
    code: &'static str,
    message: String,
}

#[derive(Debug, serde::Serialize)]
struct OwnedWarningPayload {
    category: &'static str,
    code: &'static str,
    path: String,
    message: String,
}

impl OwnedWarningPayload {
    fn from_output_warning(warning: &OutputWarning) -> Self {
        Self {
            category: warning.category().as_str(),
            code: warning.code().as_str(),
            path: warning.path.clone(),
            message: warning.message.clone(),
        }
    }
}

#[derive(Debug, serde::Serialize)]
struct InfoPayload<'a> {
    oxdoc_version: &'static str,
    #[serde(flatten)]
    info: &'a DocumentInfo,
}

#[derive(Debug, serde::Serialize)]
struct AuditPayload<'a> {
    oxdoc_version: &'static str,
    #[serde(flatten)]
    audit: &'a DocumentAudit,
}

#[derive(Debug, serde::Serialize)]
struct WarningPayload<'a> {
    category: &'a str,
    code: &'a str,
    path: &'a str,
    message: &'a str,
}

#[derive(Debug, serde::Serialize)]
struct AllSheetsManifest {
    oxdoc_version: &'static str,
    file: String,
    sheets: Vec<ExportedSheetManifest>,
}

#[derive(Debug, serde::Serialize)]
struct ExportedSheetManifest {
    index: usize,
    visibility: &'static str,
    name: String,
    csv_path: String,
    warnings: Vec<ManifestWarning>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<ManifestError>,
}

#[derive(Debug, serde::Serialize)]
struct ManifestWarning {
    category: String,
    code: String,
    path: String,
    message: String,
}

impl ManifestWarning {
    fn from_output_warning(warning: &OutputWarning) -> Self {
        Self {
            category: warning.category().as_str().to_owned(),
            code: warning.code().as_str().to_owned(),
            path: warning.path.clone(),
            message: warning.message.clone(),
        }
    }
}

#[derive(Debug, serde::Serialize)]
struct ManifestError {
    code: &'static str,
    message: String,
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

fn csv_file_name_for_sheet(index: usize, name: &str) -> String {
    format!("{index:03}-{}.csv", sanitize_sheet_file_stem(name))
}

fn sanitize_sheet_file_stem(name: &str) -> String {
    let mut stem = String::new();
    let mut last_was_dash = false;

    for ch in name.chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            stem.push(ch);
            last_was_dash = false;
        } else if !last_was_dash && !stem.is_empty() {
            stem.push('-');
            last_was_dash = true;
        }
    }

    while stem.ends_with('-') {
        stem.pop();
    }

    if stem.is_empty() {
        "sheet".to_owned()
    } else {
        stem
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

fn emit_warnings(warnings: &[OutputWarning], format: WarningFormat) {
    for warning in warnings {
        emit_warning_parts(
            warning.category().as_str(),
            warning.code().as_str(),
            &warning.path,
            &warning.message,
            format,
        );
    }
}

fn emit_warning_parts(
    category: &str,
    code: &str,
    path: &str,
    message: &str,
    format: WarningFormat,
) {
    match format {
        WarningFormat::Text => eprintln!("warning[{category}/{code}]: {path}: {message}"),
        WarningFormat::Json => {
            let payload = WarningPayload {
                category,
                code,
                path,
                message,
            };
            let line = serde_json::to_string(&payload).expect("warning payload serializes");
            eprintln!("{line}");
        }
        WarningFormat::None => {}
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

fn print_audit(audit: &DocumentAudit) {
    println!("file: {}", audit.file);
    println!("document_type: {}", audit.document_type);
    println!("has_macros: {}", audit.metadata.has_macros);
    print_optional("author", audit.metadata.author.as_deref());
    print_optional(
        "last_modified_by",
        audit.metadata.last_modified_by.as_deref(),
    );
    print_optional("application", audit.metadata.application.as_deref());
    print_optional("company", audit.metadata.company.as_deref());
    print_optional_u64("word_count", audit.metadata.word_count);
    print_optional_u64("page_count", audit.metadata.page_count);
    print_optional_u64("slide_count", audit.metadata.slide_count);
    print_optional_u64("worksheet_count", audit.metadata.worksheet_count);
    println!("signal_count: {}", audit.signals.len());
    for signal in &audit.signals {
        print_audit_signal(signal);
    }
}

fn print_audit_signal(signal: &AuditSignal) {
    println!(
        "signal: {} {} {}: {}",
        signal.severity, signal.kind, signal.path, signal.message
    );
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

    use super::{CliError, csv_file_name_for_sheet, parse_delimiter};

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

    #[test]
    fn sheet_csv_file_names_are_deterministic_and_safe() {
        assert_eq!(csv_file_name_for_sheet(1, "Sales Q1"), "001-sales-q1.csv");
        assert_eq!(csv_file_name_for_sheet(2, "Ops/Q1 🚀"), "002-ops-q1.csv");
        assert_eq!(csv_file_name_for_sheet(12, "///"), "012-sheet.csv");
    }
}
