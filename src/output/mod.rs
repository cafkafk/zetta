pub use self::cell::{DisplayWidth, TextCell, TextCellContents};
pub use self::escape::escape;

pub mod decay;
pub mod details;
pub mod file_name;
pub mod grid;
pub mod grid_details;
pub mod icons;
pub mod lines;
pub mod render;
pub mod table;
pub mod time;

mod cell;
mod escape;
mod tree;

/// The **view** contains all information about how to format output.
#[derive(Debug)]
pub struct View {
    pub mode: Mode,
    pub width: TerminalWidth,
    pub file_style: file_name::Options,
    pub deref_links: bool,
    pub total_size: bool,
}

/// The **mode** is the “type” of output.
#[derive(PartialEq, Eq, Debug)]
#[allow(clippy::large_enum_variant)]
pub enum Mode {
    Grid(grid::Options),
    Details(details::Options),
    GridDetails(grid_details::Options),
    Lines,
}

/// The width of the terminal requested by the user.
#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum TerminalWidth {
    /// The user requested this specific number of columns.
    Set(usize),

    /// Look up the terminal size at runtime.
    Automatic,
}

impl TerminalWidth {
    pub fn actual_terminal_width(self) -> Option<usize> {
        // All of stdin, stdout, and stderr could not be connected to a
        // terminal, but we’re only interested in stdout because it’s
        // where the output goes.

        #[cfg(unix)]
        let stdout_term_width = {
            use std::os::fd::AsRawFd;
            terminal_size::terminal_size_using_fd(std::io::stdout().as_raw_fd())
                .map(|(w, _h)| w.0 as _)
        };
        #[cfg(windows)]
        let stdout_term_width = {
            use std::os::windows::io::RawHandle;
            use windows_sys::Win32::System::Console::{GetStdHandle, STD_OUTPUT_HANDLE};
            terminal_size::terminal_size_using_handle(unsafe {
                GetStdHandle(STD_OUTPUT_HANDLE) as RawHandle
            })
            .map(|(w, _h)| w.0 as _)
        };

        #[rustfmt::skip]
        return match self {
            Self::Set(width)  => Some(width),
            Self::Automatic   => stdout_term_width,
        };
    }
}
