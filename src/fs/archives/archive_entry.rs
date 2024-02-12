use std::io;
use std::path::PathBuf;

use chrono::NaiveDateTime;

use crate::fs::feature::xattr::Attribute;
use crate::fs::fields as f;
use crate::fs::file::FileTarget;
use crate::fs::mounts::MountedFs;
use crate::fs::{Archive, Dir, File, Filelike};

#[derive(Clone)]
pub struct Owner {
    pub id: u64,
    pub name: Option<String>,
}

#[derive(Clone)]
pub struct ArchiveEntry {
    pub(super) name: String,
    pub(super) path: PathBuf,
    pub(super) size: u64,
    #[cfg(unix)]
    pub(super) permissions: Option<f::Permissions>,
    #[cfg(unix)]
    pub(super) user: Option<Owner>,
    #[cfg(unix)]
    pub(super) group: Option<Owner>,
    pub(super) is_directory: bool,
    pub(super) is_link: bool,
    pub(super) link_target: Option<PathBuf>,
    pub(super) mtime: Option<u64>,
    pub(super) atime: Option<u64>,
    pub(super) ctime: Option<u64>,
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

    fn to_archive(&self) -> Option<Archive> {
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
