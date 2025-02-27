// SPDX-FileCopyrightText: 2024 Christina Sørensen
// SPDX-License-Identifier: EUPL-1.2
//
// SPDX-FileCopyrightText: 2023-2024 Christina Sørensen, eza contributors
// SPDX-FileCopyrightText: 2014 Benjamin Sago
// SPDX-License-Identifier: MIT
use std::io::{self, Write};

use term_grid::{Direction, Filling, Grid, GridOptions};

use crate::fs::filter::FileFilter;
use crate::fs::File;
use crate::output::file_name::Options as FileStyle;
use crate::theme::Theme;

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub struct Options {
    pub across: bool,
}

impl Options {
    pub fn direction(self) -> Direction {
        if self.across {
            Direction::LeftToRight
        } else {
            Direction::TopToBottom
        }
    }
}

pub struct Render<'a> {
    pub files: Vec<File<'a>>,
    pub theme: &'a Theme,
    pub file_style: &'a FileStyle,
    pub opts: &'a Options,
    pub console_width: usize,
    pub filter: &'a FileFilter,
}

impl Render<'_> {
    pub fn render<W: Write>(mut self, w: &mut W) -> io::Result<()> {
        self.filter.sort_files(&mut self.files);

        let cells = self
            .files
            .iter()
            .map(|file| {
                self.file_style
                    .for_file(file, self.theme)
                    .paint()
                    .strings()
                    .to_string()
            })
            .collect();

        let grid = Grid::new(
            cells,
            GridOptions {
                filling: Filling::Spaces(2),
                direction: self.opts.direction(),
                width: self.console_width,
            },
        );

        write!(w, "{grid}")
    }
}
