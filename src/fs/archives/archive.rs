use std::io;
use std::path::PathBuf;
use std::slice::Iter as SliceIter;

use crate::fs::{File, Filelike};

use super::{ArchiveEntry, ArchiveFormat, Error};
#[cfg(feature = "archive-inspection")]
use super::{ArchiveReader, TarReader};

pub struct Archive {
    pub format: ArchiveFormat,
    pub path: PathBuf,

    contents: Vec<Result<ArchiveEntry, Error>>,
}

#[derive(Clone)]
pub struct ArchiveIterator<'archive> {
    inner: SliceIter<'archive, Result<ArchiveEntry, Error>>,
    /// Path in archive whose content is iterated over
    path: PathBuf,
}

impl<'archive> Iterator for ArchiveIterator<'archive> {
    type Item = &'archive Result<ArchiveEntry, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(it) = self.inner.next() {
            if it.is_err()
                || it.as_ref().is_ok_and(|x| {
                    if let Some(p) = x.path().parent() {
                        p == self.path
                    } else {
                        false
                    }
                })
            {
                return Some(it);
            }
        }
        None
    }
}

impl Archive {
    pub fn from_path(path: PathBuf) -> io::Result<Self> {
        let extension = File::extension(path.as_path()).unwrap_or(String::new());
        let format =
            ArchiveFormat::from_extension(extension.as_str()).unwrap_or(ArchiveFormat::Unknown);
        let contents = match format {
            #[cfg(feature = "archive-inspection")]
            ArchiveFormat::Tar => TarReader::read_dir(&path),
            #[cfg(not(feature = "archive-inspection"))]
            ArchiveFormat::Tar => Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "Archive inspection not supported",
            )),
            ArchiveFormat::Unknown => {
                return Err(io::Error::new(
                    io::ErrorKind::Unsupported,
                    "Unsupported archive format",
                ))
            }
        }?;
        // TODO: could check if any in `contents` is Err and then
        //       return Err for silent fail
        Ok(Archive {
            format,
            path,
            contents,
        })
    }

    /// Produce an iterator of IO results of trying to read all the files in
    /// this directory.
    pub fn files(&self, root: PathBuf) -> ArchiveIterator<'_> {
        ArchiveIterator {
            inner: self.contents.iter(),
            path: root,
        }
    }
}
