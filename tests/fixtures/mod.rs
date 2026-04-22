use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

pub fn build_package(fixture: &str, file_name: &str) -> PathBuf {
    let source = workspace_root()
        .join("tests")
        .join("fixtures")
        .join("corpus")
        .join(fixture);
    let output_dir = unique_output_dir(fixture);
    fs::create_dir_all(&output_dir).unwrap();
    let output = output_dir.join(file_name);
    let file = File::create(&output).unwrap();
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);

    let mut files = Vec::new();
    collect_files(&source, &mut files).unwrap();
    files.sort();

    for path in files {
        let rel = path.strip_prefix(&source).unwrap();
        let name = rel
            .components()
            .map(|component| component.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/");
        zip.start_file(name, options).unwrap();
        let bytes = fs::read(&path).unwrap();
        zip.write_all(&bytes).unwrap();
    }

    zip.finish().unwrap();
    output
}

pub fn read_snapshot(relative_path: &str) -> String {
    let path = workspace_root()
        .join("tests")
        .join("fixtures")
        .join("snapshots")
        .join(relative_path);
    fs::read_to_string(path).unwrap()
}

pub fn fixture_file(relative_path: &str) -> PathBuf {
    workspace_root()
        .join("tests")
        .join("fixtures")
        .join("files")
        .join(relative_path)
}

pub fn read_provenance(relative_path: &str) -> String {
    let path = workspace_root()
        .join("tests")
        .join("fixtures")
        .join("provenance")
        .join(relative_path);
    fs::read_to_string(path).unwrap()
}

fn collect_files(dir: &Path, out: &mut Vec<PathBuf>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_files(&path, out)?;
        } else if path.is_file() {
            out.push(path);
        }
    }

    Ok(())
}

fn unique_output_dir(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let safe_name = name.replace('/', "-");
    std::env::temp_dir().join(format!(
        "oxdoc-fixtures-{}-{nonce}-{safe_name}",
        std::process::id()
    ))
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .to_path_buf()
}
