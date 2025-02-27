// SPDX-FileCopyrightText: 2024 Christina Sørensen
// SPDX-License-Identifier: EUPL-1.2
//
// SPDX-FileCopyrightText: 2023-2024 Christina Sørensen, eza contributors
// SPDX-FileCopyrightText: 2014 Benjamin Sago
// SPDX-License-Identifier: MIT
use std::fmt::Debug;
use std::path::Path;

use nu_ansi_term::{AnsiString as ANSIString, Style};
use path_clean;
use unicode_width::UnicodeWidthStr;

use crate::fs::{File, FileTarget};
use crate::output::cell::TextCellContents;
use crate::output::escape;
use crate::output::icons::{icon_for_file, iconify_style};
use crate::output::render::FiletypeColours;
use crate::theme::FileNameStyle;

/// Basically a file name factory.
#[derive(Debug, Copy, Clone)]
pub struct Options {
    /// Whether to append file class characters to file names.
    pub classify: Classify,

    /// Whether to prepend icon characters before file names.
    pub show_icons: ShowIcons,

    /// How to display file names with spaces (with or without quotes).
    pub quote_style: QuoteStyle,

    /// Whether to make file names hyperlinks.
    pub embed_hyperlinks: EmbedHyperlinks,

    /// Whether to display files with their absolute path.
    pub absolute: Absolute,

    /// Whether we are in a console or redirecting the output
    pub is_a_tty: bool,
}

impl Options {
    /// Create a new `FileName` that prints the given file’s name, painting it
    /// with the remaining arguments.
    pub fn for_file<'a, 'dir, C>(
        self,
        file: &'a File<'dir>,
        colours: &'a C,
    ) -> FileName<'a, 'dir, C> {
        FileName {
            file,
            colours,
            link_style: LinkStyle::JustFilenames,
            options: self,
            target: if file.is_link() {
                Some(file.link_target())
            } else {
                None
            },
            mount_style: MountStyle::JustDirectoryNames,
        }
    }
}

/// When displaying a file name, there needs to be some way to handle broken
/// links, depending on how long the resulting Cell can be.
#[derive(PartialEq, Debug, Copy, Clone)]
enum LinkStyle {
    /// Just display the file names, but colour them differently if they’re
    /// a broken link or can’t be followed.
    JustFilenames,

    /// Display all files in their usual style, but follow each link with an
    /// arrow pointing to their path, colouring the path differently if it’s
    /// a broken link, and doing nothing if it can’t be followed.
    FullLinkPaths,
}

/// Whether to append file class characters to the file names.
#[derive(PartialEq, Eq, Debug, Default, Copy, Clone)]
pub enum Classify {
    /// Just display the file names, without any characters.
    #[default]
    JustFilenames,

    /// Always add a character after the file name depending on what class of
    /// file it is.
    AddFileIndicators,

    // Like previous, but only when output is going to a terminal, not otherwise.
    AutomaticAddFileIndicators,
}

/// When displaying a directory name, there needs to be some way to handle
/// mount details, depending on how long the resulting Cell can be.
#[derive(PartialEq, Debug, Copy, Clone)]
enum MountStyle {
    /// Just display the directory names.
    JustDirectoryNames,

    /// Display mount points as directories and include information about
    /// the filesystem that's mounted there.
    MountInfo,
}

/// Whether and how to show icons.
#[derive(PartialEq, Debug, Copy, Clone)]
pub enum ShowIcons {
    /// Display icons next to file names, with the given number of spaces between
    /// the icon and the file name, even when output isn’t going to a terminal.
    Always(u32),

    /// Same as Always, but only when output is going to a terminal, not otherwise.
    Automatic(u32),

    /// Never display them, even when output is going to a terminal.
    Never,
}

/// Whether to embed hyperlinks.
#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum EmbedHyperlinks {
    Off,
    On,
}

/// Whether to show absolute paths
#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum Absolute {
    On,
    Follow,
    Off,
}

/// Whether or not to wrap file names with spaces in quotes.
#[derive(PartialEq, Debug, Copy, Clone)]
pub enum QuoteStyle {
    /// Don't ever quote file names.
    NoQuotes,

    /// Use single quotes for file names that contain spaces and no single quotes
    /// Use double quotes for file names that contain single quotes.
    QuoteSpaces,
}

/// A **file name** holds all the information necessary to display the name
/// of the given file. This is used in all of the views.
pub struct FileName<'a, 'dir, C> {
    /// A reference to the file that we’re getting the name of.
    file: &'a File<'dir>,

    /// The colours used to paint the file name and its surrounding text.
    colours: &'a C,

    /// The file that this file points to if it’s a link.
    target: Option<FileTarget<'dir>>, // todo: remove?

    /// How to handle displaying links.
    link_style: LinkStyle,

    pub options: Options,

    /// How to handle displaying a mounted filesystem.
    mount_style: MountStyle,
}

impl<'a, 'dir, C> FileName<'a, 'dir, C> {
    /// Sets the flag on this file name to display link targets with an
    /// arrow followed by their path.
    pub fn with_link_paths(mut self) -> Self {
        if !self.file.deref_links {
            self.link_style = LinkStyle::FullLinkPaths;
        }
        self
    }

    /// Sets the flag on this file name to display mounted filesystem
    ///details.
    pub fn with_mount_details(mut self, enable: bool) -> Self {
        self.mount_style = if enable {
            MountStyle::MountInfo
        } else {
            MountStyle::JustDirectoryNames
        };
        self
    }
}

impl<'a, 'dir, C: Colours> FileName<'a, 'dir, C> {
    /// Paints the name of the file using the colours, resulting in a vector
    /// of coloured cells that can be printed to the terminal.
    ///
    /// This method returns some `TextCellContents`, rather than a `TextCell`,
    /// because for the last cell in a table, it doesn’t need to have its
    /// width calculated.
    pub fn paint(&self) -> TextCellContents {
        let mut bits = Vec::new();
        let (icon_override, filename_style_override) = match self.colours.style_override(self.file)
        {
            Some(FileNameStyle { icon, filename }) => (icon, filename),
            None => (None, None),
        };

        let spaces_count_opt = match self.options.show_icons {
            ShowIcons::Always(spaces_count) => Some(spaces_count),
            ShowIcons::Automatic(spaces_count) if self.options.is_a_tty => Some(spaces_count),
            _ => None,
        };

        let should_add_classify_char = match self.options.classify {
            Classify::AddFileIndicators => true,
            Classify::AutomaticAddFileIndicators if self.options.is_a_tty => true,
            _ => false,
        };

        if let Some(spaces_count) = spaces_count_opt {
            let (style, icon) = match icon_override {
                Some(icon_override) => (
                    if let Some(style_override) = icon_override.style {
                        style_override
                    } else {
                        iconify_style(self.style())
                    },
                    icon_override
                        .glyph
                        .unwrap_or_else(|| icon_for_file(self.file))
                        .to_string(),
                ),
                None => (
                    iconify_style(self.style()),
                    icon_for_file(self.file).to_string(),
                ),
            };

            bits.push(style.paint(icon));
            bits.push(style.paint(" ".repeat(spaces_count as usize)));
        }

        if self.file.parent_dir.is_none() && self.options.absolute == Absolute::Off {
            if let Some(parent) = self.file.path.parent() {
                self.add_parent_bits(&mut bits, parent);
            }
        }

        if !self.file.name.is_empty() {
            // The “missing file” colour seems like it should be used here,
            // but it’s not! In a grid view, where there’s no space to display
            // link targets, the filename has to have a different style to
            // indicate this fact. But when showing targets, we can just
            // colour the path instead (see below), and leave the broken
            // link’s filename as the link colour.
            for bit in self.escaped_file_name(filename_style_override) {
                bits.push(bit);
            }
        }

        if let (LinkStyle::FullLinkPaths, Some(target)) = (self.link_style, self.target.as_ref()) {
            match target {
                FileTarget::Ok(target) => {
                    bits.push(Style::default().paint(" "));
                    bits.push(self.colours.normal_arrow().paint("->"));
                    bits.push(Style::default().paint(" "));

                    if let Some(parent) = target.path.parent() {
                        self.add_parent_bits(&mut bits, parent);
                    }

                    if !target.name.is_empty() {
                        let target_options = Options {
                            classify: Classify::JustFilenames,
                            quote_style: QuoteStyle::QuoteSpaces,
                            show_icons: ShowIcons::Never,
                            embed_hyperlinks: EmbedHyperlinks::Off,
                            is_a_tty: self.options.is_a_tty,
                            absolute: Absolute::Off,
                        };

                        let target_name = FileName {
                            file: target,
                            colours: self.colours,
                            target: None,
                            link_style: LinkStyle::FullLinkPaths,
                            options: target_options,
                            mount_style: MountStyle::JustDirectoryNames,
                        };

                        for bit in target_name.escaped_file_name(filename_style_override) {
                            bits.push(bit);
                        }

                        if should_add_classify_char {
                            if let Some(class) = self.classify_char(target) {
                                bits.push(Style::default().paint(class));
                            }
                        }
                    }
                }

                FileTarget::Broken(broken_path) => {
                    bits.push(Style::default().paint(" "));
                    bits.push(self.colours.broken_symlink().paint("->"));
                    bits.push(Style::default().paint(" "));

                    escape(
                        broken_path.display().to_string(),
                        &mut bits,
                        self.colours.broken_filename(),
                        self.colours.broken_control_char(),
                        self.options.quote_style,
                    );
                }

                FileTarget::Err(_) => {
                    // Do nothing — the error gets displayed on the next line
                }
            }
        } else if should_add_classify_char {
            if let Some(class) = self.classify_char(self.file) {
                bits.push(Style::default().paint(class));
            }
        }

        if self.mount_style == MountStyle::MountInfo {
            if let Some(mount_details) = self.file.mount_point_info() {
                // This is a filesystem mounted on the directory, output its details
                bits.push(Style::default().paint(" ["));
                bits.push(Style::default().paint(mount_details.source.clone()));
                bits.push(Style::default().paint(" ("));
                bits.push(Style::default().paint(mount_details.fstype.clone()));
                bits.push(Style::default().paint(")]"));
            }
        }

        bits.into()
    }

    /// Adds the bits of the parent path to the given bits vector.
    /// The path gets its characters escaped based on the colours.
    fn add_parent_bits(&self, bits: &mut Vec<ANSIString<'_>>, parent: &Path) {
        let coconut = parent.components().count();

        if coconut == 1 && parent.has_root() {
            bits.push(
                self.colours
                    .symlink_path()
                    .paint(std::path::MAIN_SEPARATOR.to_string()),
            );
        } else if coconut >= 1 {
            escape(
                parent.to_string_lossy().to_string(),
                bits,
                self.colours.symlink_path(),
                self.colours.control_char(),
                self.options.quote_style,
            );
            bits.push(
                self.colours
                    .symlink_path()
                    .paint(std::path::MAIN_SEPARATOR.to_string()),
            );
        }
    }

    /// The character to be displayed after a file when classifying is on, if
    /// the file’s type has one associated with it.
    #[cfg(unix)]
    pub(crate) fn classify_char(&self, file: &File<'_>) -> Option<&'static str> {
        if file.is_executable_file() {
            Some("*")
        } else if file.is_directory() {
            Some("/")
        } else if file.is_pipe() {
            Some("|")
        } else if file.is_link() {
            Some("@")
        } else if file.is_socket() {
            Some("=")
        } else {
            None
        }
    }

    #[cfg(windows)]
    pub(crate) fn classify_char(&self, file: &File<'_>) -> Option<&'static str> {
        if file.is_directory() {
            Some("/")
        } else if file.is_link() {
            Some("@")
        } else {
            None
        }
    }

    /// Returns at least one ANSI-highlighted string representing this file’s
    /// name using the given set of colours.
    ///
    /// If --hyperlink flag is provided, it will escape the filename accordingly.
    ///
    /// Ordinarily, this will be just one string: the file’s complete name,
    /// coloured according to its file type. If the name contains control
    /// characters such as newlines or escapes, though, we can’t just print them
    /// to the screen directly, because then there’ll be newlines in weird places.
    ///
    /// So in that situation, those characters will be escaped and highlighted in
    /// a different colour.
    fn escaped_file_name<'unused>(
        &self,
        style_override: Option<Style>,
    ) -> Vec<ANSIString<'unused>> {
        use percent_encoding::{utf8_percent_encode, CONTROLS};

        const HYPERLINK_START: &str = "\x1B]8;;";
        const HYPERLINK_END: &str = "\x1B\x5C";

        let file_style = style_override.unwrap_or(self.style());
        let mut bits = Vec::new();

        let mut display_hyperlink = false;
        if self.options.embed_hyperlinks == EmbedHyperlinks::On {
            if let Some(abs_path) = self
                .file
                .absolute_path()
                .and_then(|p| p.as_os_str().to_str())
            {
                let abs_path = utf8_percent_encode(abs_path, CONTROLS).to_string();

                // On Windows, `std::fs::canonicalize` adds the Win32 File prefix, which we need to remove
                #[cfg(target_os = "windows")]
                let abs_path = abs_path.strip_prefix("\\\\?\\").unwrap_or(&abs_path);

                bits.push(ANSIString::from(format!(
                    "{HYPERLINK_START}file://{abs_path}{HYPERLINK_END}"
                )));

                display_hyperlink = true;
            }
        }

        escape(
            self.display_name(),
            &mut bits,
            file_style,
            self.colours.control_char(),
            self.options.quote_style,
        );

        if display_hyperlink {
            bits.push(ANSIString::from(format!(
                "{HYPERLINK_START}{HYPERLINK_END}"
            )));
        }

        bits
    }

    /// Returns the string that should be displayed as the file's name.
    fn display_name(&self) -> String {
        match self.options.absolute {
            Absolute::On => std::env::current_dir().ok().and_then(|p| {
                path_clean::clean(p.join(&self.file.path))
                    .to_str()
                    .map(std::borrow::ToOwned::to_owned)
            }),
            Absolute::Follow => self
                .file
                .absolute_path()
                .and_then(|p| p.to_str())
                .map(std::borrow::ToOwned::to_owned),
            Absolute::Off => None,
        }
        .unwrap_or(self.file.name.clone())
    }

    /// Figures out which colour to paint the filename part of the output,
    /// depending on which “type” of file it appears to be — either from the
    /// class on the filesystem or from its name. (Or the broken link colour,
    /// if there’s nowhere else for that fact to be shown.)
    pub fn style(&self) -> Style {
        if let LinkStyle::JustFilenames = self.link_style {
            if let Some(ref target) = self.target {
                if target.is_broken() {
                    return self.colours.broken_symlink();
                }
            }
        }

        #[rustfmt::skip]
        return match self.file {
            f if f.is_mount_point()      => self.colours.mount_point(),
            f if f.is_btrfs_subvolume()  => self.colours.btrfs_subvol(),
            f if f.is_directory()        => self.colours.directory(),
            #[cfg(unix)]
            f if f.is_executable_file()  => self.colours.executable_file(),
            f if f.is_link()             => self.colours.symlink(),
            #[cfg(unix)]
            f if f.is_pipe()             => self.colours.pipe(),
            #[cfg(unix)]
            f if f.is_block_device()     => self.colours.block_device(),
            #[cfg(unix)]
            f if f.is_char_device()      => self.colours.char_device(),
            #[cfg(unix)]
            f if f.is_socket()           => self.colours.socket(),
            f if ! f.is_file()           => self.colours.special(),
            _                            => self.colours.colour_file(self.file),
        };
    }

    /// For grid's use, to cover the case of hyperlink escape sequences
    pub fn bare_utf8_width(&self) -> usize {
        UnicodeWidthStr::width(self.file.name.as_str())
    }
}

/// The set of colours that are needed to paint a file name.
pub trait Colours: FiletypeColours {
    /// The style to paint the path of a symlink’s target, up to but not
    /// including the file’s name.
    fn symlink_path(&self) -> Style;

    /// The style to paint the arrow between a link and its target.
    fn normal_arrow(&self) -> Style;

    /// The style to paint the filenames of broken links in views that don’t
    /// show link targets, and the style to paint the *arrow* between the link
    /// and its target in views that *do* show link targets.
    fn broken_symlink(&self) -> Style;

    /// The style to paint the entire filename of a broken link.
    fn broken_filename(&self) -> Style;

    /// The style to paint a non-displayable control character in a filename.
    fn control_char(&self) -> Style;

    /// The style to paint a non-displayable control character in a filename,
    /// when the filename is being displayed as a broken link target.
    fn broken_control_char(&self) -> Style;

    /// The style to paint a file that has its executable bit set.
    fn executable_file(&self) -> Style;

    /// The style to paint a directory that has a filesystem mounted on it.
    fn mount_point(&self) -> Style;

    /// The style to paint a directory representing a Btrfs subvolume.
    fn btrfs_subvol(&self) -> Style;

    fn colour_file(&self, file: &File<'_>) -> Style;

    fn style_override(&self, file: &File<'_>) -> Option<FileNameStyle>;
}
