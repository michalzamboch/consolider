mod copy;
mod paths;
mod scan;

use std::{
    fmt, io,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};

pub use copy::execute;
pub use scan::analyze;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Invalid(String),
    Io {
        operation: &'static str,
        path: PathBuf,
        source: io::Error,
    },
    Cancelled,
}

impl Error {
    fn io(operation: &'static str, path: &Path, source: io::Error) -> Self {
        Self::Io {
            operation,
            path: path.to_owned(),
            source,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(message) => f.write_str(message),
            Self::Io {
                operation,
                path,
                source,
            } => write!(f, "Cannot {operation} {}: {source}", path.display()),
            Self::Cancelled => f.write_str("Operation cancelled."),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Request {
    pub sources: Vec<PathBuf>,
    pub destination: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlannedFile {
    pub source: PathBuf,
    pub original_path: PathBuf,
    pub output_path: PathBuf,
    pub bytes: u64,
    pub(crate) hash: blake3::Hash,
}

impl PlannedFile {
    pub fn renamed(&self) -> bool {
        self.original_path != self.output_path
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ScannedFile {
    source: PathBuf,
    relative: PathBuf,
    bytes: u64,
    hash: blake3::Hash,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plan {
    pub request: Request,
    pub files: Vec<PlannedFile>,
    pub directories: Vec<PathBuf>,
    pub duplicates: usize,
    pub warnings: Vec<String>,
    total_bytes: u64,
    renamed_files: usize,
    // Includes duplicate inputs so a changed duplicate also invalidates the preview.
    scanned: Vec<ScannedFile>,
}

impl Plan {
    pub fn input_files(&self) -> usize {
        self.scanned.len()
    }
    pub fn total_bytes(&self) -> u64 {
        self.total_bytes
    }
    pub fn renamed_files(&self) -> usize {
        self.renamed_files
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Scanning,
    Verifying,
    Copying,
}

#[derive(Clone, Debug)]
pub struct Progress {
    pub phase: Phase,
    pub files: usize,
    pub bytes: u64,
    pub current_path: PathBuf,
}

fn check_cancel(cancel: &AtomicBool) -> Result<()> {
    if cancel.load(Ordering::Relaxed) {
        Err(Error::Cancelled)
    } else {
        Ok(())
    }
}
