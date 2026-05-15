use std::io::{Read, Seek};

use zip::ZipArchive;
use zip::read::ZipFile;
use zip::result::ZipError;

use crate::{OxdocError, Result};

const DEFAULT_MAX_PART_UNCOMPRESSED_SIZE: u64 = 64 * 1024 * 1024;
const DEFAULT_MAX_PART_COMPRESSION_RATIO: u64 = 200;
const DEFAULT_MIN_RATIO_CHECK_SIZE: u64 = 4 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OoxmlLimits {
    pub max_part_uncompressed_size: u64,
    pub max_part_compression_ratio: u64,
    pub min_ratio_check_size: u64,
}

impl Default for OoxmlLimits {
    fn default() -> Self {
        Self {
            max_part_uncompressed_size: DEFAULT_MAX_PART_UNCOMPRESSED_SIZE,
            max_part_compression_ratio: DEFAULT_MAX_PART_COMPRESSION_RATIO,
            min_ratio_check_size: DEFAULT_MIN_RATIO_CHECK_SIZE,
        }
    }
}

pub struct OoxmlPackage<R: Read + Seek> {
    archive: ZipArchive<R>,
    limits: OoxmlLimits,
}

impl<R: Read + Seek> OoxmlPackage<R> {
    pub fn new(reader: R) -> Result<Self> {
        Self::with_limits(reader, OoxmlLimits::default())
    }

    pub fn with_limits(reader: R, limits: OoxmlLimits) -> Result<Self> {
        Ok(Self {
            archive: ZipArchive::new(reader)?,
            limits,
        })
    }

    pub fn with_entry<T>(
        &mut self,
        path: &str,
        read_entry: impl FnOnce(&mut dyn Read) -> Result<T>,
    ) -> Result<T> {
        self.with_entry_limits(path, self.limits, read_entry)
    }

    pub fn with_entry_limits<T>(
        &mut self,
        path: &str,
        limits: OoxmlLimits,
        read_entry: impl FnOnce(&mut dyn Read) -> Result<T>,
    ) -> Result<T> {
        match self.archive.by_name(path) {
            Ok(mut entry) => {
                validate_entry(path, &entry, limits)?;
                let mut entry =
                    LimitedEntryReader::new(&mut entry, limits.max_part_uncompressed_size);
                let result = read_entry(&mut entry);
                if entry.exceeded_limit() {
                    Err(OxdocError::PartTooLarge {
                        path: path.to_owned(),
                        size: entry.observed_size(),
                        limit: limits.max_part_uncompressed_size,
                    })
                } else {
                    result
                }
            }
            Err(err) => Err(map_zip_entry_error(path, err)),
        }
    }

    pub fn read_to_string(&mut self, path: &str) -> Result<String> {
        self.with_entry(path, |entry| {
            let mut content = String::new();
            entry.read_to_string(&mut content)?;
            Ok(content)
        })
    }

    pub fn contains(&mut self, path: &str) -> bool {
        self.archive.by_name(path).is_ok()
    }

    pub fn contains_any(&mut self, paths: &[&str]) -> bool {
        paths.iter().any(|path| self.contains(path))
    }

    pub fn part_names(&mut self) -> Vec<String> {
        (0..self.archive.len())
            .filter_map(|index| {
                self.archive.by_index(index).ok().and_then(|entry| {
                    entry
                        .enclosed_name()
                        .and_then(|path| path.to_str().map(str::to_owned))
                })
            })
            .collect()
    }
}

struct LimitedEntryReader<'a, R: Read + ?Sized> {
    inner: &'a mut R,
    limit: u64,
    read: u64,
    exceeded: bool,
    observed_size: u64,
}

impl<'a, R: Read + ?Sized> LimitedEntryReader<'a, R> {
    fn new(inner: &'a mut R, limit: u64) -> Self {
        Self {
            inner,
            limit,
            read: 0,
            exceeded: false,
            observed_size: 0,
        }
    }

    fn exceeded_limit(&self) -> bool {
        self.exceeded
    }

    fn observed_size(&self) -> u64 {
        self.observed_size.max(self.read)
    }
}

impl<R: Read + ?Sized> Read for LimitedEntryReader<'_, R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }

        if self.read >= self.limit {
            let mut probe = [0u8; 1];
            let bytes = self.inner.read(&mut probe)?;
            if bytes == 0 {
                return Ok(0);
            }
            self.exceeded = true;
            self.observed_size = self.read.saturating_add(bytes as u64);
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "OOXML part exceeded its configured uncompressed size limit",
            ));
        }

        let remaining = self.limit - self.read;
        let allowed = (buf.len() as u64).min(remaining) as usize;
        let bytes = self.inner.read(&mut buf[..allowed])?;
        self.read = self.read.saturating_add(bytes as u64);
        self.observed_size = self.read;
        Ok(bytes)
    }
}

fn validate_entry<R: Read + ?Sized>(
    path: &str,
    entry: &ZipFile<'_, R>,
    limits: OoxmlLimits,
) -> Result<()> {
    if entry.encrypted() {
        return Err(OxdocError::UnsupportedEncryptedPart(path.to_owned()));
    }

    if entry.is_dir() {
        return Err(OxdocError::SuspiciousZipEntry {
            path: path.to_owned(),
            reason: "required OOXML part resolves to a directory".to_owned(),
        });
    }

    if entry.enclosed_name().is_none() {
        return Err(OxdocError::SuspiciousZipEntry {
            path: path.to_owned(),
            reason: "ZIP entry name is not enclosed within the package".to_owned(),
        });
    }

    let size = entry.size();
    if size > limits.max_part_uncompressed_size {
        return Err(OxdocError::PartTooLarge {
            path: path.to_owned(),
            size,
            limit: limits.max_part_uncompressed_size,
        });
    }

    let compressed_size = entry.compressed_size();
    if size >= limits.min_ratio_check_size
        && (compressed_size == 0
            || size > compressed_size.saturating_mul(limits.max_part_compression_ratio))
    {
        return Err(OxdocError::SuspiciousZipEntry {
            path: path.to_owned(),
            reason: format!(
                "uncompressed size {size} bytes is too large for compressed size {compressed_size} bytes"
            ),
        });
    }

    Ok(())
}

fn map_zip_entry_error(path: &str, err: ZipError) -> OxdocError {
    match err {
        ZipError::FileNotFound => OxdocError::MissingPart(path.to_owned()),
        ZipError::UnsupportedArchive(reason) if reason == ZipError::PASSWORD_REQUIRED => {
            OxdocError::UnsupportedEncryptedPart(path.to_owned())
        }
        err => OxdocError::CorruptedZip(err),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use zip::write::{SimpleFileOptions, ZipWriter};

    fn create_zip(content: &[(&str, &[u8])]) -> Cursor<Vec<u8>> {
        let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
        for (name, data) in content {
            zip.start_file(*name, SimpleFileOptions::default()).unwrap();
            use std::io::Write;
            zip.write_all(data).unwrap();
        }
        zip.finish().unwrap()
    }

    #[test]
    fn test_vfs_contains_any_and_read_to_string() {
        let cursor = create_zip(&[("file1.txt", b"hello"), ("file2.txt", b"world")]);
        let mut pkg = OoxmlPackage::new(cursor).unwrap();

        assert!(pkg.contains_any(&["nonexistent.txt", "file2.txt"]));
        assert!(!pkg.contains_any(&["nonexistent.txt"]));

        assert_eq!(pkg.read_to_string("file1.txt").unwrap(), "hello");
    }

    #[test]
    fn test_vfs_limited_reader_empty_buf() {
        let cursor = create_zip(&[("file1.txt", b"hello")]);
        let mut pkg = OoxmlPackage::new(cursor).unwrap();

        pkg.with_entry("file1.txt", |reader| {
            let mut buf = [];
            assert_eq!(reader.read(&mut buf).unwrap(), 0);
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn test_map_zip_entry_error() {
        assert!(matches!(
            map_zip_entry_error("test", ZipError::FileNotFound),
            OxdocError::MissingPart(_)
        ));
        assert!(matches!(
            map_zip_entry_error(
                "test",
                ZipError::UnsupportedArchive(ZipError::PASSWORD_REQUIRED)
            ),
            OxdocError::UnsupportedEncryptedPart(_)
        ));
        assert!(matches!(
            map_zip_entry_error("test", ZipError::InvalidArchive("bad".into())),
            OxdocError::CorruptedZip(_)
        ));
    }

    #[test]
    fn test_vfs_contains_and_limits() {
        let cursor = create_zip(&[("file1.txt", b"hello world")]);
        let mut pkg = OoxmlPackage::new(cursor).unwrap();

        assert!(pkg.contains("file1.txt"));
        assert!(!pkg.contains("file2.txt"));

        // Exceed limit
        let limits = OoxmlLimits {
            max_part_uncompressed_size: 5,
            ..Default::default()
        };
        let err = pkg
            .with_entry_limits("file1.txt", limits, |reader| {
                let mut s = String::new();
                reader.read_to_string(&mut s)?;
                Ok(())
            })
            .unwrap_err();
        assert!(matches!(err, OxdocError::PartTooLarge { .. }));
    }
}
