use std::io::{self, Write};

use nu_ansi_term::AnsiStrings as ANSIStrings;

use crate::fs::filter::FileFilter;
use crate::fs::Filelike;
use crate::output::cell::TextCellContents;
use crate::output::file_name::{GetStyle, Options as FileStyle};
use crate::theme::Theme;

/// The lines view literally just displays each file, line-by-line.
pub struct Render<'a, F: Filelike> {
    pub files: Vec<F>,
    pub theme: &'a Theme,
    pub file_style: &'a FileStyle,
    pub filter: &'a FileFilter,
}

impl<'a, F: Filelike + GetStyle + AsRef<F>> Render<'a, F> {
    pub fn render<W: Write>(mut self, w: &mut W) -> io::Result<()> {
        self.filter.sort_files(&mut self.files);
        for file in &self.files {
            let name_cell = self.render_file(file);
            writeln!(w, "{}", ANSIStrings(&name_cell))?;
        }

        Ok(())
    }

    fn render_file(&self, file: &F) -> TextCellContents {
        self.file_style
            .for_file(file, self.theme)
            .with_link_paths()
            .with_mount_details(false)
            .paint()
    }
}
