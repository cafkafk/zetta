mod archive_entry;
pub use self::archive_entry::ArchiveEntry;
#[cfg(feature = "archive-inspection")]
#[cfg(unix)]
use self::archive_entry::Owner;
mod archive;
pub use self::archive::Archive;
mod archive_inspection;
pub use self::archive_inspection::ArchiveInspection;
mod archive_reader;
pub use self::archive_reader::{ArchiveFormat, ArchiveReader, Error};

#[cfg(feature = "archive-inspection")]
mod tar_reader;
#[cfg(feature = "archive-inspection")]
use self::tar_reader::TarReader;
