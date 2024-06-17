use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[cfg(unix)]
use crate::fs::fields as f;
use crate::fs::File;

#[cfg(unix)]
use super::Owner;
use super::{ArchiveEntry, ArchiveReader, Error};

pub struct TarReader {}

impl TarReader {
    /// Get size of entry; the size written in the header field takes precedence
    pub fn size<R: std::io::Read>(entry: &tar::Entry<'_, R>) -> u64 {
        entry.header().size().unwrap_or(entry.size())
    }

    pub fn path<R: std::io::Read>(entry: &tar::Entry<'_, R>) -> io::Result<PathBuf> {
        let mut path = entry.header().path();
        if path.is_err() {
            path = entry.path();
        }
        path.map(|p| p.to_path_buf())
    }

    pub fn is_directory<R: std::io::Read>(entry: &tar::Entry<'_, R>) -> bool {
        entry.header().entry_type().is_dir()
    }

    pub fn is_link<R: std::io::Read>(entry: &tar::Entry<'_, R>) -> bool {
        entry.header().entry_type().is_symlink()
    }

    pub fn link_target<R: std::io::Read>(entry: &tar::Entry<'_, R>) -> io::Result<Option<PathBuf>> {
        entry
            .header()
            .link_name()
            .map(|o| o.map(|p| p.to_path_buf()))
    }

    #[cfg(unix)]
    pub fn uid<R: std::io::Read>(entry: &tar::Entry<'_, R>) -> io::Result<u64> {
        entry.header().uid()
    }

    #[cfg(unix)]
    pub fn gid<R: std::io::Read>(entry: &tar::Entry<'_, R>) -> io::Result<u64> {
        entry.header().gid()
    }

    #[cfg(unix)]
    pub fn username<R: std::io::Read>(
        entry: &tar::Entry<'_, R>,
    ) -> Result<Option<String>, std::str::Utf8Error> {
        entry.header().username().map(|o| o.map(str::to_owned))
    }

    #[cfg(unix)]
    pub fn groupname<R: std::io::Read>(
        entry: &tar::Entry<'_, R>,
    ) -> Result<Option<String>, std::str::Utf8Error> {
        entry.header().groupname().map(|o| o.map(str::to_owned))
    }

    #[cfg(unix)]
    pub fn permissions<R: std::io::Read>(entry: &tar::Entry<'_, R>) -> io::Result<f::Permissions> {
        let mode = entry.header().mode()?;
        Ok(f::Permissions::from_mode(mode))
    }

    pub fn mtime<R: std::io::Read>(entry: &tar::Entry<'_, R>) -> io::Result<u64> {
        entry.header().mtime()
    }

    pub fn atime<R: std::io::Read>(entry: &tar::Entry<'_, R>) -> io::Result<u64> {
        entry
            .header()
            .as_gnu()
            .ok_or(io::Error::new(
                io::ErrorKind::Unsupported,
                "archive header does not support atime",
            ))
            .and_then(tar::GnuHeader::atime)
    }

    pub fn ctime<R: std::io::Read>(entry: &tar::Entry<'_, R>) -> io::Result<u64> {
        entry
            .header()
            .as_gnu()
            .ok_or(io::Error::new(
                io::ErrorKind::Unsupported,
                "archive header does not support ctime",
            ))
            .and_then(tar::GnuHeader::ctime)
    }

    pub fn tar_entry<R: std::io::Read>(entry: &tar::Entry<'_, R>) -> Result<ArchiveEntry, Error> {
        let path = TarReader::path(entry);
        match path {
            Ok(path) => Ok(ArchiveEntry {
                name: File::filename(&path),
                path,
                size: TarReader::size(entry),
                #[cfg(unix)]
                permissions: Some(TarReader::permissions(entry)?),
                #[cfg(unix)]
                user: Some(Owner {
                    id: TarReader::uid(entry)?,
                    name: TarReader::username(entry)?,
                }),
                #[cfg(unix)]
                group: Some(Owner {
                    id: TarReader::gid(entry)?,
                    name: TarReader::groupname(entry)?,
                }),
                mtime: Some(TarReader::mtime(entry)?),
                atime: TarReader::atime(entry).ok(),
                ctime: TarReader::ctime(entry).ok(),
                link_target: TarReader::link_target(entry)?,
                is_link: TarReader::is_link(entry),
                is_directory: TarReader::is_directory(entry),
            }),
            Err(e) => Err(e.into()),
        }
    }
}

impl ArchiveReader for TarReader {
    fn read_dir(path: &Path) -> io::Result<Vec<Result<ArchiveEntry, Error>>> {
        let mut result = Vec::new();
        let file_content = fs::File::open(path)?;
        tar::Archive::new(file_content).entries().map(|entries| {
            for entry in entries {
                match entry {
                    Ok(entry) => result.push(TarReader::tar_entry(&entry)),
                    Err(error) => result.push(Err(error.into())),
                }
            }
        })?;
        Ok(result)
    }
}
