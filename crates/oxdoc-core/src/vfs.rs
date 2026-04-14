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
        let limits = self.limits;
        match self.archive.by_name(path) {
            Ok(mut entry) => {
                validate_entry(path, &entry, limits)?;
                read_entry(&mut entry)
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
