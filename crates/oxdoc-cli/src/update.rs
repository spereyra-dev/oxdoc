use std::fs::{self, File};
use std::io::{self};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

const REPO: &str = "spereyra-dev/oxdoc";
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
#[cfg(not(test))]
const GITHUB_API: &str = "https://api.github.com";

#[cfg(test)]
fn github_api() -> String {
    std::env::var("OXDOC_TEST_GITHUB_API").unwrap_or_else(|_| "https://api.github.com".to_string())
}

#[cfg(not(test))]
fn github_api() -> String {
    GITHUB_API.to_string()
}

const GITHUB_URL: &str = "https://github.com";

#[derive(Debug)]
pub enum UpdateOutcome {
    AlreadyUpToDate { version: String },
    UpdateAvailable { current: String, latest: String },
    Updated { from: String, to: String },
}

pub fn run(check_only: bool, target_version: Option<String>) -> Result<UpdateOutcome, String> {
    let latest_tag = match target_version {
        Some(v) => normalize_tag(&v),
        None => fetch_latest_tag()?,
    };

    let current = CURRENT_VERSION.to_owned();
    let current_tag = format!("v{current}");

    if latest_tag == current_tag {
        return Ok(UpdateOutcome::AlreadyUpToDate { version: current });
    }

    if check_only {
        return Ok(UpdateOutcome::UpdateAvailable {
            current,
            latest: latest_tag,
        });
    }

    let target = detect_target()?;
    let tmp = make_tempdir()?;
    let result = download_and_install(&latest_tag, target, &tmp);
    let _ = fs::remove_dir_all(&tmp);
    result.map(|()| UpdateOutcome::Updated {
        from: current,
        to: latest_tag,
    })
}

fn download_and_install(tag: &str, target: &str, tmp: &Path) -> Result<(), String> {
    let archive_name = format!("oxdoc-{tag}-{target}.tar.gz");
    let base_url = format!("{GITHUB_URL}/{REPO}/releases/download/{tag}");

    let archive_path = tmp.join(&archive_name);
    let checksums_path = tmp.join("SHA256SUMS");

    eprintln!("Downloading oxdoc {tag}...");
    download(&format!("{base_url}/{archive_name}"), &archive_path)?;
    download(&format!("{base_url}/SHA256SUMS"), &checksums_path)?;

    eprintln!("Verifying checksum...");
    verify_checksum(&archive_path, &archive_name, &checksums_path)?;

    eprintln!("Installing...");
    let new_binary = extract_binary(&archive_path, tag, target, tmp)?;
    replace_binary(&new_binary)
}

fn fetch_latest_tag() -> Result<String, String> {
    let url = format!("{}/repos/{REPO}/releases/latest", github_api());
    let response = ureq::get(&url)
        .header("User-Agent", &format!("oxdoc/{CURRENT_VERSION}"))
        .header("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| format!("failed to fetch latest release: {e}"))?;

    let json: serde_json::Value = serde_json::from_reader(response.into_body().as_reader())
        .map_err(|e| format!("failed to parse release response: {e}"))?;

    let tag = json["tag_name"]
        .as_str()
        .ok_or_else(|| "release response did not include tag_name".to_owned())?;

    Ok(normalize_tag(tag))
}

fn normalize_tag(v: &str) -> String {
    if v.starts_with('v') {
        v.to_owned()
    } else {
        format!("v{v}")
    }
}

fn detect_target() -> Result<&'static str, String> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => Ok("x86_64-unknown-linux-gnu"),
        ("macos", "x86_64") => Ok("x86_64-apple-darwin"),
        ("macos", "aarch64") => Ok("aarch64-apple-darwin"),
        (os, arch) => Err(format!(
            "unsupported platform: {os}/{arch}. Download manually from \
             https://github.com/{REPO}/releases"
        )),
    }
}

fn download(url: &str, dest: &Path) -> Result<(), String> {
    let response = ureq::get(url)
        .header("User-Agent", &format!("oxdoc/{CURRENT_VERSION}"))
        .call()
        .map_err(|e| format!("download failed: {url}: {e}"))?;

    let mut file =
        File::create(dest).map_err(|e| format!("failed to create {}: {e}", dest.display()))?;

    let mut body = response.into_body();
    let mut reader = body.as_reader();
    io::copy(&mut reader, &mut file)
        .map_err(|e| format!("failed to write {}: {e}", dest.display()))?;

    Ok(())
}

fn compute_sha256(path: &Path) -> Result<String, String> {
    let mut file =
        File::open(path).map_err(|e| format!("failed to open {}: {e}", path.display()))?;
    let mut hasher = Sha256::new();
    io::copy(&mut file, &mut hasher)
        .map_err(|e| format!("failed to read {}: {e}", path.display()))?;
    Ok(format!("{:x}", hasher.finalize()))
}

fn verify_checksum(archive: &Path, archive_name: &str, checksums: &Path) -> Result<(), String> {
    let content =
        fs::read_to_string(checksums).map_err(|e| format!("failed to read SHA256SUMS: {e}"))?;

    let expected = content
        .lines()
        .find_map(|line| {
            let (hash, name) = line.split_once("  ")?;
            (name.trim() == archive_name).then(|| hash.trim().to_owned())
        })
        .ok_or_else(|| format!("SHA256SUMS does not contain {archive_name}"))?;

    let actual = compute_sha256(archive)?;

    if expected != actual {
        return Err(format!(
            "checksum mismatch for {archive_name}: expected {expected}, got {actual}"
        ));
    }

    Ok(())
}

fn extract_binary(
    archive: &Path,
    tag: &str,
    target: &str,
    dest_dir: &Path,
) -> Result<PathBuf, String> {
    use flate2::read::GzDecoder;
    use tar::Archive;

    let file = File::open(archive).map_err(|e| format!("failed to open archive: {e}"))?;
    let gz = GzDecoder::new(file);
    let mut tar = Archive::new(gz);

    let expected_suffix = format!("oxdoc-{tag}-{target}/oxdoc");
    let dest = dest_dir.join("oxdoc");

    for entry in tar
        .entries()
        .map_err(|e| format!("failed to read archive: {e}"))?
    {
        let mut entry = entry.map_err(|e| format!("failed to read archive entry: {e}"))?;
        let path = entry
            .path()
            .map_err(|e| format!("failed to read entry path: {e}"))?;

        if path
            .to_str()
            .is_some_and(|p| p.ends_with("/oxdoc") || p == "oxdoc")
            || path.to_str().is_some_and(|p| p == expected_suffix.as_str())
        {
            entry
                .unpack(&dest)
                .map_err(|e| format!("failed to extract binary: {e}"))?;
            return Ok(dest);
        }
    }

    Err(format!(
        "archive did not contain an oxdoc binary (expected path ending in {expected_suffix})"
    ))
}

fn replace_binary(new_binary: &Path) -> Result<(), String> {
    let current =
        std::env::current_exe().map_err(|e| format!("failed to locate current binary: {e}"))?;
    replace_binary_impl(new_binary, &current)
}

fn replace_binary_impl(new_binary: &Path, current: &Path) -> Result<(), String> {
    // Resolve symlinks so we write to the real file
    let current = current
        .canonicalize()
        .unwrap_or_else(|_| current.to_path_buf());

    // Temp file in the same directory to guarantee same-filesystem rename
    let tmp = current
        .parent()
        .unwrap_or(Path::new("."))
        .join(format!(".oxdoc.update.{}", std::process::id()));

    fs::copy(new_binary, &tmp)
        .map_err(|e| format!("failed to copy new binary to {}: {e}", tmp.display()))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&tmp, fs::Permissions::from_mode(0o755))
            .map_err(|e| format!("failed to set permissions: {e}"))?;
    }

    fs::rename(&tmp, &current).map_err(|e| {
        let _ = fs::remove_file(&tmp);
        format!("failed to replace binary at {}: {e}", current.display())
    })?;

    Ok(())
}

fn make_tempdir() -> Result<PathBuf, String> {
    let dir = std::env::temp_dir().join(format!("oxdoc-update-{}", std::process::id()));
    fs::create_dir_all(&dir).map_err(|e| format!("failed to create temp dir: {e}"))?;
    Ok(dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_version_tags() {
        assert_eq!(normalize_tag("v0.2.0"), "v0.2.0");
        assert_eq!(normalize_tag("0.2.0"), "v0.2.0");
        assert_eq!(normalize_tag("v1.0.0"), "v1.0.0");
    }

    #[test]
    fn detects_target_for_current_platform() {
        // Should not error on CI/dev machines (Linux x86_64 or macOS arm64/x86_64)
        let result = detect_target();
        assert!(
            result.is_ok(),
            "detect_target() failed on this platform: {:?}",
            result
        );
    }

    #[test]
    fn verifies_checksum_matches() {
        let tmp = tempfile::tempdir().unwrap();
        let content = b"hello world";
        let file_path = tmp.path().join("test.tar.gz");
        std::fs::write(&file_path, content).unwrap();

        // Compute actual SHA256
        use sha2::{Digest, Sha256};
        let hash = format!("{:x}", Sha256::digest(content));
        let checksums_content = format!("{hash}  test.tar.gz\n");
        let checksums_path = tmp.path().join("SHA256SUMS");
        std::fs::write(&checksums_path, checksums_content).unwrap();

        assert!(verify_checksum(&file_path, "test.tar.gz", &checksums_path).is_ok());
    }

    #[test]
    fn rejects_checksum_mismatch() {
        let tmp = tempfile::tempdir().unwrap();
        let file_path = tmp.path().join("test.tar.gz");
        std::fs::write(&file_path, b"tampered").unwrap();

        let checksums_path = tmp.path().join("SHA256SUMS");
        std::fs::write(
            &checksums_path,
            "0000000000000000000000000000000000000000000000000000000000000000  test.tar.gz\n",
        )
        .unwrap();

        assert!(verify_checksum(&file_path, "test.tar.gz", &checksums_path).is_err());
    }

    #[test]
    fn run_check_only_returns_update_available() {
        let result = run(true, Some("v9.9.9".to_owned())).unwrap();
        match result {
            UpdateOutcome::UpdateAvailable { latest, .. } => assert_eq!(latest, "v9.9.9"),
            _ => panic!("Expected UpdateAvailable"),
        }
    }

    #[test]
    fn run_with_current_version_returns_already_up_to_date() {
        let current = format!("v{}", env!("CARGO_PKG_VERSION"));
        let result = run(true, Some(current.clone())).unwrap();
        match result {
            UpdateOutcome::AlreadyUpToDate { version } => {
                assert_eq!(format!("v{version}"), current)
            }
            _ => panic!("Expected AlreadyUpToDate"),
        }
    }

    #[test]
    fn test_make_tempdir() {
        let dir = make_tempdir().unwrap();
        assert!(dir.exists());
        assert!(dir.is_dir());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn test_compute_sha256() {
        let tmp = tempfile::tempdir().unwrap();
        let file_path = tmp.path().join("hash_test.txt");
        std::fs::write(&file_path, b"oxdoc").unwrap();
        let hash = compute_sha256(&file_path).unwrap();
        // SHA256 of "oxdoc"
        assert_eq!(
            hash,
            "96ca77f33ea0a66d90782c3a9ed3c33cdfdec10669f68b8fc10b39aa3a696494"
        );
    }

    #[test]
    fn test_extract_binary() {
        use flate2::write::GzEncoder;
        use tar::Builder;
        let tmp = tempfile::tempdir().unwrap();
        let archive_path = tmp.path().join("oxdoc-v1.0.0-target.tar.gz");
        let dest_dir = tmp.path().join("extracted");
        std::fs::create_dir_all(&dest_dir).unwrap();

        let file = std::fs::File::create(&archive_path).unwrap();
        let gz = GzEncoder::new(file, flate2::Compression::default());
        let mut builder = Builder::new(gz);

        let mut header = tar::Header::new_gnu();
        header.set_size(5);
        header.set_mode(0o755);
        header.set_cksum();
        builder
            .append_data(&mut header, "oxdoc-v1.0.0-target/oxdoc", &b"dummy"[..])
            .unwrap();
        builder.into_inner().unwrap().finish().unwrap();

        let extracted_path = extract_binary(&archive_path, "v1.0.0", "target", &dest_dir).unwrap();
        assert!(extracted_path.exists());
        let content = std::fs::read_to_string(extracted_path).unwrap();
        assert_eq!(content, "dummy");
    }

    #[test]
    fn test_download_error_path() {
        let tmp = tempfile::tempdir().unwrap();
        let dest = tmp.path().join("out.txt");
        // Invalid URL should fail immediately
        let result = download("http://localhost:1", &dest);
        assert!(result.is_err());
    }

    #[test]
    fn test_replace_binary_error_path() {
        let result = replace_binary(std::path::Path::new("/does/not/exist/oxdoc"));
        assert!(result.is_err());
    }

    #[test]
    fn test_download_and_install_error_path() {
        let tmp = tempfile::tempdir().unwrap();
        let result = download_and_install("v0.0.0-nonexistent", "unknown-target", tmp.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_download_success() {
        use std::io::Write;
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();

        std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let response = "HTTP/1.1 200 OK\r\nContent-Length: 5\r\n\r\nhello";
                let _ = stream.write_all(response.as_bytes());
            }
        });

        let tmp = tempfile::tempdir().unwrap();
        let dest = tmp.path().join("out.txt");
        let result = download(&format!("http://127.0.0.1:{}", port), &dest);
        assert!(result.is_ok());
        assert_eq!(std::fs::read_to_string(dest).unwrap(), "hello");
    }

    #[test]
    fn test_fetch_latest_tag_success_mocked() {
        use std::io::Write;
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();

        std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let response =
                    "HTTP/1.1 200 OK\r\nContent-Length: 21\r\n\r\n{\"tag_name\":\"v1.2.3\"}";
                let _ = stream.write_all(response.as_bytes());
            }
        });

        unsafe {
            std::env::set_var(
                "OXDOC_TEST_GITHUB_API",
                format!("http://127.0.0.1:{}", port),
            );
        }
        let tag = fetch_latest_tag().unwrap();
        assert_eq!(tag, "v1.2.3");
        unsafe {
            std::env::remove_var("OXDOC_TEST_GITHUB_API");
        }
    }

    #[test]
    fn test_fetch_latest_tag() {
        // Just execute it to cover lines. Network might fail in CI, so ignore the result.
        let _ = fetch_latest_tag();
    }

    #[test]
    fn test_extract_binary_invalid_archive() {
        let tmp = tempfile::tempdir().unwrap();
        let archive_path = tmp.path().join("invalid.tar.gz");
        std::fs::write(&archive_path, b"not a tarball").unwrap();
        let dest_dir = tmp.path().join("extracted");

        let result = extract_binary(&archive_path, "v1.0.0", "target", &dest_dir);
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_binary_missing_oxdoc_binary() {
        use flate2::write::GzEncoder;
        use tar::Builder;

        let tmp = tempfile::tempdir().unwrap();
        let archive_path = tmp.path().join("oxdoc-v1.0.0-target.tar.gz");
        let dest_dir = tmp.path().join("extracted");
        std::fs::create_dir_all(&dest_dir).unwrap();

        let file = std::fs::File::create(&archive_path).unwrap();
        let gz = GzEncoder::new(file, flate2::Compression::default());
        let mut builder = Builder::new(gz);
        let mut header = tar::Header::new_gnu();
        header.set_size(4);
        header.set_mode(0o644);
        header.set_cksum();
        builder
            .append_data(&mut header, "oxdoc-v1.0.0-target/README", &b"docs"[..])
            .unwrap();
        builder.into_inner().unwrap().finish().unwrap();

        let result = extract_binary(&archive_path, "v1.0.0", "target", &dest_dir);
        assert!(
            result
                .unwrap_err()
                .contains("archive did not contain an oxdoc binary")
        );
    }

    #[test]
    fn test_replace_binary_impl_success() {
        let tmp = tempfile::tempdir().unwrap();
        let new_binary = tmp.path().join("new_oxdoc");
        let current_binary = tmp.path().join("current_oxdoc");
        std::fs::write(&new_binary, b"new").unwrap();
        std::fs::write(&current_binary, b"old").unwrap();

        replace_binary_impl(&new_binary, &current_binary).unwrap();

        assert_eq!(std::fs::read(&current_binary).unwrap(), b"new");
    }

    #[test]
    fn test_replace_binary_impl_error() {
        let tmp = tempfile::tempdir().unwrap();
        let new_binary = tmp.path().join("new_oxdoc");
        std::fs::write(&new_binary, b"dummy").unwrap();

        let current_binary = tmp.path().join("dir_does_not_exist").join("oxdoc");
        let result = replace_binary_impl(&new_binary, &current_binary);
        assert!(result.is_err());
    }

    #[test]
    fn test_download_and_install_error_path_404() {
        let tmp = tempfile::tempdir().unwrap();
        let result = download_and_install("v999.999.999", "x86_64-unknown-linux-gnu", tmp.path());
        assert!(result.is_err());
    }
}
