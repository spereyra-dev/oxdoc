use std::io::{Read, Seek};

use zip::ZipArchive;
use zip::result::ZipError;

use crate::{OxdocError, Result};

pub struct OoxmlPackage<R: Read + Seek> {
    archive: ZipArchive<R>,
}

impl<R: Read + Seek> OoxmlPackage<R> {
    pub fn new(reader: R) -> Result<Self> {
        Ok(Self {
            archive: ZipArchive::new(reader)?,
        })
    }

    pub fn with_entry<T>(
        &mut self,
        path: &str,
        read_entry: impl FnOnce(&mut dyn Read) -> Result<T>,
    ) -> Result<T> {
        match self.archive.by_name(path) {
            Ok(mut entry) => read_entry(&mut entry),
            Err(ZipError::FileNotFound) => Err(OxdocError::MissingPart(path.to_owned())),
            Err(err) => Err(OxdocError::CorruptedZip(err)),
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
