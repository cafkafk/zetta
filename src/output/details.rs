// SPDX-FileCopyrightText: 2024 Christina Sørensen
// SPDX-License-Identifier: EUPL-1.2
//
// SPDX-FileCopyrightText: 2023-2024 Christina Sørensen, eza contributors
// SPDX-FileCopyrightText: 2014 Benjamin Sago
// SPDX-License-Identifier: MIT
//! The **Details** output view displays each file as a row in a table.
//!
//! It’s used in the following situations:
//!
//! - Most commonly, when using the `--long` command-line argument to display the
//!   details of each file, which requires using a table view to hold all the data;
//! - When using the `--tree` argument, which uses the same table view to display
//!   each file on its own line, with the table providing the tree characters;
//! - When using both the `--long` and `--grid` arguments, which constructs a
//!   series of tables to fit all the data on the screen.
//!
//! You will probably recognise it from the `ls --long` command. It looks like
//! this:
//!
//! ```text
//!     .rw-r--r--  9.6k ben 29 Jun 16:16 Cargo.lock
//!     .rw-r--r--   547 ben 23 Jun 10:54 Cargo.toml
//!     .rw-r--r--  1.1k ben 23 Nov  2014 LICENCE
//!     .rw-r--r--  2.5k ben 21 May 14:38 README.md
//!     .rw-r--r--  382k ben  8 Jun 21:00 screenshot.png
//!     drwxr-xr-x     - ben 29 Jun 14:50 src
//!     drwxr-xr-x     - ben 28 Jun 19:53 target
//! ```
//!
//! The table is constructed by creating a `Table` value, which produces a `Row`
//! value for each file. These rows can contain a vector of `Cell`s, or they can
//! contain depth information for the tree view, or both. These are described
//! below.
//!
//!
//! ## Constructing Detail Views
//!
//! When using the `--long` command-line argument, the details of each file are
//! displayed next to its name.
//!
//! The table holds a vector of all the column types. For each file and column, a
//! `Cell` value containing the ANSI-coloured text and Unicode width of each cell
//! is generated, with the row and column determined by indexing into both arrays.
//!
//! The column types vector does not actually include the filename. This is
//! because the filename is always the rightmost field, and as such, it does not
//! need to have its width queried or be padded with spaces.
//!
//! To illustrate the above:
//!
//! ```text
//!     ┌─────────────────────────────────────────────────────────────────────────┐
//!     │ columns: [ Permissions,  Size,   User,  Date(Modified) ]                │
//!     ├─────────────────────────────────────────────────────────────────────────┤
//!     │   rows:  cells:                                            filename:    │
//!     │   row 1: [ ".rw-r--r--", "9.6k", "ben", "29 Jun 16:16" ]   Cargo.lock   │
//!     │   row 2: [ ".rw-r--r--",  "547", "ben", "23 Jun 10:54" ]   Cargo.toml   │
//!     │   row 3: [ "drwxr-xr-x",    "-", "ben", "29 Jun 14:50" ]   src          │
//!     │   row 4: [ "drwxr-xr-x",    "-", "ben", "28 Jun 19:53" ]   target       │
//!     └─────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! Each column in the table needs to be resized to fit its widest argument. This
//! means that we must wait until every row has been added to the table before it
//! can be displayed, in order to make sure that every column is wide enough.

use std::io::{self, Write};
use std::path::PathBuf;
use std::vec::IntoIter as VecIntoIter;

use nu_ansi_term::Style;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use log::*;

use crate::fs::dir_action::RecurseOptions;
use crate::fs::feature::git::GitCache;
use crate::fs::feature::xattr::Attribute;
use crate::fs::fields::SecurityContextType;
use crate::fs::filter::FileFilter;
use crate::fs::{Dir, File};
use crate::output::cell::TextCell;
use crate::output::color_scale::{ColorScaleInformation, ColorScaleOptions};
use crate::output::file_name::Options as FileStyle;
use crate::output::table::{Options as TableOptions, Row as TableRow, Table};
use crate::output::tree::{TreeDepth, TreeParams, TreeTrunk};
use crate::theme::Theme;

/// With the **Details** view, the output gets formatted into columns, with
/// each `Column` object showing some piece of information about the file,
/// such as its size, or its permissions.
///
/// To do this, the results have to be written to a table, instead of
/// displaying each file immediately. Then, the width of each column can be
/// calculated based on the individual results, and the fields are padded
/// during output.
///
/// Almost all the heavy lifting is done in a Table object, which handles the
/// columns for each row.
#[allow(clippy::struct_excessive_bools)]
/// This clearly isn't a state machine
#[derive(PartialEq, Eq, Debug)]
pub struct Options {
    /// Options specific to drawing a table.
    ///
    /// Directories themselves can pick which columns are *added* to this
    /// list, such as the Git column.
    pub table: Option<TableOptions>,

    /// Whether to show a header line or not.
    pub header: bool,

    /// Whether to show each file’s extended attributes.
    pub xattr: bool,

    /// Whether to show each file's security attribute.
    pub secattr: bool,

    /// Whether to show a directory's mounted filesystem details
    pub mounts: bool,

    pub color_scale: ColorScaleOptions,

    /// Whether to drill down into symbolic links that point to directories
    pub follow_links: bool,
}

pub struct Render<'a> {
    pub dir: Option<&'a Dir>,
    pub files: Vec<File<'a>>,
    pub theme: &'a Theme,
    pub file_style: &'a FileStyle,
    pub opts: &'a Options,

    /// Whether to recurse through directories with a tree view, and if so,
    /// which options to use. This field is only relevant here if the `tree`
    /// field of the `RecurseOptions` is `true`.
    pub recurse: Option<RecurseOptions>,

    /// How to sort and filter the files after getting their details.
    pub filter: &'a FileFilter,

    /// Whether we are skipping Git-ignored files.
    pub git_ignoring: bool,

    pub git: Option<&'a GitCache>,

    pub git_repos: bool,

    /// Whether we are skipping recursing into submodules.
    pub ignoring_submodule_contents: bool,
}

#[rustfmt::skip]
struct Egg<'a> {
    table_row: Option<TableRow>,
    xattrs:    &'a [Attribute],
    errors:    Vec<(io::Error, Option<PathBuf>)>,
    dir:       Option<Dir>,
    file:      &'a File<'a>,
}

impl<'a> AsRef<File<'a>> for Egg<'a> {
    fn as_ref(&self) -> &File<'a> {
        self.file
    }
}

impl<'a> Render<'a> {
    pub fn render<W: Write>(mut self, w: &mut W) -> io::Result<()> {
        let mut rows = Vec::new();

        let color_scale_info = ColorScaleInformation::from_color_scale(
            self.opts.color_scale,
            &self.files,
            self.filter.dot_filter,
            self.git,
            self.git_ignoring,
            self.ignoring_submodule_contents,
            self.recurse,
        );

        if let Some(ref table) = self.opts.table {
            match (self.git, self.dir) {
                (Some(g), Some(d)) => {
                    if !g.has_anything_for(&d.path) {
                        self.git = None;
                    }
                }
                (Some(g), None) => {
                    if !self.files.iter().any(|f| g.has_anything_for(&f.path)) {
                        self.git = None;
                    }
                }
                (None, _) => { /* Keep Git how it is */ }
            }

            let mut table = Table::new(table, self.git, self.theme, self.git_repos);

            if self.opts.header {
                let header = table.header_row();
                table.add_widths(&header);
                rows.push(self.render_header(header));
            }

            // This is weird, but I can’t find a way around it:
            // https://internals.rust-lang.org/t/should-option-mut-t-implement-copy/3715/6
            let mut table = Some(table);
            self.add_files_to_table(
                &mut table,
                &mut rows,
                &self.files,
                TreeDepth::root(),
                color_scale_info,
            );

            for row in self.iterate_with_table(table.unwrap(), rows) {
                writeln!(w, "{}", row.strings())?;
            }
        } else {
            self.add_files_to_table(
                &mut None,
                &mut rows,
                &self.files,
                TreeDepth::root(),
                color_scale_info,
            );

            for row in self.iterate(rows) {
                writeln!(w, "{}", row.strings())?;
            }
        }

        Ok(())
    }

    /// Whether to show the extended attribute hint
    pub fn show_xattr_hint(&self, file: &File<'_>) -> bool {
        // Do not show the hint '@' if the only extended attribute is the security
        // attribute and the security attribute column is active.
        let xattr_count = file.extended_attributes().len();
        let selinux_ctx_shown = self.opts.secattr
            && match file.security_context().context {
                SecurityContextType::SELinux(_) => true,
                SecurityContextType::None => false,
            };
        xattr_count > 1 || (xattr_count == 1 && !selinux_ctx_shown)
    }

    /// Adds files to the table, possibly recursively. This is easily
    /// parallelisable, and uses a pool of threads.
    fn add_files_to_table<'dir>(
        &self,
        table: &mut Option<Table<'a>>,
        rows: &mut Vec<Row>,
        src: &[File<'dir>],
        depth: TreeDepth,
        color_scale_info: Option<ColorScaleInformation>,
    ) {
        use crate::fs::feature::xattr;

        let mut file_eggs: Vec<_> = src
            .par_iter()
            .map(|file| {
                let mut errors = Vec::new();

                // There are three “levels” of extended attribute support:
                //
                // 1. If we’re compiling without that feature, then
                //    exa pretends all files have no attributes.
                // 2. If the feature is enabled and the --extended flag
                //    has been specified, then display an @ in the
                //    permissions column for files with attributes, the
                //    names of all attributes and their values, and any
                //    errors encountered when getting them.
                // 3. If the --extended flag *hasn’t* been specified, then
                //    display the @, but don’t display anything else.
                //
                // For a while, exa took a stricter approach to (3):
                // if an error occurred while checking a file’s xattrs to
                // see if it should display the @, exa would display that
                // error even though the attributes weren’t actually being
                // shown! This was confusing, as users were being shown
                // errors for something they didn’t explicitly ask for,
                // and just cluttered up the output. So now errors aren’t
                // printed unless the user passes --extended to signify
                // that they want to see them.

                let xattrs: &[Attribute] = if xattr::ENABLED && self.opts.xattr {
                    file.extended_attributes()
                } else {
                    &[]
                };

                let table_row = table
                    .as_ref()
                    .map(|t| t.row_for_file(file, self.show_xattr_hint(file), color_scale_info));

                let mut dir = None;
                let follow_links = self.opts.follow_links;
                if let Some(r) = self.recurse {
                    if (if follow_links {
                        file.points_to_directory()
                    } else {
                        file.is_directory()
                    }) && r.tree
                        && !r.is_too_deep(depth.0)
                        && !(self.ignoring_submodule_contents
                            && self
                                .git
                                .as_ref()
                                .map(|g| g.has_in_submodule(&file.path))
                                .unwrap_or(false))
                    {
                        trace!("matching on to_dir");
                        match file.to_dir() {
                            Ok(d) => {
                                dir = Some(d);
                            }
                            Err(e) => {
                                errors.push((e, None));
                            }
                        }
                    }
                };

                Egg {
                    table_row,
                    xattrs,
                    errors,
                    dir,
                    file,
                }
            })
            .collect();

        // this is safe because all entries have been initialized above
        self.filter.sort_files(&mut file_eggs);

        for (tree_params, egg) in depth.iterate_over(file_eggs.into_iter()) {
            let mut files = Vec::new();
            let errors = egg.errors;

            if let (Some(ref mut t), Some(row)) = (table.as_mut(), egg.table_row.as_ref()) {
                t.add_widths(row);
            }

            let file_name = self
                .file_style
                .for_file(egg.file, self.theme)
                .with_link_paths()
                .with_mount_details(self.opts.mounts)
                .paint()
                .promote();

            debug!("file_name {:?}", file_name);

            let row = Row {
                tree: tree_params,
                cells: egg.table_row,
                name: file_name,
            };

            rows.push(row);

            if let Some(ref dir) = egg.dir {
                for file_to_add in dir.files(
                    self.filter.dot_filter,
                    self.git,
                    self.git_ignoring,
                    egg.file.deref_links,
                    egg.file.is_recursive_size(),
                ) {
                    files.push(file_to_add);
                }

                self.filter
                    .filter_child_files(self.recurse.is_some(), &mut files);

                if !files.is_empty() {
                    for xattr in egg.xattrs {
                        rows.push(self.render_xattr(xattr, TreeParams::new(depth.deeper(), false)));
                    }

                    for (error, path) in errors {
                        rows.push(self.render_error(
                            &error,
                            TreeParams::new(depth.deeper(), false),
                            path,
                        ));
                    }

                    self.add_files_to_table(table, rows, &files, depth.deeper(), color_scale_info);
                    continue;
                }
            }

            let count = egg.xattrs.len();
            for (index, xattr) in egg.xattrs.iter().enumerate() {
                let params =
                    TreeParams::new(depth.deeper(), errors.is_empty() && index == count - 1);
                let r = self.render_xattr(xattr, params);
                rows.push(r);
            }

            let count = errors.len();
            for (index, (error, path)) in errors.into_iter().enumerate() {
                let params = TreeParams::new(depth.deeper(), index == count - 1);
                let r = self.render_error(&error, params, path);
                rows.push(r);
            }
        }
    }

    pub fn render_header(&self, header: TableRow) -> Row {
        Row {
            tree: TreeParams::new(TreeDepth::root(), false),
            cells: Some(header),
            name: TextCell::paint_str(self.theme.ui.header.unwrap_or_default(), "Name"),
        }
    }

    fn render_error(&self, error: &io::Error, tree: TreeParams, path: Option<PathBuf>) -> Row {
        use crate::output::file_name::Colours;

        let error_message = if let Some(path) = path {
            format!("<{}: {}>", path.display(), error)
        } else {
            format!("<{error}>")
        };

        // TODO: broken_symlink() doesn’t quite seem like the right name for
        // the style that’s being used here. Maybe split it in two?
        let name = TextCell::paint(self.theme.broken_symlink(), error_message);
        Row {
            cells: None,
            name,
            tree,
        }
    }

    fn render_xattr(&self, xattr: &Attribute, tree: TreeParams) -> Row {
        let name = TextCell::paint(
            self.theme.ui.perms.unwrap_or_default().attribute(),
            format!("{xattr}"),
        );
        Row {
            cells: None,
            name,
            tree,
        }
    }

    pub fn iterate_with_table(&'a self, table: Table<'a>, rows: Vec<Row>) -> TableIter<'a> {
        TableIter {
            tree_trunk: TreeTrunk::default(),
            total_width: table.widths().total(),
            table,
            inner: rows.into_iter(),
            tree_style: self.theme.ui.punctuation.unwrap_or_default(),
        }
    }

    pub fn iterate(&'a self, rows: Vec<Row>) -> Iter {
        Iter {
            tree_trunk: TreeTrunk::default(),
            inner: rows.into_iter(),
            tree_style: self.theme.ui.punctuation.unwrap_or_default(),
        }
    }
}

pub struct Row {
    /// Vector of cells to display.
    ///
    /// Most of the rows will be used to display files’ metadata, so this will
    /// almost always be `Some`, containing a vector of cells. It will only be
    /// `None` for a row displaying an attribute or error, neither of which
    /// have cells.
    pub cells: Option<TableRow>,

    /// This file’s name, in coloured output. The name is treated separately
    /// from the other cells, as it never requires padding.
    pub name: TextCell,

    /// Information used to determine which symbols to display in a tree.
    pub tree: TreeParams,
}

#[rustfmt::skip]
pub struct TableIter<'a> {
    inner: VecIntoIter<Row>,
    table: Table<'a>,

    total_width: usize,
    tree_style:  Style,
    tree_trunk:  TreeTrunk,
}

impl<'a> Iterator for TableIter<'a> {
    type Item = TextCell;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|row| {
            let mut cell = if let Some(cells) = row.cells {
                self.table.render(cells)
            } else {
                let mut cell = TextCell::default();
                cell.add_spaces(self.total_width);
                cell
            };

            for tree_part in self.tree_trunk.new_row(row.tree) {
                cell.push(self.tree_style.paint(tree_part.ascii_art()), 4);
            }

            cell.append(row.name);
            cell
        })
    }
}

pub struct Iter {
    tree_trunk: TreeTrunk,
    tree_style: Style,
    inner: VecIntoIter<Row>,
}

impl Iterator for Iter {
    type Item = TextCell;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|row| {
            let mut cell = TextCell::default();

            for tree_part in self.tree_trunk.new_row(row.tree) {
                cell.push(self.tree_style.paint(tree_part.ascii_art()), 4);
            }

            cell.append(row.name);
            cell
        })
    }
}
