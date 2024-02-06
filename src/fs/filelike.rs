use chrono::NaiveDateTime;
use std::fs::Metadata;
use std::io;
use std::path::PathBuf;

use crate::fs::dir::Dir;
use crate::fs::feature::xattr::Attribute;
use crate::fs::fields as f;
use crate::fs::file::FileTarget;
use crate::fs::mounts::MountedFs;

pub trait Filelike {
    /// Path
    fn path(&self) -> &PathBuf;

    /// File name
    fn name(&self) -> &String;

    /// File extension
    fn extension(&self) -> Option<String>;

    /// Whether to dereference symbolic links when querying for information.
    ///
    /// For instance, when querying the size of a symbolic link, if
    /// dereferencing is enabled, the size of the target will be displayed
    /// instead.
    fn deref_links(&self) -> bool;

    /// Get the extended attributes of a file
    fn extended_attributes(&self) -> &[Attribute];

    /// Metadata for file in filesystem
    fn metadata(&self) -> Option<&Metadata>;

    /// A reference to the directory that contains this file, if any.
    ///
    /// Filenames that get passed in on the command-line directly will have no
    /// parent directory reference — although they technically have one on the
    /// filesystem, we’ll never need to look at it, so it’ll be `None`.
    /// However, *directories* that get passed in will produce files that
    /// contain a reference to it, which is used in certain operations (such
    /// as looking up compiled files).
    fn parent_directory(&self) -> Option<&Dir>;

    /// If this file is a directory on the filesystem, then clone its
    /// `PathBuf` for use in one of our own `Dir` values, and read a list of
    /// its contents.
    ///
    /// Returns an IO error upon failure, but this shouldn’t be used to check
    /// if it is a directory or not! For that, just use `is_directory()`.
    ///
    /// If this file is not representable in the filesystem, `None` will be
    /// returned.
    fn to_dir(&self) -> Option<io::Result<Dir>>;

    /// Whether this file is a directory on the filesystem.
    fn is_directory(&self) -> bool;

    /// Whether this file is a directory, or a symlink pointing to a directory.
    fn points_to_directory(&self) -> bool;

    /// Whether this file is a regular file on the filesystem — that is, not a
    /// directory, a link, or anything else treated specially.
    fn is_file(&self) -> bool;

    /// Whether this file is both a regular file *and* executable for the
    /// current user. An executable file has a different purpose from an
    /// executable directory, so they should be highlighted differently.
    #[cfg(unix)]
    fn is_executable_file(&self) -> bool;

    /// Whether this file is a symlink on the filesystem.
    fn is_link(&self) -> bool;

    /// Whether this file is a named pipe on the filesystem.
    #[cfg(unix)]
    fn is_pipe(&self) -> bool;

    /// Whether this file is a char device on the filesystem.
    #[cfg(unix)]
    fn is_char_device(&self) -> bool;

    /// Whether this file is a block device on the filesystem.
    #[cfg(unix)]
    fn is_block_device(&self) -> bool;

    /// Whether this file is a socket on the filesystem.
    #[cfg(unix)]
    fn is_socket(&self) -> bool;

    /// Determine the full path resolving all symbolic links on demand.
    fn absolute_path(&self) -> Option<&PathBuf>;

    /// Whether this file is a mount point
    fn is_mount_point(&self) -> bool;

    /// The filesystem device and type for a mount point
    fn mount_point_info(&self) -> Option<&MountedFs>;

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
    fn link_target<'a>(&self) -> FileTarget<'a>;

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
    fn link_target_recurse<'a>(&self) -> FileTarget<'a>;

    /// This file’s number of hard links.
    ///
    /// It also reports whether this is both a regular file, and a file with
    /// multiple links. This is important, because a file with multiple links
    /// is uncommon, while you come across directories and other types
    /// with multiple links much more often. Thus, it should get highlighted
    /// more attentively.
    #[cfg(unix)]
    fn links(&self) -> f::Links;

    /// This file’s inode.
    #[cfg(unix)]
    fn inode(&self) -> f::Inode;

    /// This actual size the file takes up on disk, in bytes.
    #[cfg(unix)]
    fn blocksize(&self) -> f::Blocksize;

    /// The ID of the user that own this file. If dereferencing links, the links
    /// may be broken, in which case `None` will be returned.
    #[cfg(unix)]
    fn user(&self) -> Option<f::User>;

    /// The ID of the group that owns this file.
    #[cfg(unix)]
    fn group(&self) -> Option<f::Group>;

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
    ///
    /// For Windows platforms, the size of directories is not computed and will
    /// return `Size::None`.
    fn size(&self) -> f::Size;

    /// Returns the same value as `self.metadata.len()` or the recursive size
    /// of a directory when `total_size` is used.
    fn length(&self) -> u64;

    /// Is the file is using recursive size calculation
    fn is_recursive_size(&self) -> bool;

    /// Determines if the directory is empty or not.
    ///
    /// For Unix platforms, this function first checks the link count to quickly
    /// determine non-empty directories. On most UNIX filesystems the link count
    /// is two plus the number of subdirectories. If the link count is less than
    /// or equal to 2, it then checks the directory contents to determine if
    /// it's truly empty. The naive approach used here checks the contents
    /// directly, as certain filesystems make it difficult to infer emptiness
    /// based on directory size alone.
    ///
    /// For Windows platforms, this function checks the directory contents directly
    /// to determine if it's empty. Since certain filesystems on Windows make it
    /// challenging to infer emptiness based on directory size, this approach is used.
    fn is_empty_dir(&self) -> bool;

    /// This file’s last modified timestamp, if available on this platform.
    fn modified_time(&self) -> Option<NaiveDateTime>;

    /// This file’s last changed timestamp, if available on this platform.
    fn changed_time(&self) -> Option<NaiveDateTime>;

    /// This file’s last accessed timestamp, if available on this platform.
    fn accessed_time(&self) -> Option<NaiveDateTime>;

    /// This file’s created timestamp, if available on this platform.
    fn created_time(&self) -> Option<NaiveDateTime>;

    /// This file’s ‘type’.
    ///
    /// This is used a the leftmost character of the permissions column.
    /// The file type can usually be guessed from the colour of the file, but
    /// ls puts this character there.
    fn type_char(&self) -> f::Type;

    /// This file’s permissions, with flags for each bit.
    #[cfg(unix)]
    fn permissions(&self) -> Option<f::Permissions>;

    #[cfg(windows)]
    fn attributes(&self) -> f::Attributes;

    /// This file’s security context field.
    fn security_context(&self) -> f::SecurityContext<'_>;

    /// User file flags.
    fn flags(&self) -> f::Flags;
}
