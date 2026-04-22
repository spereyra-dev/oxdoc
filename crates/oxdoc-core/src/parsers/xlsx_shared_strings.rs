use std::fs::{File, OpenOptions, remove_file};
use std::io::{BufRead, Read, Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use quick_xml::Reader;
use quick_xml::events::Event;

use crate::models::{Extraction, OutputWarning};
use crate::parsers::{decode_xml_reference, decode_xml_text, name_eq};
use crate::{OxdocError, Result};

pub(crate) const DEFAULT_SHARED_STRING_MEMORY_LIMIT: usize = 8 * 1024 * 1024;

const ESTIMATED_INDEX_MEMORY_COST: usize = 16;

static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

pub(crate) trait SharedStringLookup {
    fn lookup(&mut self, index: usize) -> Result<Option<String>>;
}

#[derive(Debug)]
pub(crate) enum SharedStringStore {
    Memory(MemorySharedStringStore),
    Disk(DiskSharedStringStore),
}

impl SharedStringStore {
    pub(crate) fn empty() -> Self {
        Self::Memory(MemorySharedStringStore::default())
    }

    pub(crate) fn from_values(values: Vec<String>) -> Self {
        Self::Memory(MemorySharedStringStore::new(values))
    }

    pub(crate) fn parse<R: BufRead>(source: R, path: &str) -> Result<Extraction<Self>> {
        Self::parse_with_memory_limit(source, path, DEFAULT_SHARED_STRING_MEMORY_LIMIT)
    }

    pub(crate) fn parse_with_memory_limit<R: BufRead>(
        source: R,
        path: &str,
        memory_limit: usize,
    ) -> Result<Extraction<Self>> {
        parse_shared_strings(source, path, memory_limit)
    }

    #[cfg(test)]
    fn is_disk_backed(&self) -> bool {
        matches!(self, Self::Disk(_))
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        match self {
            Self::Memory(store) => store.values.len(),
            Self::Disk(store) => store.len,
        }
    }
}

impl SharedStringLookup for SharedStringStore {
    fn lookup(&mut self, index: usize) -> Result<Option<String>> {
        match self {
            Self::Memory(store) => store.lookup(index),
            Self::Disk(store) => store.lookup(index),
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct MemorySharedStringStore {
    values: Vec<String>,
}

impl MemorySharedStringStore {
    fn new(values: Vec<String>) -> Self {
        Self { values }
    }
}

impl SharedStringLookup for MemorySharedStringStore {
    fn lookup(&mut self, index: usize) -> Result<Option<String>> {
        Ok(self.values.get(index).cloned())
    }
}

#[derive(Debug)]
pub(crate) struct DiskSharedStringStore {
    data: TempFile,
    index: TempFile,
    len: usize,
}

impl DiskSharedStringStore {
    fn new() -> Result<Self> {
        Ok(Self {
            data: TempFile::new("oxdoc-shared-strings-data")?,
            index: TempFile::new("oxdoc-shared-strings-index")?,
            len: 0,
        })
    }

    fn push(&mut self, value: &str) -> Result<()> {
        let offset = self.data.file.seek(SeekFrom::End(0))?;
        let length = u64::try_from(value.len()).map_err(|_| {
            OxdocError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "shared string length does not fit in u64",
            ))
        })?;

        self.data.file.write_all(value.as_bytes())?;
        self.index.file.seek(SeekFrom::End(0))?;
        self.index.file.write_all(&offset.to_le_bytes())?;
        self.index.file.write_all(&length.to_le_bytes())?;
        self.len += 1;
        Ok(())
    }
}

impl SharedStringLookup for DiskSharedStringStore {
    fn lookup(&mut self, index: usize) -> Result<Option<String>> {
        if index >= self.len {
            return Ok(None);
        }

        let index_offset = u64::try_from(index)
            .ok()
            .and_then(|index| index.checked_mul(16))
            .ok_or_else(|| {
                OxdocError::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "shared string index offset overflowed",
                ))
            })?;
        self.index.file.seek(SeekFrom::Start(index_offset))?;

        let mut entry = [0u8; 16];
        self.index.file.read_exact(&mut entry)?;
        let offset = u64::from_le_bytes(entry[0..8].try_into().expect("offset slice is 8 bytes"));
        let length_u64 =
            u64::from_le_bytes(entry[8..16].try_into().expect("length slice is 8 bytes"));
        let length = usize::try_from(length_u64).map_err(|_| {
            OxdocError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "shared string length does not fit in usize",
            ))
        })?;

        self.data.file.seek(SeekFrom::Start(offset))?;
        let mut bytes = vec![0u8; length];
        self.data.file.read_exact(&mut bytes)?;
        Ok(Some(String::from_utf8_lossy(&bytes).into_owned()))
    }
}

#[derive(Debug)]
struct SharedStringStoreBuilder {
    memory_limit: usize,
    memory_bytes: usize,
    memory_values: Vec<String>,
    disk_store: Option<DiskSharedStringStore>,
}

impl SharedStringStoreBuilder {
    fn new(memory_limit: usize) -> Self {
        Self {
            memory_limit,
            memory_bytes: 0,
            memory_values: Vec::new(),
            disk_store: None,
        }
    }

    fn push(&mut self, value: String) -> Result<()> {
        if let Some(store) = &mut self.disk_store {
            return store.push(&value);
        }

        let next_memory_bytes = self
            .memory_bytes
            .saturating_add(estimated_memory_cost(&value));
        if next_memory_bytes > self.memory_limit {
            self.spill_to_disk()?;
            if let Some(store) = &mut self.disk_store {
                return store.push(&value);
            }
        }

        self.memory_bytes = next_memory_bytes;
        self.memory_values.push(value);
        Ok(())
    }

    fn spill_to_disk(&mut self) -> Result<()> {
        let mut store = DiskSharedStringStore::new()?;
        for value in &self.memory_values {
            store.push(value)?;
        }
        self.memory_values.clear();
        self.memory_bytes = 0;
        self.disk_store = Some(store);
        Ok(())
    }

    fn finish(self) -> SharedStringStore {
        if let Some(store) = self.disk_store {
            SharedStringStore::Disk(store)
        } else {
            SharedStringStore::from_values(self.memory_values)
        }
    }
}

fn estimated_memory_cost(value: &str) -> usize {
    value.len().saturating_add(ESTIMATED_INDEX_MEMORY_COST)
}

fn parse_shared_strings<R: BufRead>(
    source: R,
    path: &str,
    memory_limit: usize,
) -> Result<Extraction<SharedStringStore>> {
    let mut reader = Reader::from_reader(source);
    reader.config_mut().trim_text(false);
    let mut buf = Vec::new();
    let mut store = SharedStringStoreBuilder::new(memory_limit);
    let mut warnings = Vec::new();
    let mut current_string: Option<String> = None;
    let mut in_text = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(element)) => {
                if name_eq(element.name().as_ref(), b"si") {
                    current_string = Some(String::new());
                } else if current_string.is_some() && name_eq(element.name().as_ref(), b"t") {
                    in_text = true;
                }
            }
            Ok(Event::Empty(element)) => {
                if name_eq(element.name().as_ref(), b"si") {
                    store.push(String::new())?;
                } else if let Some(value) = &mut current_string {
                    if name_eq(element.name().as_ref(), b"tab") {
                        value.push('\t');
                    } else if name_eq(element.name().as_ref(), b"br")
                        || name_eq(element.name().as_ref(), b"cr")
                    {
                        value.push('\n');
                    }
                }
            }
            Ok(Event::Text(value)) if in_text => {
                if let Some(current) = &mut current_string {
                    current.push_str(&decode_xml_text(value.as_ref()));
                }
            }
            Ok(Event::CData(value)) if in_text => {
                if let Some(current) = &mut current_string {
                    current.push_str(&decode_xml_text(value.as_ref()));
                }
            }
            Ok(Event::GeneralRef(value)) if in_text => {
                if let Some(current) = &mut current_string {
                    current.push_str(&decode_xml_reference(value.as_ref()));
                }
            }
            Ok(Event::End(element)) => {
                if name_eq(element.name().as_ref(), b"t") {
                    in_text = false;
                } else if name_eq(element.name().as_ref(), b"si")
                    && let Some(current) = current_string.take()
                {
                    store.push(current)?;
                }
            }
            Ok(Event::Eof) => break,
            Err(source) => {
                warnings.push(OutputWarning::malformed_xml(path, source));
                break;
            }
            _ => {}
        }
        buf.clear();
    }

    Ok(Extraction::with_warnings(store.finish(), warnings))
}

#[derive(Debug)]
struct TempFile {
    file: File,
    path: PathBuf,
}

impl TempFile {
    fn new(prefix: &str) -> Result<Self> {
        let directory = std::env::temp_dir();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let process_id = std::process::id();

        for _ in 0..100 {
            let counter = TEMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = directory.join(format!("{prefix}-{process_id}-{now}-{counter}.tmp"));
            match OpenOptions::new()
                .read(true)
                .write(true)
                .create_new(true)
                .open(&path)
            {
                Ok(file) => return Ok(Self { file, path }),
                Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(err) => return Err(err.into()),
            }
        }

        Err(OxdocError::Io(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            "could not create a unique shared string temporary file",
        )))
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = remove_file(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use std::path::PathBuf;

    use super::{SharedStringLookup, SharedStringStore};

    #[test]
    fn parses_shared_strings_in_memory() {
        let xml = r#"
            <sst>
              <si><t>Cliente</t></si>
              <si><r><t>A</t></r><r><t> &amp; B</t></r></si>
              <si><t><![CDATA[A < B]]></t><r><br/></r><r><tab/></r><r><t>&quot;ok&quot;</t></r></si>
              <si/>
            </sst>
        "#;

        let mut store =
            SharedStringStore::parse(Cursor::new(xml.as_bytes()), "xl/sharedStrings.xml")
                .unwrap()
                .value;

        assert!(!store.is_disk_backed());
        assert_eq!(store.len(), 4);
        assert_eq!(store.lookup(0).unwrap().as_deref(), Some("Cliente"));
        assert_eq!(store.lookup(1).unwrap().as_deref(), Some("A & B"));
        assert_eq!(store.lookup(2).unwrap().as_deref(), Some("A < B\n\t\"ok\""));
        assert_eq!(store.lookup(3).unwrap().as_deref(), Some(""));
        assert_eq!(store.lookup(4).unwrap(), None);
    }

    #[test]
    fn spills_shared_strings_to_disk_after_threshold() {
        let xml = r#"
            <sst>
              <si><t>alpha</t></si>
              <si><t>beta</t></si>
              <si><t>gamma</t></si>
            </sst>
        "#;

        let mut store = SharedStringStore::parse_with_memory_limit(
            Cursor::new(xml.as_bytes()),
            "xl/sharedStrings.xml",
            6,
        )
        .unwrap()
        .value;

        assert!(store.is_disk_backed());
        assert_eq!(store.len(), 3);
        assert_eq!(store.lookup(0).unwrap().as_deref(), Some("alpha"));
        assert_eq!(store.lookup(1).unwrap().as_deref(), Some("beta"));
        assert_eq!(store.lookup(2).unwrap().as_deref(), Some("gamma"));
        assert_eq!(store.lookup(3).unwrap(), None);
    }

    #[test]
    fn removes_disk_store_temp_files_on_drop() {
        let xml = r#"
            <sst>
              <si><t>alpha</t></si>
              <si><t>beta</t></si>
            </sst>
        "#;

        let store = SharedStringStore::parse_with_memory_limit(
            Cursor::new(xml.as_bytes()),
            "xl/sharedStrings.xml",
            1,
        )
        .unwrap()
        .value;

        let (data_path, index_path): (PathBuf, PathBuf) = match store {
            SharedStringStore::Disk(ref disk) => (disk.data.path.clone(), disk.index.path.clone()),
            SharedStringStore::Memory(_) => panic!("test threshold should force disk store"),
        };
        assert!(data_path.exists());
        assert!(index_path.exists());

        drop(store);

        assert!(!data_path.exists());
        assert!(!index_path.exists());
    }

    #[test]
    fn keeps_partial_shared_string_store_after_malformed_xml() {
        let result = SharedStringStore::parse(
            Cursor::new(br#"<sst><si><t>first</t></si><"#.as_slice()),
            "xl/sharedStrings.xml",
        )
        .unwrap();
        let mut store = result.value;

        assert_eq!(store.lookup(0).unwrap().as_deref(), Some("first"));
        assert_eq!(result.warnings.len(), 1);
    }
}
