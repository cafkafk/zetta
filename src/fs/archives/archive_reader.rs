use std::io;
use std::path::Path;

use super::ArchiveEntry;

pub enum ArchiveFormat {
    Tar,
    Unknown,
}

pub trait ArchiveReader {
    fn read_dir(path: &Path) -> io::Result<Vec<Result<ArchiveEntry, Error>>>;
}

impl ArchiveFormat {
    pub fn from_extension(extension: &str) -> Option<ArchiveFormat> {
        match extension {
            "tar" => Some(ArchiveFormat::Tar),
            _ => None,
        }
    }
}

pub struct Error {
    message: String,
}

impl<E: std::fmt::Display + std::error::Error> From<E> for Error {
    fn from(value: E) -> Self {
        let full_message = value.to_string();
        let mut lines = full_message.lines();
        let mut message = lines.next().unwrap_or("").to_owned();
        if lines.next().is_some() {
            message += "...";
        }
        Error { message }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        fmt.write_str(self.message.as_str())
    }
}
