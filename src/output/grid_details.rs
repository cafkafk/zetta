//! The grid-details view lists several details views side-by-side.

use std::io::{self, Write};

use ansi_width;
use grid::{Direction, Filling, Grid, GridOptions};
use term_grid as grid;

use crate::fs::feature::git::GitCache;
use crate::fs::filter::FileFilter;
use crate::fs::{File, Filelike};
use crate::output::cell::TextCell;
use crate::output::color_scale::ColorScaleInformation;
use crate::output::details::{Options as DetailsOptions, Render as DetailsRender};
use crate::output::file_name::{GetStyle, Options as FileStyle};
use crate::output::table::{Options as TableOptions, Table};
use crate::theme::Theme;

#[derive(PartialEq, Eq, Debug)]
pub struct Options {
    pub details: DetailsOptions,
    pub row_threshold: RowThreshold,
}

impl Options {
    pub fn to_details_options(&self) -> &DetailsOptions {
        &self.details
    }
}

/// The grid-details view can be configured to revert to just a details view
/// (with one column) if it wouldn’t produce enough rows of output.
///
/// Doing this makes the resulting output look a bit better: when listing a
/// small directory of four files in four columns, the files just look spaced
/// out and it’s harder to see what’s going on. So it can be enabled just for
/// larger directory listings.
#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum RowThreshold {
    /// Only use grid-details view if it would result in at least this many
    /// rows of output.
    MinimumRows(usize),

    /// Use the grid-details view no matter what.
    AlwaysGrid,
}

pub struct Render<'a, F: Filelike> {
    /// The directory that’s being rendered here.
    /// We need this to know which columns to put in the output.
    pub dir_path: Option<&'a std::path::PathBuf>,

    /// The files that have been read from the directory. They should all
    /// hold a reference to it.
    pub files: Vec<F>,

    /// How to colour various pieces of text.
    pub theme: &'a Theme,

    /// How to format filenames.
    pub file_style: &'a FileStyle,

    /// The details part of the grid-details view.
    pub details: &'a DetailsOptions,

    /// How to filter files after listing a directory. The files in this
    /// render will already have been filtered and sorted, but any directories
    /// that we recurse into will have to have this applied.
    pub filter: &'a FileFilter,

    /// The minimum number of rows that there need to be before grid-details
    /// mode is activated.
    pub row_threshold: RowThreshold,

    /// Whether we are skipping Git-ignored files.
    pub git_ignoring: bool,

    pub git: Option<&'a GitCache>,

    pub console_width: usize,

    pub git_repos: bool,
}

impl<'a, F: Filelike + GetStyle + std::marker::Sync> Render<'a, F> {
    /// Create a temporary Details render that gets used for the columns of
    /// the grid-details render that’s being generated.
    ///
    /// This includes an empty files vector because the files get added to
    /// the table in *this* file, not in details: we only want to insert every
    /// *n* files into each column’s table, not all of them.
    fn details_for_column(&self) -> DetailsRender<'a, File<'a>> {
        #[rustfmt::skip]
        return DetailsRender {
            dir_path:      self.dir_path,
            files:         Vec::new(),
            theme:         self.theme,
            file_style:    self.file_style,
            opts:          self.details,
            recurse:       None,
            filter:        self.filter,
            git_ignoring:  self.git_ignoring,
            git:           self.git,
            git_repos:     self.git_repos,
        };
    }

    // This doesn’t take an IgnoreCache even though the details one does
    // because grid-details has no tree view.

    pub fn render<W: Write>(mut self, w: &mut W) -> io::Result<()> {
        let options = self
            .details
            .table
            .as_ref()
            .expect("Details table options not given!");

        let drender = self.details_for_column();

        let color_scale_info = ColorScaleInformation::from_color_scale(
            self.details.color_scale,
            &self.files,
            self.filter.dot_filter,
            self.git,
            self.git_ignoring,
            None,
        );

        let mut table = self.make_table(options);

        // It is important to collect all these rows _before_ turning them into
        // cells, because the width calculations need to consider all rows
        // before each row is turned into a string.
        let rows: Vec<_> = self
            .files
            .iter()
            .map(|file| {
                let row = table.row_for_file(file, drender.show_xattr_hint(file), color_scale_info);
                table.add_widths(&row);
                row
            })
            .collect();

        let cells = rows
            .into_iter()
            .zip(self.files)
            .map(|(row, file)| {
                let filename = self
                    .file_style
                    .for_file(&file, self.theme)
                    .paint()
                    .strings()
                    .to_string();
                let details = table.render(row).strings().to_string();

                // This bit fixes a strange corner case. If there is a header,
                // then "Name" will be added to the header row. That means that
                // the filename column, should be at least 4 characters wide.
                // Therefore we pad the filenames with some spaces. We have to
                // use ansi_width here, because the filename might contain some
                // styling.
                let padding = " ".repeat(if self.details.header {
                    4usize.saturating_sub(ansi_width::ansi_width(&filename))
                } else {
                    0
                });

                format!("{details} {filename}{padding}")
            })
            .collect();

        let grid = Grid::new(
            cells,
            GridOptions {
                filling: Filling::Spaces(4),
                direction: Direction::TopToBottom,
                width: self.console_width,
            },
        );

        if self.details.header {
            let row = table.header_row();
            let name = TextCell::paint_str(self.theme.ui.header, "Name")
                .strings()
                .to_string();
            let s = table.render(row).strings().to_string();
            let combined_header = format!("{s} {name}");
            let header_width = ansi_width::ansi_width(&combined_header);
            for column_width in grid.column_widths() {
                let padding = " ".repeat((column_width + 4).saturating_sub(header_width));
                write!(w, "{combined_header}{padding}")?;
            }
            writeln!(w)?;
        }

        write!(w, "{grid}")?;

        Ok(())
    }

    fn make_table(&mut self, options: &'a TableOptions) -> Table<'a> {
        match (self.git, self.dir_path) {
            (Some(g), Some(d)) => {
                if !g.has_anything_for(d) {
                    self.git = None;
                }
            }
            (Some(g), None) => {
                if !self.files.iter().any(|f| g.has_anything_for(f.path())) {
                    self.git = None;
                }
            }
            (None, _) => { /* Keep Git how it is */ }
        }

        let mut table = Table::new(options, self.git, self.theme, self.git_repos);

        // The header row will be printed separately, but it should be
        // considered for the width calculations.
        if self.details.header {
            let row = table.header_row();
            table.add_widths(&row);
        }

        table
    }
}
