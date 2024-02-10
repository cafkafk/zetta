use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::slice::Iter as SliceIter;

use chrono::NaiveDateTime;

use crate::fs::feature::xattr::Attribute;
use crate::fs::fields as f;
use crate::fs::file::FileTarget;
use crate::fs::{Dir, File, Filelike};

use super::mounts::MountedFs;

#[derive(Debug, Clone, PartialEq)]
pub enum ArchiveInspection {
    Always,
    Never,
    // TODO: option to limit file size (especially for compressed archives)
}

#[derive(Clone)]
pub struct Owner {
    pub id: u64,
    pub name: Option<String>,
}

#[derive(Clone)]
pub struct ArchiveEntry {
    name: String,
    path: PathBuf,
    size: u64,
    #[cfg(unix)]
    permissions: Option<f::Permissions>,
    #[cfg(unix)]
    user: Option<Owner>,
    #[cfg(unix)]
    group: Option<Owner>,
    is_directory: bool,
    is_link: bool,
    link_target: Option<PathBuf>,
    mtime: Option<u64>,
    atime: Option<u64>,
    ctime: Option<u64>,
}

impl Filelike for ArchiveEntry {
    fn path(&self) -> &PathBuf {
        &self.path
    }

    fn name(&self) -> &String {
        &self.name
    }

    fn extension(&self) -> Option<String> {
        File::extension(&self.path)
    }

    fn deref_links(&self) -> bool {
        false
    }

    fn extended_attributes(&self) -> &[Attribute] {
        &[]
    }

    fn metadata(&self) -> Option<&std::fs::Metadata> {
        None
    }

    fn parent_directory(&self) -> Option<&Dir> {
        None
    }

    fn to_dir(&self) -> Option<io::Result<Dir>> {
        None
    }

    fn is_directory(&self) -> bool {
        self.is_directory
    }

    fn points_to_directory(&self) -> bool {
        // symlinks in archive will always be handled as broken links,
        // thus no link will ever be a directory
        self.is_directory
    }

    fn is_file(&self) -> bool {
        !self.is_link && !self.is_directory
    }

    #[cfg(unix)]
    fn is_executable_file(&self) -> bool {
        false
    }

    fn is_link(&self) -> bool {
        self.is_link
    }

    #[cfg(unix)]
    fn is_pipe(&self) -> bool {
        false
    }

    #[cfg(unix)]
    fn is_char_device(&self) -> bool {
        false
    }

    #[cfg(unix)]
    fn is_block_device(&self) -> bool {
        false
    }

    #[cfg(unix)]
    fn is_socket(&self) -> bool {
        false
    }

    fn absolute_path(&self) -> Option<&PathBuf> {
        // TODO: could be argued that this should also include path to archive;
        //       but that would be kind of ugly to implement since every ArchiveEntry
        //       either needs to store the entire path or keep a reference to the
        //       archive which would then have to have mutable content (since it has
        //       to be constructed before any entry is created); thus, I think this
        //       behavior is sufficient
        Some(&self.path)
    }

    fn is_mount_point(&self) -> bool {
        false
    }

    fn mount_point_info(&self) -> Option<&MountedFs> {
        None
    }

    fn link_target<'a>(&self) -> FileTarget<'a> {
        if let Some(link_target) = &self.link_target {
            FileTarget::Broken(link_target.clone())
        } else {
            FileTarget::Err(io::Error::new(io::ErrorKind::Other, "no link target"))
        }
    }

    fn link_target_recurse<'a>(&self) -> FileTarget<'a> {
        self.link_target()
    }

    #[cfg(unix)]
    fn links(&self) -> f::Links {
        f::Links {
            count: 0,
            multiple: false,
        }
    }

    #[cfg(unix)]
    fn inode(&self) -> f::Inode {
        // inode 0 can be used to indicate that there is no inode
        f::Inode(0)
    }

    #[cfg(unix)]
    fn blocksize(&self) -> f::Blocksize {
        f::Blocksize::None
    }

    #[cfg(unix)]
    fn user(&self) -> Option<f::User> {
        self.user.as_ref().map(|o| f::User(o.id as u32))
    }

    #[cfg(unix)]
    fn group(&self) -> Option<f::Group> {
        self.group.as_ref().map(|o| f::Group(o.id as u32))
    }

    fn size(&self) -> f::Size {
        if self.is_directory || self.is_link {
            f::Size::None
        } else {
            f::Size::Some(self.size)
        }
    }

    fn length(&self) -> u64 {
        self.size
    }

    fn is_recursive_size(&self) -> bool {
        false
    }

    fn is_empty_dir(&self) -> bool {
        // TODO: could check if there is any other entry in archive with "{path}/" as prefix;
        //       but kind of expensive for very little benefit
        false
    }

    fn modified_time(&self) -> Option<NaiveDateTime> {
        NaiveDateTime::from_timestamp_opt(self.mtime? as i64, 0)
    }

    fn changed_time(&self) -> Option<NaiveDateTime> {
        NaiveDateTime::from_timestamp_opt(self.ctime? as i64, 0)
    }

    fn accessed_time(&self) -> Option<NaiveDateTime> {
        NaiveDateTime::from_timestamp_opt(self.atime? as i64, 0)
    }

    fn created_time(&self) -> Option<NaiveDateTime> {
        None
    }

    fn type_char(&self) -> f::Type {
        if self.is_link {
            f::Type::Link
        } else if self.is_directory {
            f::Type::Directory
        } else {
            f::Type::File
        }
    }

    #[cfg(unix)]
    fn permissions(&self) -> Option<f::Permissions> {
        self.permissions
    }

    #[cfg(windows)]
    fn attributes(&self) -> f::Attributes {
        f::Attributes {
            archive: false,
            directory: false,
            readonly: true,
            hidden: false,
            system: false,
            reparse_point: false,
        }
    }

    fn security_context(&self) -> f::SecurityContext<'_> {
        f::SecurityContext {
            context: f::SecurityContextType::None,
        }
    }

    fn flags(&self) -> f::Flags {
        f::Flags(0)
    }
}

impl AsRef<ArchiveEntry> for ArchiveEntry {
    fn as_ref(&self) -> &ArchiveEntry {
        self
    }
}

pub enum ArchiveFormat {
    Tar,
    Unknown,
}

trait ArchiveReader {
    fn read_dir(path: &Path) -> io::Result<Vec<Result<ArchiveEntry, Error>>>;
}

struct TarReader {}

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
                    if let Some(p) = x.path.parent() {
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
            ArchiveFormat::Tar => TarReader::read_dir(&path),
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
