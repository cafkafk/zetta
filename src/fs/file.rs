//! Files, and methods and fields to access their metadata.

use std::io;
#[cfg(unix)]
use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};
#[cfg(windows)]
use std::os::windows::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use chrono::prelude::*;

use log::*;

use crate::fs::dir::Dir;
use crate::fs::feature::xattr;
use crate::fs::feature::xattr::{FileAttributes, Attribute};
use crate::fs::fields as f;

use super::mounts::all_mounts;
use super::mounts::MountedFs;


/// A **File** is a wrapper around one of Rust’s `PathBuf` values, along with
/// associated data about the file.
///
/// Each file is definitely going to have its filename displayed at least
/// once, have its file extension extracted at least once, and have its metadata
/// information queried at least once, so it makes sense to do all this at the
/// start and hold on to all the information.
pub struct File<'dir> {

    /// The filename portion of this file’s path, including the extension.
    ///
    /// This is used to compare against certain filenames (such as checking if
    /// it’s “Makefile” or something) and to highlight only the filename in
    /// colour when displaying the path.
    pub name: String,

    /// The file’s name’s extension, if present, extracted from the name.
    ///
    /// This is queried many times over, so it’s worth caching it.
    pub ext: Option<String>,

    /// The path that begat this file.
    ///
    /// Even though the file’s name is extracted, the path needs to be kept
    /// around, as certain operations involve looking up the file’s absolute
    /// location (such as searching for compiled files) or using its original
    /// path (following a symlink).
    pub path: PathBuf,

    /// A cached `metadata` (`stat`) call for this file.
    ///
    /// This too is queried multiple times, and is *not* cached by the OS, as
    /// it could easily change between invocations — but exa is so short-lived
    /// it’s better to just cache it.
    pub metadata: std::fs::Metadata,

    /// A reference to the directory that contains this file, if any.
    ///
    /// Filenames that get passed in on the command-line directly will have no
    /// parent directory reference — although they technically have one on the
    /// filesystem, we’ll never need to look at it, so it’ll be `None`.
    /// However, *directories* that get passed in will produce files that
    /// contain a reference to it, which is used in certain operations (such
    /// as looking up compiled files).
    pub parent_dir: Option<&'dir Dir>,

    /// Whether this is one of the two `--all all` directories, `.` and `..`.
    ///
    /// Unlike all other entries, these are not returned as part of the
    /// directory’s children, and are in fact added specifically by exa; this
    /// means that they should be skipped when recursing.
    pub is_all_all: bool,

    /// Whether to dereference symbolic links when querying for information.
    ///
    /// For instance, when querying the size of a symbolic link, if
    /// dereferencing is enabled, the size of the target will be displayed
    /// instead.
    pub deref_links: bool,

    /// The extended attributes of this file.
    extended_attributes: OnceLock<Vec<Attribute>>,

    /// The absolute value of this path, used to look up mount points.
    canonical_path: OnceLock<Option<PathBuf>>,
}

impl<'dir> File<'dir> {
    pub fn from_args<PD, FN>(path: PathBuf, parent_dir: PD, filename: FN, deref_links: bool) -> io::Result<File<'dir>>
    where PD: Into<Option<&'dir Dir>>,
          FN: Into<Option<String>>
    {
        let parent_dir = parent_dir.into();
        let name       = filename.into().unwrap_or_else(|| File::filename(&path));
        let ext        = File::ext(&path);

        debug!("Statting file {:?}", &path);
        let metadata   = std::fs::symlink_metadata(&path)?;
        let is_all_all = false;
        let extended_attributes = OnceLock::new();
        let absolute_path = OnceLock::new();

        Ok(File { name, ext, path, metadata, parent_dir, is_all_all, deref_links, extended_attributes, canonical_path: absolute_path })
    }

    pub fn new_aa_current(parent_dir: &'dir Dir) -> io::Result<File<'dir>> {
        let path       = parent_dir.path.clone();
        let ext        = File::ext(&path);

        debug!("Statting file {:?}", &path);
        let metadata   = std::fs::symlink_metadata(&path)?;
        let is_all_all = true;
        let parent_dir = Some(parent_dir);
        let extended_attributes = OnceLock::new();
        let absolute_path = OnceLock::new();

        Ok(File { path, parent_dir, metadata, ext, name: ".".into(), is_all_all, deref_links: false, extended_attributes, canonical_path: absolute_path })
    }

    pub fn new_aa_parent(path: PathBuf, parent_dir: &'dir Dir) -> io::Result<File<'dir>> {
        let ext        = File::ext(&path);

        debug!("Statting file {:?}", &path);
        let metadata   = std::fs::symlink_metadata(&path)?;
        let is_all_all = true;
        let parent_dir = Some(parent_dir);
        let extended_attributes = OnceLock::new();
        let absolute_path = OnceLock::new();

        Ok(File { path, parent_dir, metadata, ext, name: "..".into(), is_all_all, deref_links: false, extended_attributes, canonical_path: absolute_path })
    }

    /// A file’s name is derived from its string. This needs to handle directories
    /// such as `/` or `..`, which have no `file_name` component. So instead, just
    /// use the last component as the name.
    pub fn filename(path: &Path) -> String {
        if let Some(back) = path.components().next_back() {
            back.as_os_str().to_string_lossy().to_string()
        }
        else {
            // use the path as fallback
            error!("Path {:?} has no last component", path);
            path.display().to_string()
        }
    }

    /// Extract an extension from a file path, if one is present, in lowercase.
    ///
    /// The extension is the series of characters after the last dot. This
    /// deliberately counts dotfiles, so the “.git” folder has the extension “git”.
    ///
    /// ASCII lowercasing is used because these extensions are only compared
    /// against a pre-compiled list of extensions which are known to only exist
    /// within ASCII, so it’s alright.
    fn ext(path: &Path) -> Option<String> {
        let name = path.file_name().map(|f| f.to_string_lossy().to_string())?;

        name.rfind('.')
            .map(|p| name[p + 1 ..]
            .to_ascii_lowercase())
    }

    /// Read the extended attributes of a file path.
    fn gather_extended_attributes(path: &Path) -> Vec<Attribute> {
        if xattr::ENABLED {
            match path.symlink_attributes() {
                Ok(xattrs) => xattrs,
                Err(e) => {
                    error!("Error looking up extended attributes for {}: {}", path.display(), e);
                    Vec::new()
                }
            }
        } else {
            Vec::new()
        }
    }

    /// Get the extended attributes of a file path on demand.
    pub fn extended_attributes(&self) -> &Vec<Attribute> {
        self.extended_attributes.get_or_init(||File::gather_extended_attributes(&self.path))
    }

    /// Whether this file is a directory on the filesystem.
    pub fn is_directory(&self) -> bool {
        self.metadata.is_dir()
    }

    /// Whether this file is a directory, or a symlink pointing to a directory.
    pub fn points_to_directory(&self) -> bool {
        if self.is_directory() {
            return true;
        }

        if self.is_link() {
            let target = self.link_target();
            if let FileTarget::Ok(target) = target {
                return target.points_to_directory();
            }
        }

        false
    }

    /// If this file is a directory on the filesystem, then clone its
    /// `PathBuf` for use in one of our own `Dir` values, and read a list of
    /// its contents.
    ///
    /// Returns an IO error upon failure, but this shouldn’t be used to check
    /// if a `File` is a directory or not! For that, just use `is_directory()`.
    pub fn to_dir(&self) -> io::Result<Dir> {
        trace!("to_dir: reading dir");
        Dir::read_dir(self.path.clone())
    }

    /// Whether this file is a regular file on the filesystem — that is, not a
    /// directory, a link, or anything else treated specially.
    pub fn is_file(&self) -> bool {
        self.metadata.is_file()
    }

    /// Whether this file is both a regular file *and* executable for the
    /// current user. An executable file has a different purpose from an
    /// executable directory, so they should be highlighted differently.
    #[cfg(unix)]
    pub fn is_executable_file(&self) -> bool {
        let bit = modes::USER_EXECUTE;
        self.is_file() && (self.metadata.permissions().mode() & bit) == bit
    }

    /// Whether this file is a symlink on the filesystem.
    pub fn is_link(&self) -> bool {
        self.metadata.file_type().is_symlink()
    }

    /// Whether this file is a named pipe on the filesystem.
    #[cfg(unix)]
    pub fn is_pipe(&self) -> bool {
        self.metadata.file_type().is_fifo()
    }

    /// Whether this file is a char device on the filesystem.
    #[cfg(unix)]
    pub fn is_char_device(&self) -> bool {
        self.metadata.file_type().is_char_device()
    }

    /// Whether this file is a block device on the filesystem.
    #[cfg(unix)]
    pub fn is_block_device(&self) -> bool {
        self.metadata.file_type().is_block_device()
    }

    /// Whether this file is a socket on the filesystem.
    #[cfg(unix)]
    pub fn is_socket(&self) -> bool {
        self.metadata.file_type().is_socket()
    }

    /// Determine the absolute path without resolving symlinks.
    pub fn absolute_path(&self) -> PathBuf {
        static CWD: OnceLock<PathBuf> = OnceLock::new();
        CWD.get_or_init(|| std::env::current_dir().unwrap()).with_file_name(&self.path)
    }

    /// Determine the absolute path and resolve symlinks.
    pub fn canonical_path(&self) -> Option<&PathBuf> {
        self.canonical_path.get_or_init(|| std::fs::canonicalize(&self.path).ok()).as_ref()
    }

    /// Whether this file is a mount point
    pub fn is_mount_point(&self) -> bool {
        cfg!(any(target_os = "linux", target_os = "macos")) &&
            self.is_directory() &&
            self.canonical_path().is_some_and(|p| all_mounts().contains_key(p))
    }

    /// The filesystem device and type for a mount point
    pub fn mount_point_info(&self) -> Option<&MountedFs> {
        if cfg!(any(target_os = "linux",target_os = "macos")) {
            return self.canonical_path().and_then(|p| all_mounts().get(p));
        }
        None
    }

    /// Again assuming this file is a symlink, follows that link and returns
    /// the result of following it.
    ///
    /// For a working symlink that the user is allowed to follow,
    /// this will be the `File` object at the other end, which can then have
    /// its name, colour, and other details read.
    ///
    /// For a broken symlink, returns where the file *would* be, if it
    /// existed. If this file cannot be read at all, returns the error that
    /// we got when we tried to read it.
    pub fn link_target(&self) -> FileTarget<'dir> {

        // We need to be careful to treat the path actually pointed to by
        // this file — which could be absolute or relative — to the path
        // we actually look up and turn into a `File` — which needs to be
        // absolute to be accessible from any directory.
        debug!("Reading link {:?}", &self.path);
        let path = match std::fs::read_link(&self.path) {
            Ok(p)   => p,
            Err(e)  => return FileTarget::Err(e),
        };

        let absolute_path = self.path.with_file_name(&path);

        // Use plain `metadata` instead of `symlink_metadata` - we *want* to
        // follow links.
        match std::fs::metadata(&absolute_path) {
            Ok(metadata) => {
                let ext  = File::ext(&path);
                let name = File::filename(&path);
                let extended_attributes = OnceLock::new();
                let absolute_path_cell = OnceLock::from(Some(absolute_path));
                let file = File {
                    parent_dir: None,
                    path,
                    ext,
                    metadata,
                    name,
                    is_all_all: false,
                    deref_links: self.deref_links,
                    extended_attributes,
                    canonical_path: absolute_path_cell,
                };
                FileTarget::Ok(Box::new(file))
            }
            Err(e) => {
                error!("Error following link {:?}: {:#?}", &path, e);
                FileTarget::Broken(path)
            }
        }
    }

    /// Assuming this file is a symlink, follows that link and any further
    /// links recursively, returning the result from following the trail.
    ///
    /// For a working symlink that the user is allowed to follow,
    /// this will be the `File` object at the other end, which can then have
    /// its name, colour, and other details read.
    ///
    /// For a broken symlink, returns where the file *would* be, if it
    /// existed. If this file cannot be read at all, returns the error that
    /// we got when we tried to read it.
    pub fn link_target_recurse(&self) -> FileTarget<'dir> {
        let target = self.link_target();
        if let FileTarget::Ok(f) = target {
            if f.is_link() {
                return f.link_target_recurse();
            }
            return FileTarget::Ok(f);
        }
        target
    }

    /// This file’s number of hard links.
    ///
    /// It also reports whether this is both a regular file, and a file with
    /// multiple links. This is important, because a file with multiple links
    /// is uncommon, while you come across directories and other types
    /// with multiple links much more often. Thus, it should get highlighted
    /// more attentively.
    #[cfg(unix)]
    pub fn links(&self) -> f::Links {
        let count = self.metadata.nlink();

        f::Links {
            count,
            multiple: self.is_file() && count > 1,
        }
    }

    /// This file’s inode.
    #[cfg(unix)]
    pub fn inode(&self) -> f::Inode {
        f::Inode(self.metadata.ino())
    }

    /// This actual size the file takes up on disk, in bytes.
    #[cfg(unix)]
    pub fn blocksize(&self) -> f::Blocksize {
        if self.is_file() || self.is_link() {
            // Note that metadata.blocks returns the number of blocks
            // for 512 byte blocks according to the POSIX standard
            // even though the physical block size may be different.
            f::Blocksize::Some(self.metadata.blocks() * 512)
        }
        else {
            f::Blocksize::None
        }
    }

    /// The ID of the user that own this file. If dereferencing links, the links
    /// may be broken, in which case `None` will be returned.
    #[cfg(unix)]
    pub fn user(&self) -> Option<f::User> {
        if self.is_link() && self.deref_links {
            match self.link_target_recurse() {
               FileTarget::Ok(f) => return f.user(),
               _ => return None,
            }
        }
        Some(f::User(self.metadata.uid()))
    }

    /// The ID of the group that owns this file.
    #[cfg(unix)]
    pub fn group(&self) -> Option<f::Group> {
        if self.is_link() && self.deref_links {
            match self.link_target_recurse() {
               FileTarget::Ok(f) => return f.group(),
               _ => return None,
            }
        }
        Some(f::Group(self.metadata.gid()))
    }

    /// This file’s size, if it’s a regular file.
    ///
    /// For directories, no size is given. Although they do have a size on
    /// some filesystems, I’ve never looked at one of those numbers and gained
    /// any information from it. So it’s going to be hidden instead.
    ///
    /// Block and character devices return their device IDs, because they
    /// usually just have a file size of zero.
    ///
    /// Links will return the size of their target (recursively through other
    /// links) if dereferencing is enabled, otherwise the size of the link
    /// itself.
    #[cfg(unix)]
    pub fn size(&self) -> f::Size {
        if self.is_link() {
            let target = self.link_target();
            if let FileTarget::Ok(target) = target {
                return target.size();
            }
        }
        if self.is_directory() {
            f::Size::None
        }
        else if self.is_char_device() || self.is_block_device() {
            let device_ids = self.metadata.rdev().to_be_bytes();

            // In C-land, getting the major and minor device IDs is done with
            // preprocessor macros called `major` and `minor` that depend on
            // the size of `dev_t`, but we just take the second-to-last and
            // last bytes.
            f::Size::DeviceIDs(f::DeviceIDs {
                major: device_ids[6],
                minor: device_ids[7],
            })
        }
        else if self.is_link() && self.deref_links {
            match self.link_target() {
                FileTarget::Ok(f) => f.size(),
                _ => f::Size::None
            }
        } else {
            f::Size::Some(self.metadata.len())
        }
    }

    /// Returns the size of the file or indicates no size if it's a directory.
    ///
    /// For Windows platforms, the size of directories is not computed and will 
    /// return `Size::None`.
    #[cfg(windows)]
    pub fn size(&self) -> f::Size {
        if self.is_directory() {
            f::Size::None
        }
        else {
            f::Size::Some(self.metadata.len())
        }
    }

    /// Determines if the directory is empty or not.
    ///
    /// For Unix platforms, this function first checks the link count to quickly 
    /// determine non-empty directories. On most UNIX filesystems the link count
    /// is two plus the number of subdirectories. If the link count is less than
    /// or equal to 2, it then checks the directory contents to determine if
    /// it's truly empty. The naive approach used here checks the contents
    /// directly, as certain filesystems make it difficult to infer emptiness
    /// based on directory size alone.
    #[cfg(unix)]
    pub fn is_empty_dir(&self) -> bool {
        if self.is_directory() {
            if self.metadata.nlink() > 2 {
                // Directories will have a link count of two if they do not have any subdirectories.
                // The '.' entry is a link to itself and the '..' is a link to the parent directory.
                // A subdirectory will have a link to its parent directory increasing the link count
                // above two.  This will avoid the expensive read_dir call below when a directory
                // has subdirectories.
                false
            } else {
                self.is_empty_directory()
            }
        } else {
            false
        }
    }

    /// Determines if the directory is empty or not.
    ///
    /// For Windows platforms, this function checks the directory contents directly 
    /// to determine if it's empty. Since certain filesystems on Windows make it 
    /// challenging to infer emptiness based on directory size, this approach is used.
    #[cfg(windows)]
    pub fn is_empty_dir(&self) -> bool {
        if self.is_directory() {
            self.is_empty_directory()
        } else {
            false
        }
    }

    /// Checks the contents of the directory to determine if it's empty.
    ///
    /// This function avoids counting '.' and '..' when determining if the directory is 
    /// empty. If any other entries are found, it returns `false`.
    ///
    /// The naive approach, as one would think that this info may have been cached.
    /// but as mentioned in the size function comment above, different filesystems
    /// make it difficult to get any info about a dir by it's size, so this may be it.
    fn is_empty_directory(&self) -> bool {
        trace!("is_empty_directory: reading dir");
        match Dir::read_dir(self.path.clone()) {
            // . & .. are skipped, if the returned iterator has .next(), it's not empty
            Ok(has_files) => has_files.files(super::DotFilter::Dotfiles, None, false, false).next().is_none(),
            Err(_) => false,
        }
    }

    /// This file’s last modified timestamp, if available on this platform.
    pub fn modified_time(&self) -> Option<NaiveDateTime> {
        if self.is_link() && self.deref_links {
            return match self.link_target_recurse() {
                FileTarget::Ok(f) => f.modified_time(),
                _ => None, 
            };
        }
        self.metadata.modified().map(|st| DateTime::<Utc>::from(st).naive_utc()).ok()
    }

    /// This file’s last changed timestamp, if available on this platform.
    #[cfg(unix)]
    pub fn changed_time(&self) -> Option<NaiveDateTime> {
        if self.is_link() && self.deref_links {
            return match self.link_target_recurse() {
                FileTarget::Ok(f) => f.changed_time(),
                _ => None,
            };
        }
        NaiveDateTime::from_timestamp_opt(
            self.metadata.ctime(),
            self.metadata.ctime_nsec() as u32,
        )
    }

    #[cfg(windows)]
    pub fn changed_time(&self) -> Option<NaiveDateTime> {
        self.modified_time()
    }

    /// This file’s last accessed timestamp, if available on this platform.
    pub fn accessed_time(&self) -> Option<NaiveDateTime> {
        if self.is_link() && self.deref_links {
            return match self.link_target_recurse() {
                FileTarget::Ok(f) => f.accessed_time(),
                _ => None, 
            };
        }
        self.metadata.accessed().map(|st| DateTime::<Utc>::from(st).naive_utc()).ok()
    }

    /// This file’s created timestamp, if available on this platform.
    pub fn created_time(&self) -> Option<NaiveDateTime> {
        if self.is_link() && self.deref_links {
            return match self.link_target_recurse() {
                FileTarget::Ok(f) => f.created_time(),
                _ => None,
            };
        }
        self.metadata.created().map(|st| DateTime::<Utc>::from(st).naive_utc()).ok()
    }

    /// This file’s ‘type’.
    ///
    /// This is used a the leftmost character of the permissions column.
    /// The file type can usually be guessed from the colour of the file, but
    /// ls puts this character there.
    #[cfg(unix)]
    pub fn type_char(&self) -> f::Type {
        if self.is_file() {
            f::Type::File
        }
        else if self.is_directory() {
            f::Type::Directory
        }
        else if self.is_pipe() {
            f::Type::Pipe
        }
        else if self.is_link() {
            f::Type::Link
        }
        else if self.is_char_device() {
            f::Type::CharDevice
        }
        else if self.is_block_device() {
            f::Type::BlockDevice
        }
        else if self.is_socket() {
            f::Type::Socket
        }
        else {
            f::Type::Special
        }
    }

    #[cfg(windows)]
    pub fn type_char(&self) -> f::Type {
        if self.is_file() {
            f::Type::File
        }
        else if self.is_directory() {
            f::Type::Directory
        }
        else {
            f::Type::Special
        }
    }

    /// This file’s permissions, with flags for each bit.
    #[cfg(unix)]
    pub fn permissions(&self) -> Option<f::Permissions> {
        if self.is_link() && self.deref_links {
            // If the chain of links is broken, we instead fall through and
            // return the permissions of the original link, as would have been
            // done if we were not dereferencing.
            match self.link_target_recurse() {
                FileTarget::Ok(f)   => return f.permissions(),
                _                   => return None,
            }
        }
        let bits = self.metadata.mode();
        let has_bit = |bit| bits & bit == bit;

        Some(f::Permissions {
            user_read:      has_bit(modes::USER_READ),
            user_write:     has_bit(modes::USER_WRITE),
            user_execute:   has_bit(modes::USER_EXECUTE),

            group_read:     has_bit(modes::GROUP_READ),
            group_write:    has_bit(modes::GROUP_WRITE),
            group_execute:  has_bit(modes::GROUP_EXECUTE),

            other_read:     has_bit(modes::OTHER_READ),
            other_write:    has_bit(modes::OTHER_WRITE),
            other_execute:  has_bit(modes::OTHER_EXECUTE),

            sticky:         has_bit(modes::STICKY),
            setgid:         has_bit(modes::SETGID),
            setuid:         has_bit(modes::SETUID),
        })
    }

    #[cfg(windows)]
    pub fn attributes(&self) -> f::Attributes {
        let bits = self.metadata.file_attributes();
        let has_bit = |bit| bits & bit == bit;

        // https://docs.microsoft.com/en-us/windows/win32/fileio/file-attribute-constants
        f::Attributes {
            directory:      has_bit(0x10),
            archive:        has_bit(0x20),
            readonly:       has_bit(0x1),
            hidden:         has_bit(0x2),
            system:         has_bit(0x4),
            reparse_point:  has_bit(0x400),
        }
    }

    /// This file’s security context field.
    pub fn security_context(&self) -> f::SecurityContext<'_> {
        let context = match self.extended_attributes().iter().find(|a| a.name == "security.selinux") {
            Some(attr) => f::SecurityContextType::SELinux(&attr.value),
            None       => f::SecurityContextType::None
        };

        f::SecurityContext { context }
    }
}


impl<'a> AsRef<File<'a>> for File<'a> {
    fn as_ref(&self) -> &File<'a> {
        self
    }
}


/// The result of following a symlink.
pub enum FileTarget<'dir> {

    /// The symlink pointed at a file that exists.
    Ok(Box<File<'dir>>),

    /// The symlink pointed at a file that does not exist. Holds the path
    /// where the file would be, if it existed.
    Broken(PathBuf),

    /// There was an IO error when following the link. This can happen if the
    /// file isn’t a link to begin with, but also if, say, we don’t have
    /// permission to follow it.
    Err(io::Error),

    // Err is its own variant, instead of having the whole thing be inside an
    // `io::Result`, because being unable to follow a symlink is not a serious
    // error — we just display the error message and move on.
}

impl<'dir> FileTarget<'dir> {

    /// Whether this link doesn’t lead to a file, for whatever reason. This
    /// gets used to determine how to highlight the link in grid views.
    pub fn is_broken(&self) -> bool {
        matches!(self, Self::Broken(_) | Self::Err(_))
    }
}


/// More readable aliases for the permission bits exposed by libc.
#[allow(trivial_numeric_casts)]
#[cfg(unix)]
mod modes {

    // The `libc::mode_t` type’s actual type varies, but the value returned
    // from `metadata.permissions().mode()` is always `u32`.
    pub type Mode = u32;

    pub const USER_READ: Mode     = libc::S_IRUSR as Mode;
    pub const USER_WRITE: Mode    = libc::S_IWUSR as Mode;
    pub const USER_EXECUTE: Mode  = libc::S_IXUSR as Mode;

    pub const GROUP_READ: Mode    = libc::S_IRGRP as Mode;
    pub const GROUP_WRITE: Mode   = libc::S_IWGRP as Mode;
    pub const GROUP_EXECUTE: Mode = libc::S_IXGRP as Mode;

    pub const OTHER_READ: Mode    = libc::S_IROTH as Mode;
    pub const OTHER_WRITE: Mode   = libc::S_IWOTH as Mode;
    pub const OTHER_EXECUTE: Mode = libc::S_IXOTH as Mode;

    pub const STICKY: Mode        = libc::S_ISVTX as Mode;
    pub const SETGID: Mode        = libc::S_ISGID as Mode;
    pub const SETUID: Mode        = libc::S_ISUID as Mode;
}


#[cfg(test)]
mod ext_test {
    use super::File;
    use std::path::Path;

    #[test]
    fn extension() {
        assert_eq!(Some("dat".to_string()), File::ext(Path::new("fester.dat")))
    }

    #[test]
    fn dotfile() {
        assert_eq!(Some("vimrc".to_string()), File::ext(Path::new(".vimrc")))
    }

    #[test]
    fn no_extension() {
        assert_eq!(None, File::ext(Path::new("jarlsberg")))
    }
}


#[cfg(test)]
mod filename_test {
    use super::File;
    use std::path::Path;

    #[test]
    fn file() {
        assert_eq!("fester.dat", File::filename(Path::new("fester.dat")))
    }

    #[test]
    fn no_path() {
        assert_eq!("foo.wha", File::filename(Path::new("/var/cache/foo.wha")))
    }

    #[test]
    fn here() {
        assert_eq!(".", File::filename(Path::new(".")))
    }

    #[test]
    fn there() {
        assert_eq!("..", File::filename(Path::new("..")))
    }

    #[test]
    fn everywhere() {
        assert_eq!("..", File::filename(Path::new("./..")))
    }

    #[test]
    #[cfg(unix)]
    fn topmost() {
        assert_eq!("/", File::filename(Path::new("/")))
    }
}
