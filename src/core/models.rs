use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
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

#[derive(Debug)]
pub struct Snippet {
    metadata: Metadata,
    path: PathBuf,
}

impl Snippet {
    const SUFFIX: &str = "snipp";

    /// Create a new Snippet.
    pub fn create(
        name: impl Into<String>,
        description: Option<impl Into<String>>,
        source: impl AsRef<Path>,
        output: impl AsRef<Path>,
    ) -> Result<Self> {
        let metadata = Metadata {
            name: name.into(),
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

        for entry in walk {
            let entry = entry?;
            let path = &entry.path();

            if entry.metadata()?.is_dir() {
                continue;
            }

            let relative = path.strip_prefix(origin)?;
            let relative = contents.join(relative);
            zf.start_file_from_path(relative, opt)?;

            let contents: Vec<u8> = std::fs::read(&entry.path())?;
            zf.write(&contents)?;
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
