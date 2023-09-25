#![warn(deprecated_in_future)]
#![warn(future_incompatible)]
#![warn(nonstandard_style)]
#![warn(rust_2018_compatibility)]
#![warn(rust_2018_idioms)]
#![warn(trivial_casts, trivial_numeric_casts)]
#![warn(unused)]
#![warn(clippy::all, clippy::pedantic)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::enum_glob_use)]
#![allow(clippy::map_unwrap_or)]
#![allow(clippy::match_same_arms)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::non_ascii_literal)]
#![allow(clippy::option_if_let_else)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::unused_self)]
#![allow(clippy::upper_case_acronyms)]
#![allow(clippy::wildcard_imports)]

use clap::Parser;
use std::env;
use std::ffi::{OsStr, OsString};
use std::io::{self, ErrorKind, Write};
use std::path::{Component, PathBuf};
use std::process::exit;

use ansiterm::{ANSIStrings, Style};

use log::*;

use crate::fs::feature::git::GitCache;
use crate::fs::filter::GitIgnore;
use crate::fs::{Dir, File};
use crate::options::parser::Opts;
use crate::options::{vars, Options, Vars};
use crate::output::{details, escape, grid, grid_details, lines, Mode, View};
use crate::theme::Theme;

mod fs;
mod info;
mod logger;
mod options;
mod output;
mod theme;

fn main() {
    #[cfg(unix)]
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }

    logger::configure(env::var_os(vars::EZA_DEBUG).or_else(|| env::var_os(vars::EXA_DEBUG)));

    #[cfg(windows)]
    if let Err(e) = ansiterm::enable_ansi_support() {
        warn!("Failed to enable ANSI support: {}", e);
    }

    let cli = Opts::parse();
    let mut input_paths: Vec<&OsStr> = cli.paths.iter().map(OsString::as_os_str).collect();
    if input_paths.is_empty() {
        input_paths.push(OsStr::new("."));
    }
    let options = match Options::deduce(&cli, &LiveVars) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("{e}");
            exit(exits::OPTIONS_ERROR);
        }
    };

    let git = git_options(&options, &input_paths);
    let writer = io::stdout();

    let console_width = options.view.width.actual_terminal_width();
    let theme = options
        .theme
        .to_theme(terminal_size::terminal_size().is_some());
    let exa = Exa {
        options,
        writer,
        input_paths,
        theme,
        console_width,
        git,
    };

    match exa.run() {
        Ok(exit_status) => {
            exit(exit_status);
        }

        Err(e) if e.kind() == ErrorKind::BrokenPipe => {
            warn!("Broken pipe error: {e}");
            exit(exits::SUCCESS);
        }

        Err(e) => {
            eprintln!("{e}");
            exit(exits::RUNTIME_ERROR);
        }
    }
}

/// The main program wrapper.
pub struct Exa<'args> {
    /// List of command-line options, having been successfully parsed.
    pub options: Options,

    /// The output handle that we write to.
    pub writer: io::Stdout,

    /// List of the free command-line arguments that should correspond to file
    /// names (anything that isn’t an option).
    pub input_paths: Vec<&'args OsStr>,

    /// The theme that has been configured from the command-line options and
    /// environment variables. If colours are disabled, this is a theme with
    /// every style set to the default.
    pub theme: Theme,

    /// The detected width of the console. This is used to determine which
    /// view to use.
    pub console_width: Option<usize>,

    /// A global Git cache, if the option was passed in.
    /// This has to last the lifetime of the program, because the user might
    /// want to list several directories in the same repository.
    pub git: Option<GitCache>,
}

/// The “real” environment variables type.
/// Instead of just calling `var_os` from within the options module,
/// the method of looking up environment variables has to be passed in.
struct LiveVars;
impl Vars for LiveVars {
    fn get(&self, name: &'static str) -> Option<OsString> {
        env::var_os(name)
    }
}

/// Create a Git cache populated with the arguments that are going to be
/// listed before they’re actually listed, if the options demand it.
fn git_options(options: &Options, args: &[&OsStr]) -> Option<GitCache> {
    if options.should_scan_for_git() {
        Some(args.iter().map(PathBuf::from).collect())
    } else {
        None
    }
}

impl<'args> Exa<'args> {
    /// # Errors
    ///
    /// Will return `Err` if printing to stderr fails.
    pub fn run(mut self) -> io::Result<i32> {
        debug!("Running with options: {:#?}", self.options);

        let mut files = Vec::new();
        let mut dirs = Vec::new();
        let mut exit_status = 0;

        for file_path in &self.input_paths {
            match File::from_args(
                PathBuf::from(file_path),
                None,
                None,
                self.options.view.deref_links,
            ) {
                Err(e) => {
                    exit_status = 2;
                    writeln!(io::stderr(), "{file_path:?}: {e}")?;
                }

                Ok(f) => {
                    if f.points_to_directory() && !self.options.dir_action.treat_dirs_as_files() {
                        trace!("matching on to_dir");
                        match f.to_dir() {
                            Ok(d) => dirs.push(d),
                            Err(e) if e.kind() == ErrorKind::PermissionDenied => {
                                warn!("Permission Denied: {e}");
                                exit(exits::PERMISSION_DENIED);
                            }
                            Err(e) => writeln!(io::stderr(), "{file_path:?}: {e}")?,
                        }
                    } else {
                        files.push(f);
                    }
                }
            }
        }

        // We want to print a directory’s name before we list it, *except* in
        // the case where it’s the only directory, *except* if there are any
        // files to print as well. (It’s a double negative)

        let no_files = files.is_empty();
        let is_only_dir = dirs.len() == 1 && no_files;

        self.options.filter.filter_argument_files(&mut files);
        self.print_files(None, files)?;

        self.print_dirs(dirs, no_files, is_only_dir, exit_status)
    }

    fn print_dirs(
        &mut self,
        dir_files: Vec<Dir>,
        mut first: bool,
        is_only_dir: bool,
        exit_status: i32,
    ) -> io::Result<i32> {
        for dir in dir_files {
            // Put a gap between directories, or between the list of files and
            // the first directory.
            if first {
                first = false;
            } else {
                writeln!(&mut self.writer)?;
            }

            if !is_only_dir {
                let mut bits = Vec::new();
                escape(
                    dir.path.display().to_string(),
                    &mut bits,
                    Style::default(),
                    Style::default(),
                );
                writeln!(&mut self.writer, "{}:", ANSIStrings(&bits))?;
            }

            let mut children = Vec::new();
            let git_ignore = self.options.filter.git_ignore == GitIgnore::CheckAndIgnore;
            for file in dir.files(
                self.options.filter.dot_filter,
                self.git.as_ref(),
                git_ignore,
                self.options.view.deref_links,
            ) {
                match file {
                    Ok(file) => children.push(file),
                    Err((path, e)) => writeln!(io::stderr(), "[{}: {}]", path.display(), e)?,
                }
            }

            self.options.filter.filter_child_files(&mut children);
            self.options.filter.sort_files(&mut children);

            if let Some(recurse_opts) = self.options.dir_action.recurse_options() {
                let depth = dir
                    .path
                    .components()
                    .filter(|&c| c != Component::CurDir)
                    .count()
                    + 1;
                if !recurse_opts.tree && !recurse_opts.is_too_deep(depth) {
                    let mut child_dirs = Vec::new();
                    for child_dir in children
                        .iter()
                        .filter(|f| f.is_directory() && !f.is_all_all)
                    {
                        match child_dir.to_dir() {
                            Ok(d) => child_dirs.push(d),
                            Err(e) => {
                                writeln!(io::stderr(), "{}: {}", child_dir.path.display(), e)?;
                            }
                        }
                    }

                    self.print_files(Some(&dir), children)?;
                    match self.print_dirs(child_dirs, false, false, exit_status) {
                        Ok(_) => (),
                        Err(e) => return Err(e),
                    }
                    continue;
                }
            }

            self.print_files(Some(&dir), children)?;
        }

        Ok(exit_status)
    }

    /// Prints the list of files using whichever view is selected.
    fn print_files(&mut self, dir: Option<&Dir>, files: Vec<File<'_>>) -> io::Result<()> {
        if files.is_empty() {
            return Ok(());
        }

        let theme = &self.theme;
        let View {
            ref mode,
            ref file_style,
            ..
        } = self.options.view;

        match (mode, self.console_width) {
            (Mode::Grid(ref opts), Some(console_width)) => {
                let filter = &self.options.filter;
                let r = grid::Render {
                    files,
                    theme,
                    file_style,
                    opts,
                    console_width,
                    filter,
                };
                r.render(&mut self.writer)
            }

            (Mode::Grid(_), None) | (Mode::Lines, _) => {
                let filter = &self.options.filter;
                let r = lines::Render {
                    files,
                    theme,
                    file_style,
                    filter,
                };
                r.render(&mut self.writer)
            }

            (Mode::Details(ref opts), _) => {
                let filter = &self.options.filter;
                let recurse = self.options.dir_action.recurse_options();

                let git_ignoring = self.options.filter.git_ignore == GitIgnore::CheckAndIgnore;
                let git = self.git.as_ref();
                let r = details::Render {
                    dir,
                    files,
                    theme,
                    file_style,
                    opts,
                    recurse,
                    filter,
                    git_ignoring,
                    git,
                };
                r.render(&mut self.writer)
            }

            (Mode::GridDetails(ref opts), Some(console_width)) => {
                let grid = &opts.grid;
                let details = &opts.details;
                let row_threshold = opts.row_threshold;

                let filter = &self.options.filter;
                let git_ignoring = self.options.filter.git_ignore == GitIgnore::CheckAndIgnore;
                let git = self.git.as_ref();

                let r = grid_details::Render {
                    dir,
                    files,
                    theme,
                    file_style,
                    grid,
                    details,
                    filter,
                    row_threshold,
                    git_ignoring,
                    git,
                    console_width,
                };
                r.render(&mut self.writer)
            }

            (Mode::GridDetails(ref opts), None) => {
                let opts = &opts.to_details_options();
                let filter = &self.options.filter;
                let recurse = self.options.dir_action.recurse_options();
                let git_ignoring = self.options.filter.git_ignore == GitIgnore::CheckAndIgnore;

                let git = self.git.as_ref();
                let r = details::Render {
                    dir,
                    files,
                    theme,
                    file_style,
                    opts,
                    recurse,
                    filter,
                    git_ignoring,
                    git,
                };
                r.render(&mut self.writer)
            }
        }
    }
}

mod exits {

    /// Exit code for when exa runs OK.
    pub const SUCCESS: i32 = 0;

    /// Exit code for when there was at least one I/O error during execution.
    pub const RUNTIME_ERROR: i32 = 1;

    /// Exit code for when the command-line options are invalid.
    pub const OPTIONS_ERROR: i32 = 3;

    /// Exit code for missing file permissions
    pub const PERMISSION_DENIED: i32 = 13;
}
