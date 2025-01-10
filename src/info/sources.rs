// SPDX-FileCopyrightText: 2024 Christina Sørensen
// SPDX-License-Identifier: EUPL-1.2
//
// SPDX-FileCopyrightText: 2023-2024 Christina Sørensen, eza contributors
// SPDX-FileCopyrightText: 2014 Benjamin Sago
// SPDX-License-Identifier: MIT
use std::path::PathBuf;

use crate::fs::File;

impl File<'_> {
    /// For this file, return a vector of alternate file paths that, if any of
    /// them exist, mean that *this* file should be coloured as “compiled”.
    ///
    /// The point of this is to highlight compiled files such as `foo.js` when
    /// their source file `foo.coffee` exists in the same directory.
    /// For example, `foo.js` is perfectly valid without `foo.coffee`, so we
    /// don’t want to always blindly highlight `*.js` as compiled.
    /// (See also `FileType`)
    pub fn get_source_files(&self) -> Vec<PathBuf> {
        if let Some(ext) = &self.ext {
            match &ext[..] {
                "css"   => vec![self.path.with_extension("sass"), self.path.with_extension("scss"),  // SASS, SCSS
                                self.path.with_extension("styl"), self.path.with_extension("less")],  // Stylus, Less
                "mjs"   => vec![self.path.with_extension("mts")],  // JavaScript ES Modules source
                "cjs"   => vec![self.path.with_extension("cts")],  // JavaScript Commonjs Modules source
                "js"    => vec![self.path.with_extension("coffee"), self.path.with_extension("ts")],  // CoffeeScript, TypeScript
                "aux" |                                          // TeX: auxiliary file
                "bbl" |                                          // BibTeX bibliography file
                "bcf" |                                          // biblatex control file
                "blg" |                                          // BibTeX log file
                "fdb_latexmk" |                                  // TeX latexmk file
                "fls" |                                          // TeX -recorder file
                "headfootlength" |                               // TeX package autofancyhdr file
                "lof" |                                          // TeX list of figures
                "log" |                                          // TeX log file
                "lot" |                                          // TeX list of tables
                "out" |                                          // hyperref list of bookmarks
                "toc" |                                          // TeX table of contents
                "xdv" => vec![self.path.with_extension("tex")],  // XeTeX dvi

                _ => vec![],  // No source files if none of the above
            }
        } else {
            vec![] // No source files if there’s no extension, either!
        }
    }
}
