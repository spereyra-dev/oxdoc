use std::hint::black_box;
use std::io::{Cursor, Write};

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use oxdoc_core::{XlsxCsvOptions, extract_docx_text_from_reader, extract_xlsx_csv_from_reader};
use zip::CompressionMethod;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

fn docx_text_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("docx_text_throughput");

    for target_bytes in [16 * 1024, 256 * 1024] {
        let fixture = synthetic_docx(target_bytes);
        group.throughput(Throughput::Bytes(fixture.expected_text_bytes as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_text_bytes", fixture.expected_text_bytes)),
            &fixture,
            |b, fixture| {
                b.iter(|| {
                    let extraction =
                        extract_docx_text_from_reader(Cursor::new(fixture.package.as_slice()))
                            .expect("synthetic DOCX should parse");
                    assert!(extraction.warnings.is_empty());
                    assert_eq!(extraction.value.len(), fixture.expected_text_bytes);
                    black_box(extraction.value);
                });
            },
        );
    }

    group.finish();
}

fn xlsx_row_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("xlsx_row_throughput");

    for rows in [1_000, 10_000] {
        let fixture = synthetic_xlsx(rows, 8);
        group.throughput(Throughput::Elements(rows as u64));
        group.bench_with_input(BenchmarkId::from_parameter(rows), &fixture, |b, fixture| {
            b.iter(|| {
                let mut csv = Vec::with_capacity(fixture.expected_csv_bytes);
                let extraction = extract_xlsx_csv_from_reader(
                    Cursor::new(fixture.package.as_slice()),
                    XlsxCsvOptions::default(),
                    &mut csv,
                )
                .expect("synthetic XLSX should parse");
                assert!(extraction.warnings.is_empty());
                assert_eq!(csv.len(), fixture.expected_csv_bytes);
                black_box(csv);
            });
        });
    }

    group.finish();
}

fn xlsx_shared_string_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("xlsx_shared_string_throughput");

    for (label, shared_count, value_len) in [
        ("memory", 128usize, 32usize),
        ("spill", 4_096usize, 2_048usize),
    ] {
        let fixture = synthetic_xlsx_shared_strings(shared_count, value_len);
        group.throughput(Throughput::Bytes(fixture.expected_csv_bytes as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{label}_{shared_count}x{value_len}")),
            &fixture,
            |b, fixture| {
                b.iter(|| {
                    let mut csv = Vec::with_capacity(fixture.expected_csv_bytes);
                    let extraction = extract_xlsx_csv_from_reader(
                        Cursor::new(fixture.package.as_slice()),
                        XlsxCsvOptions::default(),
                        &mut csv,
                    )
                    .expect("synthetic shared-string XLSX should parse");
                    assert!(extraction.warnings.is_empty());
                    assert_eq!(csv.len(), fixture.expected_csv_bytes);
                    black_box(csv);
                });
            },
        );
    }

    group.finish();
}

#[derive(Debug)]
struct DocxFixture {
    package: Vec<u8>,
    expected_text_bytes: usize,
}

#[derive(Debug)]
struct XlsxFixture {
    package: Vec<u8>,
    expected_csv_bytes: usize,
}

fn synthetic_docx(target_text_bytes: usize) -> DocxFixture {
    let mut document = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body>"#,
    );
    let mut expected_text_bytes = 0;
    let mut paragraph = 0;

    while expected_text_bytes < target_text_bytes {
        let text = format!("Benchmark paragraph {paragraph} with stable ASCII text.");
        document.push_str("<w:p><w:r><w:t>");
        document.push_str(&text);
        document.push_str("</w:t></w:r></w:p>");
        expected_text_bytes += text.len() + 1;
        paragraph += 1;
    }

    document.push_str("</w:body></w:document>");

    DocxFixture {
        package: zip_package([("word/document.xml", document.as_bytes())]),
        expected_text_bytes,
    }
}

fn synthetic_xlsx(rows: usize, columns: usize) -> XlsxFixture {
    let workbook = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets><sheet name="Data" sheetId="1" r:id="rId1"/></sheets></workbook>"#;
    let workbook_rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#;

    let mut sheet = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData>"#,
    );
    let mut expected_csv_bytes = 0;

    for row in 1..=rows {
        sheet.push_str("<row>");
        for column in 1..=columns {
            let value = row * column;
            let cell = format!(
                r#"<c r="{}{}"><v>{value}</v></c>"#,
                column_name(column),
                row
            );
            sheet.push_str(&cell);
            expected_csv_bytes += value.to_string().len();
            if column < columns {
                expected_csv_bytes += 1;
            }
        }
        sheet.push_str("</row>");
        expected_csv_bytes += 1;
    }

    sheet.push_str("</sheetData></worksheet>");

    XlsxFixture {
        package: zip_package([
            ("xl/workbook.xml", workbook.as_bytes()),
            ("xl/_rels/workbook.xml.rels", workbook_rels.as_bytes()),
            ("xl/worksheets/sheet1.xml", sheet.as_bytes()),
        ]),
        expected_csv_bytes,
    }
}

fn synthetic_xlsx_shared_strings(shared_count: usize, value_len: usize) -> XlsxFixture {
    let workbook = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets><sheet name="Data" sheetId="1" r:id="rId1"/></sheets></workbook>"#;
    let workbook_rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#;

    let mut shared_strings = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">"#,
    );
    let mut sheet = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData>"#,
    );
    let mut expected_csv_bytes = 0;

    for index in 0..shared_count {
        let value = shared_string_value(index, value_len);
        shared_strings.push_str("<si><t>");
        shared_strings.push_str(&value);
        shared_strings.push_str("</t></si>");

        let row = index + 1;
        sheet.push_str("<row>");
        sheet.push_str(&format!(r#"<c r="A{row}" t="s"><v>{index}</v></c>"#));
        sheet.push_str("</row>");
        expected_csv_bytes += value.len() + 1;
    }

    shared_strings.push_str("</sst>");
    sheet.push_str("</sheetData></worksheet>");

    XlsxFixture {
        package: zip_package([
            ("xl/workbook.xml", workbook.as_bytes()),
            ("xl/_rels/workbook.xml.rels", workbook_rels.as_bytes()),
            ("xl/sharedStrings.xml", shared_strings.as_bytes()),
            ("xl/worksheets/sheet1.xml", sheet.as_bytes()),
        ]),
        expected_csv_bytes,
    }
}

fn shared_string_value(index: usize, value_len: usize) -> String {
    let prefix = format!("shared-{index:08}-");
    if prefix.len() >= value_len {
        return prefix[..value_len].to_owned();
    }

    let mut value = prefix;
    value.extend(std::iter::repeat_n('x', value_len - value.len()));
    value
}

fn zip_package<'a>(entries: impl IntoIterator<Item = (&'a str, &'a [u8])>) -> Vec<u8> {
    let mut cursor = Cursor::new(Vec::new());
    let mut zip = ZipWriter::new(&mut cursor);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);

    for (path, contents) in entries {
        zip.start_file(path, options)
            .expect("synthetic ZIP entry should start");
        zip.write_all(contents)
            .expect("synthetic ZIP entry should write");
    }

    zip.finish().expect("synthetic ZIP should finish");
    cursor.into_inner()
}

fn column_name(mut index: usize) -> String {
    let mut name = Vec::new();

    while index > 0 {
        index -= 1;
        name.push(b'A' + (index % 26) as u8);
        index /= 26;
    }

    name.reverse();
    String::from_utf8(name).expect("column name should be ASCII")
}

criterion_group!(
    benches,
    docx_text_throughput,
    xlsx_row_throughput,
    xlsx_shared_string_throughput
);
criterion_main!(benches);
