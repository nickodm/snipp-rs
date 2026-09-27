use anyhow::{Result, bail};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use thiserror::Error;
use walkdir::WalkDir;
use zip::ZipArchive;
use zip::write::{SimpleFileOptions, ZipWriter};

#[derive(Debug, Serialize, Deserialize)]
pub struct Metadata {
    name: String,
    description: Option<String>,
    creation_date: Option<DateTime<Utc>>,
    software_version: (u8, u8, u8),
}

impl Metadata {
    /// The name of the file where the metadata will be saved.
    pub const FILENAME: &str = "metadata.json";

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    pub fn creation_date(&self) -> Option<&DateTime<Utc>> {
        self.creation_date.as_ref()
    }
}

pub const NAME_MIN_LEN: usize = 3;

#[derive(Error, Debug)]
pub enum InvalidNameError {
    #[error("Name is too short. [minimum length: {NAME_MIN_LEN}]")]
    TooShort,
    #[error("Invalid char: '{0}'.")]
    InvalidChar(char),
    #[error("Name cannot include any spaces. Replace it with '-' or '_'.")]
    Spaces,
    #[error("The name cannot start with '{0}'.")]
    BadStart(char),
}

/// Check whether a snippet's name only contains letters, numbers, underscores and dashes.
pub fn validate_name<S: ToString>(name: S) -> Result<(), InvalidNameError> {
    let name = name.to_string();

    if name.len() < NAME_MIN_LEN {
        return Err(InvalidNameError::TooShort);
    } else if name.starts_with("-") {
        let c = name.chars().next().unwrap();
        return Err(InvalidNameError::BadStart(c));
    }

    let chars = name.chars();

    for c in chars {
        if c == ' ' {
            return Err(InvalidNameError::Spaces);
        }

        let is_valid = (c >= 'A' && c <= 'Z')
            || (c >= 'a' && c <= 'z')
            || (c >= '0' && c <= '9')
            || (c == '-')
            || (c == '_');

        if !is_valid {
            return Err(InvalidNameError::InvalidChar(c));
        }
    }

    Ok(())
}

#[test]
fn test_name_validation() {
    assert!(validate_name("my_snippet").is_ok());
    assert!(validate_name("my-snippet").is_ok());
    assert!(validate_name("mysnippet").is_ok());
    assert!(validate_name("mysnippet3").is_ok());
    assert!(validate_name("MY_SNIPPET").is_ok());
    assert!(validate_name("mys").is_ok());

    assert!(validate_name("sn").is_err());
    assert!(validate_name("my snippet").is_err());
    assert!(validate_name("my").is_err());
    assert!(validate_name("").is_err());
    assert!(validate_name("my@snippet").is_err());
    assert!(validate_name("-mysnippet").is_err());
}

#[derive(Debug)]
pub struct Snippet {
    metadata: Metadata,
    path: PathBuf,
}

impl Snippet {
    pub const SUFFIX: &str = "snipp";

    /// Create a new Snippet.
    pub fn create(
        name: impl Into<String>,
        description: Option<impl Into<String>>,
        source: impl AsRef<Path>,
        output: impl AsRef<Path>,
    ) -> Result<Self> {
        let name = name.into();
        validate_name(&name)?;

        let metadata = Metadata {
            name,
            description: description.map(|s| s.into()),
            creation_date: Some(Utc::now()),
            software_version: (0, 1, 0),
        };

        let path = output.as_ref().to_path_buf().with_extension(Self::SUFFIX);
        let s = Self {
            metadata,
            path: path.clone(),
        };

        s.compress(source, output)?;
        Ok(s)
    }

    /// Load a built snippet from `path`.
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();

        let f = File::open(path)?;
        let mut zf = ZipArchive::new(f)?;
        let metadata: Metadata;

        {
            let metafile = zf.by_name(Metadata::FILENAME)?;
            metadata = serde_json::from_reader(metafile)?;
        }

        validate_name(&metadata.name)?;

        Ok(Self {
            metadata,
            path: path.to_path_buf(),
        })
    }

    pub fn metadata(&self) -> &Metadata {
        &self.metadata
    }

    fn compress(&self, source: impl AsRef<Path>, destiny: impl AsRef<Path>) -> Result<()> {
        let origin = source.as_ref();
        let destiny = destiny.as_ref().to_path_buf().with_extension(Self::SUFFIX);

        let f = File::create(&destiny)?;
        let mut zf = ZipWriter::new(f);
        let opt = SimpleFileOptions::default();

        zf.start_file(Metadata::FILENAME, opt)?;
        zf.write(&serde_json::to_vec_pretty(&self.metadata)?)?;

        let walk = WalkDir::new(origin);
        let contents = PathBuf::from("contents");

        let mut buffer = [0; 4096];

        for entry in walk {
            let entry = entry?;
            let path = &entry.path();

            if entry.metadata()?.is_dir() {
                continue;
            }

            let relative = path.strip_prefix(origin)?;
            let relative = contents.join(relative);
            zf.start_file_from_path(relative, opt)?;

            let mut file = File::open(path)?;

            loop {
                let read: usize = file.read(&mut buffer)?;

                if read == 0 {
                    break;
                }

                zf.write(&buffer[..read])?;
            }
        }

        Ok(())
    }

    /// Extract all the files inside the snippet's contents. It does not include any file outside
    /// the `contents/` dir.
    pub fn deploy<P: AsRef<Path>>(&self, output: P) -> Result<()> {
        let output = output.as_ref();
        let f = File::open(&self.path)?;
        let mut zf = ZipArchive::new(f)?;
        let mut buffer = [0u8; 8192]; // 8 Kib
        let mut i = 0;

        while let Ok(mut zfile) = zf.by_index(i) {
            let path = match zfile.enclosed_name() {
                Some(p) => p,
                None => zfile.mangled_name(),
            };

            if !path.starts_with("contents/") {
                i += 1;
                continue;
            }

            let relative = path.strip_prefix("contents/")?;
            let dest_path = output.join(relative);

            if let Some(parent) = dest_path.parent() {
                fs::create_dir_all(parent)?;
            }

            let mut destiny = File::create(output.join(relative))?;

            loop {
                let read = zfile.read(&mut buffer)?;

                if read == 0 {
                    break;
                }

                destiny.write(&buffer[..read])?;
            }

            i += 1;
        }

        Ok(())
    }

    pub fn iter(&self) -> Result<SnippIter> {
        let f = File::open(&self.path)?;
        let zf = ZipArchive::new(f)?;
        Ok(SnippIter::new(zf))
    }

    /// Count the files inside the snippet's contents. It does not include the metadata files that
    /// are not deployed.
    pub fn file_count(&self) -> Result<usize> {
        Ok(self
            .iter()?
            .filter(|name| name.starts_with("contents/"))
            .count())
    }
}

pub struct SnippIter {
    zf: ZipArchive<File>,
    index: usize,
}

impl SnippIter {
    pub fn new(zf: ZipArchive<File>) -> Self {
        Self { zf, index: 0 }
    }
}

impl Iterator for SnippIter {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.zf.len() {
            return None;
        }

        let zfile = self.zf.by_index(self.index).ok()?;
        let filename = zfile.name().to_string();
        self.index += 1;
        Some(filename)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.zf.len().saturating_sub(self.index);
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for SnippIter {}
