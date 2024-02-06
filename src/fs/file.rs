//! Files, and methods and fields to access their metadata.

#[cfg(unix)]
use std::collections::HashMap;
use std::io;
#[cfg(unix)]
use std::os::unix::fs::{FileTypeExt, MetadataExt};
#[cfg(windows)]
use std::os::windows::fs::MetadataExt;
use std::path::{Path, PathBuf};
#[cfg(unix)]
use std::str;
#[cfg(unix)]
use std::sync::Mutex;
use std::sync::OnceLock;

use chrono::prelude::*;

use log::*;
#[cfg(unix)]
use once_cell::sync::Lazy;

use crate::fs::archive::Archive;
use crate::fs::dir::Dir;
use crate::fs::feature::xattr;
use crate::fs::feature::xattr::{Attribute, FileAttributes};
use crate::fs::fields as f;
use crate::fs::fields::SecurityContextType;
use crate::fs::filelike::Filelike;
use crate::fs::recursive_size::RecursiveSize;

use super::mounts::all_mounts;
use super::mounts::MountedFs;

// Maps (device_id, inode) => (size_in_bytes, size_in_blocks)
// Mutex::new is const but HashMap::new is not const requiring us to use lazy
// initialization.
// TODO: Replace with std::sync::LazyLock when it is stable.
#[allow(clippy::type_complexity)]
#[cfg(unix)]
static DIRECTORY_SIZE_CACHE: Lazy<Mutex<HashMap<(u64, u64), (u64, u64)>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

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

    /// The recursive directory size when total_size is used.
    recursive_size: RecursiveSize,

    /// The extended attributes of this file.
    extended_attributes: OnceLock<Vec<Attribute>>,

    /// The absolute value of this path, used to look up mount points.
    absolute_path: OnceLock<Option<PathBuf>>,
}

impl<'dir> File<'dir> {
    pub fn from_args<PD, FN>(
        path: PathBuf,
        parent_dir: PD,
        filename: FN,
        deref_links: bool,
        total_size: bool,
    ) -> io::Result<File<'dir>>
    where
        PD: Into<Option<&'dir Dir>>,
        FN: Into<Option<String>>,
    {
        let parent_dir = parent_dir.into();
        let name = filename.into().unwrap_or_else(|| File::filename(&path));
        let ext = File::extension(&path);

        debug!("Statting file {:?}", &path);
        let metadata = std::fs::symlink_metadata(&path)?;
        let is_all_all = false;
        let extended_attributes = OnceLock::new();
        let absolute_path = OnceLock::new();
        let recursive_size = if total_size {
            RecursiveSize::Unknown
        } else {
            RecursiveSize::None
        };

        let mut file = File {
            name,
            ext,
            path,
            metadata,
            parent_dir,
            is_all_all,
            deref_links,
            recursive_size,
            extended_attributes,
            absolute_path,
        };

        if total_size {
            file.recursive_size = file.recursive_directory_size();
        }

        Ok(file)
    }

    fn new_aa(
        path: PathBuf,
        parent_dir: &'dir Dir,
        name: &'static str,
        total_size: bool,
    ) -> io::Result<File<'dir>> {
        let ext = File::extension(&path);

        debug!("Statting file {:?}", &path);
        let metadata = std::fs::symlink_metadata(&path)?;
        let is_all_all = true;
        let parent_dir = Some(parent_dir);
        let extended_attributes = OnceLock::new();
        let absolute_path = OnceLock::new();
        let recursive_size = if total_size {
            RecursiveSize::Unknown
        } else {
            RecursiveSize::None
        };

        let mut file = File {
            name: name.into(),
            ext,
            path,
            metadata,
            parent_dir,
            is_all_all,
            deref_links: false,
            extended_attributes,
            absolute_path,
            recursive_size,
        };

        if total_size {
            file.recursive_size = file.recursive_directory_size();
        }

        Ok(file)
    }

    pub fn new_aa_current(parent_dir: &'dir Dir, total_size: bool) -> io::Result<File<'dir>> {
        File::new_aa(parent_dir.path.clone(), parent_dir, ".", total_size)
    }

    pub fn new_aa_parent(
        path: PathBuf,
        parent_dir: &'dir Dir,
        total_size: bool,
    ) -> io::Result<File<'dir>> {
        File::new_aa(path, parent_dir, "..", total_size)
    }

    /// A file’s name is derived from its string. This needs to handle directories
    /// such as `/` or `..`, which have no `file_name` component. So instead, just
    /// use the last component as the name.
    pub fn filename(path: &Path) -> String {
        if let Some(back) = path.components().next_back() {
            back.as_os_str().to_string_lossy().to_string()
        } else {
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
    pub fn extension(path: &Path) -> Option<String> {
        let name = path.file_name().map(|f| f.to_string_lossy().to_string())?;

        name.rfind('.').map(|p| name[p + 1..].to_ascii_lowercase())
    }

    /// Read the extended attributes of a file path.
    fn gather_extended_attributes(&self) -> Vec<Attribute> {
        if xattr::ENABLED {
            let attributes = if self.deref_links {
                self.path.attributes()
            } else {
                self.path.symlink_attributes()
            };
            match attributes {
                Ok(xattrs) => xattrs,
                Err(e) => {
                    error!(
                        "Error looking up extended attributes for {}: {}",
                        self.path.display(),
                        e
                    );
                    Vec::new()
                }
            }
        } else {
            Vec::new()
        }
    }

    /// Re-prefixes the path pointed to by this file, if it’s a symlink, to
    /// make it an absolute path that can be accessed from whichever
    /// directory exa is being run from.
    fn reorient_target_path(&self, path: &Path) -> PathBuf {
        if path.is_absolute() {
            path.to_path_buf()
        } else if let Some(dir) = self.parent_dir {
            dir.join(path)
        } else if let Some(parent) = self.path.parent() {
            parent.join(path)
        } else {
            self.path.join(path)
        }
    }

    /// Calculate the total directory size recursively.  If not a directory `None`
    /// will be returned.  The directory size is cached for recursive directory
    /// listing.
    #[cfg(unix)]
    fn recursive_directory_size(&self) -> RecursiveSize {
        if self.is_directory() {
            let key = (self.metadata.dev(), self.metadata.ino());
            if let Some(size) = DIRECTORY_SIZE_CACHE.lock().unwrap().get(&key) {
                return RecursiveSize::Some(size.0, size.1);
            }
            Dir::read_dir(self.path.clone()).map_or(RecursiveSize::Unknown, |dir| {
                let mut size = 0;
                let mut blocks = 0;
                for file in dir
                    .files(super::DotFilter::Dotfiles, None, false, false, true)
                    .flatten()
                {
                    match file.recursive_directory_size() {
                        RecursiveSize::Some(bytes, blks) => {
                            size += bytes;
                            blocks += blks;
                        }
                        RecursiveSize::Unknown => {}
                        RecursiveSize::None => {
                            size += file.metadata.size();
                            blocks += file.metadata.blocks();
                        }
                    }
                }
                DIRECTORY_SIZE_CACHE
                    .lock()
                    .unwrap()
                    .insert(key, (size, blocks));
                RecursiveSize::Some(size, blocks)
            })
        } else {
            RecursiveSize::None
        }
    }

    /// Windows version always returns None.  The metadata for
    /// `volume_serial_number` and `file_index` are marked unstable so we can
    /// not cache the sizes.  Without caching we could end up walking the
    /// directory structure several times.
    #[cfg(windows)]
    fn recursive_directory_size(&self) -> RecursiveSize {
        RecursiveSize::None
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
            Ok(has_files) => has_files
                .files(super::DotFilter::Dotfiles, None, false, false, false)
                .next()
                .is_none(),
            Err(_) => false,
        }
    }

    /// Interpret file as archive
    pub fn to_archive(&self) -> Option<Archive> {
        Archive::from_path(self.path.clone()).ok()
    }
}

impl<'dir> Filelike for File<'dir> {
    fn path(&self) -> &PathBuf {
        &self.path
    }

    fn name(&self) -> &String {
        &self.name
    }

    fn extension(&self) -> Option<String> {
        self.ext.clone() // TODO: too many clones?
    }

    fn deref_links(&self) -> bool {
        self.deref_links
    }

    fn metadata(&self) -> Option<&std::fs::Metadata> {
        Some(&self.metadata)
    }

    fn parent_directory(&self) -> Option<&'dir Dir> {
        self.parent_dir
    }

    fn to_dir(&self) -> Option<io::Result<Dir>> {
        trace!("to_dir: reading dir");
        Some(Dir::read_dir(self.path.clone()))
    }

    /// Get the extended attributes of a file path on demand.
    fn extended_attributes(&self) -> &[Attribute] {
        self.extended_attributes
            .get_or_init(|| self.gather_extended_attributes())
    }

    /// Whether this file is a directory on the filesystem.
    fn is_directory(&self) -> bool {
        self.metadata.is_dir()
    }

    /// Whether this file is a directory, or a symlink pointing to a directory.
    fn points_to_directory(&self) -> bool {
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

    /// Whether this file is a regular file on the filesystem — that is, not a
    /// directory, a link, or anything else treated specially.
    fn is_file(&self) -> bool {
        self.metadata.is_file()
    }

    /// Whether this file is both a regular file *and* executable for the
    /// current user. An executable file has a different purpose from an
    /// executable directory, so they should be highlighted differently.
    #[cfg(unix)]
    fn is_executable_file(&self) -> bool {
        self.is_file() && self.permissions().is_some_and(|p| p.user_execute)
    }

    /// Whether this file is a symlink on the filesystem.
    fn is_link(&self) -> bool {
        self.metadata.file_type().is_symlink()
    }

    /// Whether this file is a named pipe on the filesystem.
    #[cfg(unix)]
    fn is_pipe(&self) -> bool {
        self.metadata.file_type().is_fifo()
    }

    /// Whether this file is a char device on the filesystem.
    #[cfg(unix)]
    fn is_char_device(&self) -> bool {
        self.metadata.file_type().is_char_device()
    }

    /// Whether this file is a block device on the filesystem.
    #[cfg(unix)]
    fn is_block_device(&self) -> bool {
        self.metadata.file_type().is_block_device()
    }

    /// Whether this file is a socket on the filesystem.
    #[cfg(unix)]
    fn is_socket(&self) -> bool {
        self.metadata.file_type().is_socket()
    }

    /// Determine the full path resolving all symbolic links on demand.
    fn absolute_path(&self) -> Option<&PathBuf> {
        self.absolute_path
            .get_or_init(|| {
                if self.is_link() && self.link_target().is_broken() {
                    // workaround for broken symlinks to get absolute path for parent and then
                    // append name of file; std::fs::canonicalize requires all path components
                    // (including the last one) to exist
                    self.path
                        .parent()
                        .and_then(|parent| std::fs::canonicalize(parent).ok())
                        .map(|p| p.join(self.name.clone()))
                } else {
                    std::fs::canonicalize(&self.path).ok()
                }
            })
            .as_ref()
    }

    /// Whether this file is a mount point
    fn is_mount_point(&self) -> bool {
        cfg!(any(target_os = "linux", target_os = "macos"))
            && self.is_directory()
            && self
                .absolute_path()
                .is_some_and(|p| all_mounts().contains_key(p))
    }

    /// The filesystem device and type for a mount point
    fn mount_point_info(&self) -> Option<&MountedFs> {
        if cfg!(any(target_os = "linux", target_os = "macos")) {
            return self.absolute_path().and_then(|p| all_mounts().get(p));
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
    fn link_target<'a>(&self) -> FileTarget<'a> {
        // We need to be careful to treat the path actually pointed to by
        // this file — which could be absolute or relative — to the path
        // we actually look up and turn into a `File` — which needs to be
        // absolute to be accessible from any directory.
        debug!("Reading link {:?}", &self.path);
        let path = match std::fs::read_link(&self.path) {
            Ok(p) => p,
            Err(e) => return FileTarget::Err(e),
        };

        let absolute_path = self.reorient_target_path(&path);

        // Use plain `metadata` instead of `symlink_metadata` - we *want* to
        // follow links.
        match std::fs::metadata(&absolute_path) {
            Ok(metadata) => {
                let ext = File::extension(&path);
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
                    absolute_path: absolute_path_cell,
                    recursive_size: RecursiveSize::None,
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
    fn link_target_recurse<'a>(&self) -> FileTarget<'a> {
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
    fn links(&self) -> f::Links {
        let count = self.metadata.nlink();

        f::Links {
            count,
            multiple: self.is_file() && count > 1,
        }
    }

    /// This file’s inode.
    #[cfg(unix)]
    fn inode(&self) -> f::Inode {
        f::Inode(self.metadata.ino())
    }

    /// This actual size the file takes up on disk, in bytes.
    #[cfg(unix)]
    fn blocksize(&self) -> f::Blocksize {
        if self.deref_links && self.is_link() {
            match self.link_target() {
                FileTarget::Ok(f) => f.blocksize(),
                _ => f::Blocksize::None,
            }
        } else if self.is_directory() {
            self.recursive_size.map_or(f::Blocksize::None, |_, blocks| {
                f::Blocksize::Some(blocks * 512)
            })
        } else if self.is_file() {
            // Note that metadata.blocks returns the number of blocks
            // for 512 byte blocks according to the POSIX standard
            // even though the physical block size may be different.
            f::Blocksize::Some(self.metadata.blocks() * 512)
        } else {
            // directory or symlinks
            f::Blocksize::None
        }
    }

    /// The ID of the user that own this file. If dereferencing links, the links
    /// may be broken, in which case `None` will be returned.
    #[cfg(unix)]
    fn user(&self) -> Option<f::User> {
        if self.is_link() && self.deref_links {
            return match self.link_target_recurse() {
                FileTarget::Ok(f) => f.user(),
                _ => None,
            };
        }
        Some(f::User(self.metadata.uid()))
    }

    /// The ID of the group that owns this file.
    #[cfg(unix)]
    fn group(&self) -> Option<f::Group> {
        if self.is_link() && self.deref_links {
            return match self.link_target_recurse() {
                FileTarget::Ok(f) => f.group(),
                _ => None,
            };
        }
        Some(f::Group(self.metadata.gid()))
    }

    /// This file’s size, if it’s a regular file.
    ///
    /// For directories, the recursive size or no size is given depending on
    /// flags. Although they do have a size on some filesystems, I’ve never
    /// looked at one of those numbers and gained any information from it.
    ///
    /// Block and character devices return their device IDs, because they
    /// usually just have a file size of zero.
    ///
    /// Links will return the size of their target (recursively through other
    /// links) if dereferencing is enabled, otherwise None.
    #[cfg(unix)]
    fn size(&self) -> f::Size {
        if self.deref_links && self.is_link() {
            match self.link_target() {
                FileTarget::Ok(f) => f.size(),
                _ => f::Size::None,
            }
        } else if self.is_directory() {
            self.recursive_size
                .map_or(f::Size::None, |bytes, _| f::Size::Some(bytes))
        } else if self.is_char_device() || self.is_block_device() {
            let device_id = self.metadata.rdev();

            // MacOS and Linux have different arguments and return types for the
            // functions major and minor.  On Linux the try_into().unwrap() and
            // the "as u32" cast are not needed.  We turn off the warning to
            // allow it to compile cleanly on Linux.
            #[allow(trivial_numeric_casts)]
            #[allow(clippy::unnecessary_cast, clippy::useless_conversion)]
            f::Size::DeviceIDs(f::DeviceIDs {
                // SAFETY: Calling libc function to decompose the device_id
                major: unsafe { libc::major(device_id.try_into().unwrap()) } as u32,
                minor: unsafe { libc::minor(device_id.try_into().unwrap()) } as u32,
            })
        } else if self.is_file() {
            f::Size::Some(self.metadata.len())
        } else {
            // symlink
            f::Size::None
        }
    }

    /// Returns the size of the file or indicates no size if it's a directory.
    ///
    /// For Windows platforms, the size of directories is not computed and will
    /// return `Size::None`.
    #[cfg(windows)]
    fn size(&self) -> f::Size {
        if self.is_directory() {
            f::Size::None
        } else {
            f::Size::Some(self.metadata.len())
        }
    }

    /// Returns the same value as `self.metadata.len()` or the recursive size
    /// of a directory when `total_size` is used.
    #[inline]
    fn length(&self) -> u64 {
        self.recursive_size.unwrap_bytes_or(self.metadata.len())
    }

    /// Is the file is using recursive size calculation
    #[inline]
    fn is_recursive_size(&self) -> bool {
        !self.recursive_size.is_none()
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
    fn is_empty_dir(&self) -> bool {
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
    fn is_empty_dir(&self) -> bool {
        if self.is_directory() {
            self.is_empty_directory()
        } else {
            false
        }
    }

    /// This file’s last modified timestamp, if available on this platform.
    fn modified_time(&self) -> Option<NaiveDateTime> {
        if self.is_link() && self.deref_links {
            return match self.link_target_recurse() {
                FileTarget::Ok(f) => f.modified_time(),
                _ => None,
            };
        }
        self.metadata
            .modified()
            .map(|st| DateTime::<Utc>::from(st).naive_utc())
            .ok()
    }

    /// This file’s last changed timestamp, if available on this platform.
    #[cfg(unix)]
    fn changed_time(&self) -> Option<NaiveDateTime> {
        if self.is_link() && self.deref_links {
            return match self.link_target_recurse() {
                FileTarget::Ok(f) => f.changed_time(),
                _ => None,
            };
        }
        NaiveDateTime::from_timestamp_opt(self.metadata.ctime(), self.metadata.ctime_nsec() as u32)
    }

    #[cfg(windows)]
    fn changed_time(&self) -> Option<NaiveDateTime> {
        self.modified_time()
    }

    /// This file’s last accessed timestamp, if available on this platform.
    fn accessed_time(&self) -> Option<NaiveDateTime> {
        if self.is_link() && self.deref_links {
            return match self.link_target_recurse() {
                FileTarget::Ok(f) => f.accessed_time(),
                _ => None,
            };
        }
        self.metadata
            .accessed()
            .map(|st| DateTime::<Utc>::from(st).naive_utc())
            .ok()
    }

    /// This file’s created timestamp, if available on this platform.
    fn created_time(&self) -> Option<NaiveDateTime> {
        if self.is_link() && self.deref_links {
            return match self.link_target_recurse() {
                FileTarget::Ok(f) => f.created_time(),
                _ => None,
            };
        }
        match self.metadata.created() {
            Ok(btime) => Some(DateTime::<Utc>::from(btime).naive_utc()),
            Err(_) => None,
        }
    }

    /// This file’s ‘type’.
    ///
    /// This is used a the leftmost character of the permissions column.
    /// The file type can usually be guessed from the colour of the file, but
    /// ls puts this character there.
    #[cfg(unix)]
    fn type_char(&self) -> f::Type {
        if self.is_file() {
            f::Type::File
        } else if self.is_directory() {
            f::Type::Directory
        } else if self.is_pipe() {
            f::Type::Pipe
        } else if self.is_link() {
            f::Type::Link
        } else if self.is_char_device() {
            f::Type::CharDevice
        } else if self.is_block_device() {
            f::Type::BlockDevice
        } else if self.is_socket() {
            f::Type::Socket
        } else {
            f::Type::Special
        }
    }

    #[cfg(windows)]
    fn type_char(&self) -> f::Type {
        if self.is_file() {
            f::Type::File
        } else if self.is_directory() {
            f::Type::Directory
        } else {
            f::Type::Special
        }
    }

    /// This file’s permissions, with flags for each bit.
    #[cfg(unix)]
    fn permissions(&self) -> Option<f::Permissions> {
        if self.is_link() && self.deref_links {
            // If the chain of links is broken, we instead fall through and
            // return the permissions of the original link, as would have been
            // done if we were not dereferencing.
            return match self.link_target_recurse() {
                FileTarget::Ok(f) => f.permissions(),
                _ => None,
            };
        }
        let bits = self.metadata.mode();
        Some(f::Permissions::from_mode(bits))
    }

    #[cfg(windows)]
    fn attributes(&self) -> f::Attributes {
        let bits = self.metadata.file_attributes();
        let has_bit = |bit| bits & bit == bit;

        // https://docs.microsoft.com/en-us/windows/win32/fileio/file-attribute-constants
        f::Attributes {
            directory: has_bit(0x10),
            archive: has_bit(0x20),
            readonly: has_bit(0x1),
            hidden: has_bit(0x2),
            system: has_bit(0x4),
            reparse_point: has_bit(0x400),
        }
    }

    /// This file’s security context field.
    #[cfg(unix)]
    fn security_context(&self) -> f::SecurityContext<'_> {
        let context = match self
            .extended_attributes()
            .iter()
            .find(|a| a.name == "security.selinux")
        {
            Some(attr) => match &attr.value {
                None => SecurityContextType::None,
                Some(value) => match str::from_utf8(value) {
                    Ok(v) => SecurityContextType::SELinux(v.trim_end_matches(char::from(0))),
                    Err(_) => SecurityContextType::None,
                },
            },
            None => SecurityContextType::None,
        };

        f::SecurityContext { context }
    }

    #[cfg(windows)]
    fn security_context(&self) -> f::SecurityContext<'_> {
        f::SecurityContext {
            context: SecurityContextType::None,
        }
    }

    /// User file flags.
    #[cfg(any(
        target_os = "macos",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd",
        target_os = "dragonfly"
    ))]
    fn flags(&self) -> f::Flags {
        #[cfg(target_os = "dragonfly")]
        use std::os::dragonfly::fs::MetadataExt;
        #[cfg(target_os = "freebsd")]
        use std::os::freebsd::fs::MetadataExt;
        #[cfg(target_os = "macos")]
        use std::os::macos::fs::MetadataExt;
        #[cfg(target_os = "netbsd")]
        use std::os::netbsd::fs::MetadataExt;
        #[cfg(target_os = "openbsd")]
        use std::os::openbsd::fs::MetadataExt;
        f::Flags(self.metadata.st_flags())
    }

    #[cfg(windows)]
    fn flags(&self) -> f::Flags {
        f::Flags(self.metadata.file_attributes())
    }

    #[cfg(not(any(
        target_os = "macos",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd",
        target_os = "dragonfly",
        target_os = "windows"
    )))]
    fn flags(&self) -> f::Flags {
        f::Flags(0)
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

#[cfg(test)]
mod ext_test {
    use super::File;
    use std::path::Path;

    #[test]
    fn extension() {
        assert_eq!(
            Some("dat".to_string()),
            File::extension(Path::new("fester.dat"))
        );
    }

    #[test]
    fn dotfile() {
        assert_eq!(
            Some("vimrc".to_string()),
            File::extension(Path::new(".vimrc"))
        );
    }

    #[test]
    fn no_extension() {
        assert_eq!(None, File::extension(Path::new("jarlsberg")));
    }
}

#[cfg(test)]
mod filename_test {
    use super::File;
    use std::path::Path;

    #[test]
    fn file() {
        assert_eq!("fester.dat", File::filename(Path::new("fester.dat")));
    }

    #[test]
    fn no_path() {
        assert_eq!("foo.wha", File::filename(Path::new("/var/cache/foo.wha")));
    }

    #[test]
    fn here() {
        assert_eq!(".", File::filename(Path::new(".")));
    }

    #[test]
    fn there() {
        assert_eq!("..", File::filename(Path::new("..")));
    }

    #[test]
    fn everywhere() {
        assert_eq!("..", File::filename(Path::new("./..")));
    }

    #[test]
    #[cfg(unix)]
    fn topmost() {
        assert_eq!("/", File::filename(Path::new("/")));
    }
}
