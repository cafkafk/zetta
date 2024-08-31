use std::ffi::OsString;
use std::io;

use crate::fs::feature::xattr;
use crate::options::parser::MatchedFlags;
use crate::options::{flags, vars, NumberSource, OptionsError, Vars};
use crate::output::color_scale::{ColorScaleMode, ColorScaleOptions};
use crate::output::file_name::Options as FileStyle;
use crate::output::grid_details::{self, RowThreshold};
use crate::output::table::{
    Columns, FlagsFormat, GroupFormat, Options as TableOptions, SizeFormat, TimeTypes, UserFormat,
};
use crate::output::time::TimeFormat;
use crate::output::{details, grid, Mode, TerminalWidth, View};
use std::io::IsTerminal;

impl View {
    pub fn deduce<V: Vars>(matches: &MatchedFlags<'_>, vars: &V) -> Result<Self, OptionsError> {
        let mode = Mode::deduce(matches, vars)?;
        let deref_links = matches.has(&flags::DEREF_LINKS)?;
        let total_size = matches.has(&flags::TOTAL_SIZE)?;
        let width = TerminalWidth::deduce(matches, vars)?;
        let file_style = FileStyle::deduce(matches, vars, width.actual_terminal_width().is_some())?;
        Ok(Self {
            mode,
            width,
            file_style,
            deref_links,
            total_size,
        })
    }
}

impl Mode {
    /// Determine which viewing mode to use based on the user’s options.
    ///
    /// As with the other options, arguments are scanned right-to-left and the
    /// first flag found is matched, so `exa --oneline --long` will pick a
    /// details view, and `exa --long --oneline` will pick the lines view.
    ///
    /// This is complicated a little by the fact that `--grid` and `--tree`
    /// can also combine with `--long`, so care has to be taken to use the
    pub fn deduce<V: Vars>(matches: &MatchedFlags<'_>, vars: &V) -> Result<Self, OptionsError> {
        let flag = matches.has_where_any(|f| {
            f.matches(&flags::LONG)
                || f.matches(&flags::ONE_LINE)
                || f.matches(&flags::GRID)
                || f.matches(&flags::TREE)
        });

        let Some(flag) = flag else {
            Self::strict_check_long_flags(matches)?;
            let grid = grid::Options::deduce(matches)?;
            return Ok(Self::Grid(grid));
        };

        if flag.matches(&flags::LONG)
            || (flag.matches(&flags::TREE) && matches.has(&flags::LONG)?)
            || (flag.matches(&flags::GRID) && matches.has(&flags::LONG)?)
        {
            let _ = matches.has(&flags::LONG)?;
            let details = details::Options::deduce_long(matches, vars)?;

            let flag =
                matches.has_where_any(|f| f.matches(&flags::GRID) || f.matches(&flags::TREE));

            if flag.is_some() && flag.unwrap().matches(&flags::GRID) {
                let _ = matches.has(&flags::GRID)?;
                let row_threshold = RowThreshold::deduce(vars)?;
                let grid_details = grid_details::Options {
                    details,
                    row_threshold,
                };
                return Ok(Self::GridDetails(grid_details));
            }

            // the --tree case is handled by the DirAction parser later
            return Ok(Self::Details(details));
        }

        Self::strict_check_long_flags(matches)?;

        if flag.matches(&flags::TREE) {
            let _ = matches.has(&flags::TREE)?;
            let details = details::Options::deduce_tree(matches, vars)?;
            return Ok(Self::Details(details));
        }

        if flag.matches(&flags::ONE_LINE) {
            let _ = matches.has(&flags::ONE_LINE)?;
            return Ok(Self::Lines);
        }

        let grid = grid::Options::deduce(matches)?;
        Ok(Self::Grid(grid))
    }

    fn strict_check_long_flags(matches: &MatchedFlags<'_>) -> Result<(), OptionsError> {
        // If --long hasn’t been passed, then check if we need to warn the
        // user about flags that won’t have any effect.
        if matches.is_strict() {
            for option in &[
                &flags::BINARY,
                &flags::BYTES,
                &flags::INODE,
                &flags::LINKS,
                &flags::BLOCKSIZE,
                &flags::TIME,
                &flags::GROUP,
                &flags::NUMERIC,
                &flags::MOUNTS,
            ] {
                if matches.has(option)? {
                    return Err(OptionsError::Useless(option, false, &flags::LONG));
                }
            }

            if matches.has(&flags::HEADER)? {
                return Err(OptionsError::Useless(&flags::HEADER, false, &flags::LONG));
            }

            if matches.has(&flags::GIT)? && !matches.has(&flags::NO_GIT)? {
                return Err(OptionsError::Useless(&flags::GIT, false, &flags::LONG));
            } else if matches.has(&flags::LEVEL)?
                && !matches.has(&flags::RECURSE)?
                && !matches.has(&flags::TREE)?
            {
                return Err(OptionsError::Useless2(
                    &flags::LEVEL,
                    &flags::RECURSE,
                    &flags::TREE,
                ));
            }
        }

        Ok(())
    }
}

impl grid::Options {
    fn deduce(matches: &MatchedFlags<'_>) -> Result<Self, OptionsError> {
        let grid = grid::Options {
            across: matches.has(&flags::ACROSS)?,
        };

        Ok(grid)
    }
}

impl details::Options {
    fn deduce_tree<V: Vars>(matches: &MatchedFlags<'_>, vars: &V) -> Result<Self, OptionsError> {
        let details = details::Options {
            table: None,
            header: false,
            xattr: xattr::ENABLED && matches.has(&flags::EXTENDED)?,
            secattr: xattr::ENABLED && matches.has(&flags::SECURITY_CONTEXT)?,
            mounts: matches.has(&flags::MOUNTS)?,
            color_scale: ColorScaleOptions::deduce(matches, vars)?,
        };

        Ok(details)
    }

    fn deduce_long<V: Vars>(matches: &MatchedFlags<'_>, vars: &V) -> Result<Self, OptionsError> {
        if matches.is_strict() {
            if matches.has(&flags::ACROSS)? && !matches.has(&flags::GRID)? {
                return Err(OptionsError::Useless(&flags::ACROSS, true, &flags::LONG));
            } else if matches.has(&flags::ONE_LINE)? {
                return Err(OptionsError::Useless(&flags::ONE_LINE, true, &flags::LONG));
            }
        }

        Ok(details::Options {
            table: Some(TableOptions::deduce(matches, vars)?),
            header: Self::deduce_header(matches)?,
            xattr: xattr::ENABLED && matches.has(&flags::EXTENDED)?,
            secattr: xattr::ENABLED && matches.has(&flags::SECURITY_CONTEXT)?,
            mounts: matches.has(&flags::MOUNTS)?,
            color_scale: ColorScaleOptions::deduce(matches, vars)?,
        })
    }

    fn deduce_header(matches: &MatchedFlags<'_>) -> Result<bool, OptionsError> {
        let word = if let Some(w) = matches.get(&flags::HEADER)? {
            w.to_os_string()
        } else {
            return Ok(false);
        };

        match word.to_string_lossy().as_ref() {
            "auto" => Ok(io::stdout().is_terminal()),
            "always" => Ok(true),
            "never" => Ok(false),
            _ => Err(OptionsError::BadArgument(&flags::HEADER, word))?,
        }
    }
}

impl TerminalWidth {
    fn deduce<V: Vars>(matches: &MatchedFlags<'_>, vars: &V) -> Result<Self, OptionsError> {
        if let Some(width) = matches.get(&flags::WIDTH)? {
            let arg_str = width.to_string_lossy();
            match arg_str.parse() {
                Ok(w) => {
                    if w >= 1 {
                        Ok(Self::Set(w))
                    } else {
                        Ok(Self::Automatic)
                    }
                }
                Err(e) => {
                    let source = NumberSource::Arg(&flags::WIDTH);
                    Err(OptionsError::FailedParse(arg_str.to_string(), source, e))
                }
            }
        } else if let Some(columns) = vars.get(vars::COLUMNS).and_then(|s| s.into_string().ok()) {
            match columns.parse() {
                Ok(width) => Ok(Self::Set(width)),
                Err(e) => {
                    let source = NumberSource::Env(vars::COLUMNS);
                    Err(OptionsError::FailedParse(columns, source, e))
                }
            }
        } else {
            Ok(Self::Automatic)
        }
    }
}

impl RowThreshold {
    fn deduce<V: Vars>(vars: &V) -> Result<Self, OptionsError> {
        if let Some(columns) = vars
            .get_with_fallback(vars::EZA_GRID_ROWS, vars::EXA_GRID_ROWS)
            .and_then(|s| s.into_string().ok())
        {
            match columns.parse() {
                Ok(rows) => Ok(Self::MinimumRows(rows)),
                Err(e) => {
                    let source = NumberSource::Env(
                        vars.source(vars::EZA_GRID_ROWS, vars::EXA_GRID_ROWS)
                            .unwrap(),
                    );
                    Err(OptionsError::FailedParse(columns, source, e))
                }
            }
        } else {
            Ok(Self::AlwaysGrid)
        }
    }
}

impl TableOptions {
    fn deduce<V: Vars>(matches: &MatchedFlags<'_>, vars: &V) -> Result<Self, OptionsError> {
        let time_format = TimeFormat::deduce(matches, vars)?;
        let size_format = SizeFormat::deduce(matches)?;
        let user_format = UserFormat::deduce(matches)?;
        let group_format = GroupFormat::deduce(matches)?;
        let flags_format = FlagsFormat::deduce(vars);
        let columns = Columns::deduce(matches, vars)?;
        Ok(Self {
            size_format,
            time_format,
            user_format,
            group_format,
            flags_format,
            columns,
        })
    }
}

impl Columns {
    fn deduce<V: Vars>(matches: &MatchedFlags<'_>, vars: &V) -> Result<Self, OptionsError> {
        let time_types = TimeTypes::deduce(matches)?;

        let no_git_env = vars
            .get_with_fallback(vars::EXA_OVERRIDE_GIT, vars::EZA_OVERRIDE_GIT)
            .is_some();

        let git = matches.has(&flags::GIT)? && !matches.has(&flags::NO_GIT)? && !no_git_env;
        let subdir_git_repos =
            matches.has(&flags::GIT_REPOS)? && !matches.has(&flags::NO_GIT)? && !no_git_env;
        let subdir_git_repos_no_stat = !subdir_git_repos
            && matches.has(&flags::GIT_REPOS_NO_STAT)?
            && !matches.has(&flags::NO_GIT)?
            && !no_git_env;

        let blocksize = matches.has(&flags::BLOCKSIZE)?;
        let group = matches.has(&flags::GROUP)?;
        let inode = matches.has(&flags::INODE)?;
        let links = matches.has(&flags::LINKS)?;
        let octal = matches.has(&flags::OCTAL)?;
        let security_context = xattr::ENABLED && matches.has(&flags::SECURITY_CONTEXT)?;
        let file_flags = matches.has(&flags::FILE_FLAGS)?;

        let permissions = !matches.has(&flags::NO_PERMISSIONS)?;
        let filesize = !matches.has(&flags::NO_FILESIZE)?;
        let user = !matches.has(&flags::NO_USER)?;

        Ok(Self {
            time_types,
            inode,
            links,
            blocksize,
            group,
            git,
            subdir_git_repos,
            subdir_git_repos_no_stat,
            octal,
            security_context,
            file_flags,
            permissions,
            filesize,
            user,
        })
    }
}

impl SizeFormat {
    /// Determine which file size to use in the file size column based on
    /// the user’s options.
    ///
    /// The default mode is to use the decimal prefixes, as they are the
    /// most commonly-understood, and don’t involve trying to parse large
    /// strings of digits in your head. Changing the format to anything else
    /// involves the `--binary` or `--bytes` flags, and these conflict with
    /// each other.
    fn deduce(matches: &MatchedFlags<'_>) -> Result<Self, OptionsError> {
        let flag = matches.has_where(|f| f.matches(&flags::BINARY) || f.matches(&flags::BYTES))?;

        Ok(match flag {
            Some(f) if f.matches(&flags::BINARY) => Self::BinaryBytes,
            Some(f) if f.matches(&flags::BYTES) => Self::JustBytes,
            _ => Self::DecimalBytes,
        })
    }
}

impl TimeFormat {
    /// Determine how time should be formatted in timestamp columns.
    fn deduce<V: Vars>(matches: &MatchedFlags<'_>, vars: &V) -> Result<Self, OptionsError> {
        let word = if let Some(w) = matches.get(&flags::TIME_STYLE)? {
            w.to_os_string()
        } else {
            match vars.get(vars::TIME_STYLE) {
                Some(ref t) if !t.is_empty() => t.clone(),
                _ => return Ok(Self::DefaultFormat),
            }
        };

        match word.to_string_lossy().as_ref() {
            "default" => Ok(Self::DefaultFormat),
            "relative" => Ok(Self::Relative),
            "iso" => Ok(Self::ISOFormat),
            "long-iso" => Ok(Self::LongISO),
            "full-iso" => Ok(Self::FullISO),
            fmt if fmt.starts_with('+') => {
                let mut lines = fmt[1..].lines();

                // line 1 will be None when:
                //   - there is nothing after `+`
                // line 1 will be empty when:
                //   - `+` is followed immediately by `\n`
                let empty_non_recent_format_msg = "Custom timestamp format is empty, \
                    please supply a chrono format string after the plus sign.";
                let non_recent = lines.next().expect(empty_non_recent_format_msg);
                let non_recent = if non_recent.is_empty() {
                    panic!("{}", empty_non_recent_format_msg)
                } else {
                    non_recent.to_owned()
                };

                // line 2 will be None when:
                //   - there is not a single `\n`
                //   - there is nothing after the first `\n`
                // line 2 will be empty when:
                //   - there exist at least 2 `\n`, and no content between the 1st and 2nd `\n`
                let empty_recent_format_msg = "Custom timestamp format for recent files is empty, \
                    please supply a chrono format string at the second line.";
                let recent = lines.next().map(|rec| {
                    if rec.is_empty() {
                        panic!("{}", empty_recent_format_msg)
                    } else {
                        rec.to_owned()
                    }
                });

                Ok(Self::Custom { non_recent, recent })
            }
            _ => Err(OptionsError::BadArgument(&flags::TIME_STYLE, word)),
        }
    }
}

impl UserFormat {
    fn deduce(matches: &MatchedFlags<'_>) -> Result<Self, OptionsError> {
        let flag = matches.has(&flags::NUMERIC)?;
        Ok(if flag { Self::Numeric } else { Self::Name })
    }
}

impl GroupFormat {
    fn deduce(matches: &MatchedFlags<'_>) -> Result<Self, OptionsError> {
        let flag = matches.has(&flags::SMART_GROUP)?;
        Ok(if flag { Self::Smart } else { Self::Regular })
    }
}

impl TimeTypes {
    /// Determine which of a file’s time fields should be displayed for it
    /// based on the user’s options.
    ///
    /// There are two separate ways to pick which fields to show: with a
    /// flag (such as `--modified`) or with a parameter (such as
    /// `--time=modified`). An error is signaled if both ways are used.
    ///
    /// It’s valid to show more than one column by passing in more than one
    /// option, but passing *no* options means that the user just wants to
    /// see the default set.
    fn deduce(matches: &MatchedFlags<'_>) -> Result<Self, OptionsError> {
        let possible_word = matches.get(&flags::TIME)?;
        let modified = matches.has(&flags::MODIFIED)?;
        let changed = matches.has(&flags::CHANGED)?;
        let accessed = matches.has(&flags::ACCESSED)?;
        let created = matches.has(&flags::CREATED)?;

        let no_time = matches.has(&flags::NO_TIME)?;

        #[rustfmt::skip]
        let time_types = if no_time {
            Self {
                modified: false,
                changed: false,
                accessed: false,
                created: false,
            }
        } else if let Some(word) = possible_word {
            if modified {
                return Err(OptionsError::Useless(&flags::MODIFIED, true, &flags::TIME));
            } else if changed {
                return Err(OptionsError::Useless(&flags::CHANGED, true, &flags::TIME));
            } else if accessed {
                return Err(OptionsError::Useless(&flags::ACCESSED, true, &flags::TIME));
            } else if created {
                return Err(OptionsError::Useless(&flags::CREATED, true, &flags::TIME));
            } else if word == "mod" || word == "modified" {
                Self { modified: true,  changed: false, accessed: false, created: false }
            } else if word == "ch" || word == "changed" {
                Self { modified: false, changed: true,  accessed: false, created: false }
            } else if word == "acc" || word == "accessed" {
                Self { modified: false, changed: false, accessed: true,  created: false }
            } else if word == "cr" || word == "created" {
                Self { modified: false, changed: false, accessed: false, created: true  }
            } else {
                return Err(OptionsError::BadArgument(&flags::TIME, word.into()));
            }
        } else if modified || changed || accessed || created {
            Self {
                modified,
                changed,
                accessed,
                created,
            }
        } else {
            Self::default()
        };

        Ok(time_types)
    }
}

impl ColorScaleOptions {
    pub fn deduce<V: Vars>(matches: &MatchedFlags<'_>, vars: &V) -> Result<Self, OptionsError> {
        let min_luminance =
            match vars.get_with_fallback(vars::EZA_MIN_LUMINANCE, vars::EXA_MIN_LUMINANCE) {
                Some(var) => match var.to_string_lossy().parse() {
                    Ok(luminance) if (-100..=100).contains(&luminance) => luminance,
                    _ => 40,
                },
                None => 40,
            };

        let mode = if let Some(w) = matches
            .get(&flags::COLOR_SCALE_MODE)?
            .or(matches.get(&flags::COLOUR_SCALE_MODE)?)
        {
            match w.to_str() {
                Some("fixed") => ColorScaleMode::Fixed,
                Some("gradient") => ColorScaleMode::Gradient,
                _ => Err(OptionsError::BadArgument(
                    &flags::COLOR_SCALE_MODE,
                    w.to_os_string(),
                ))?,
            }
        } else {
            ColorScaleMode::Gradient
        };

        let mut options = ColorScaleOptions {
            mode,
            min_luminance,
            size: false,
            age: false,
        };

        let words = if let Some(w) = matches
            .get(&flags::COLOR_SCALE)?
            .or(matches.get(&flags::COLOUR_SCALE)?)
        {
            w.to_os_string()
        } else {
            return Ok(options);
        };

        for word in words.to_string_lossy().split(',') {
            match word {
                "all" => {
                    options.size = true;
                    options.age = true;
                }
                "age" => options.age = true,
                "size" => options.size = true,
                _ => Err(OptionsError::BadArgument(
                    &flags::COLOR_SCALE,
                    OsString::from(word),
                ))?,
            };
        }

        Ok(options)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::options::flags;
    use crate::options::parser::{Arg, Flag};
    use std::ffi::OsString;

    use crate::options::test::parse_for_test;
    use crate::options::test::Strictnesses::*;

    static TEST_ARGS: &[&Arg] = &[
        &flags::BINARY,
        &flags::BYTES,
        &flags::TIME_STYLE,
        &flags::TIME,
        &flags::MODIFIED,
        &flags::CHANGED,
        &flags::CREATED,
        &flags::ACCESSED,
        &flags::HEADER,
        &flags::GROUP,
        &flags::INODE,
        &flags::GIT,
        &flags::LINKS,
        &flags::BLOCKSIZE,
        &flags::LONG,
        &flags::LEVEL,
        &flags::GRID,
        &flags::ACROSS,
        &flags::ONE_LINE,
        &flags::TREE,
        &flags::NUMERIC,
    ];

    #[allow(unused_macro_rules)]
    macro_rules! test {
        ($name:ident: $type:ident <- $inputs:expr; $stricts:expr => $result:expr) => {
            /// Macro that writes a test.
            /// If testing both strictnesses, they’ll both be done in the same function.
            #[test]
            fn $name() {
                for result in parse_for_test($inputs.as_ref(), TEST_ARGS, $stricts, |mf| {
                    $type::deduce(mf)
                }) {
                    assert_eq!(result, $result);
                }
            }
        };

        ($name:ident: $type:ident <- $inputs:expr; $stricts:expr => err $result:expr) => {
            /// Special macro for testing Err results.
            /// This is needed because sometimes the Ok type doesn’t implement `PartialEq`.
            #[test]
            fn $name() {
                for result in parse_for_test($inputs.as_ref(), TEST_ARGS, $stricts, |mf| {
                    $type::deduce(mf)
                }) {
                    assert_eq!(result.unwrap_err(), $result);
                }
            }
        };

        ($name:ident: $type:ident <- $inputs:expr; $stricts:expr => like $pat:pat) => {
            /// More general macro for testing against a pattern.
            /// Instead of using `PartialEq`, this just tests if it matches a pat.
            #[test]
            fn $name() {
                for result in parse_for_test($inputs.as_ref(), TEST_ARGS, $stricts, |mf| {
                    $type::deduce(mf)
                }) {
                    println!("Testing {:?}", result);
                    match result {
                        $pat => assert!(true),
                        _ => assert!(false),
                    }
                }
            }
        };

        ($name:ident: $type:ident <- $inputs:expr, $vars:expr; $stricts:expr => err $result:expr) => {
            /// Like above, but with $vars.
            #[test]
            fn $name() {
                for result in parse_for_test($inputs.as_ref(), TEST_ARGS, $stricts, |mf| {
                    $type::deduce(mf, &$vars)
                }) {
                    assert_eq!(result.unwrap_err(), $result);
                }
            }
        };

        ($name:ident: $type:ident <- $inputs:expr, $vars:expr; $stricts:expr => like $pat:pat) => {
            /// Like further above, but with $vars.
            #[test]
            fn $name() {
                for result in parse_for_test($inputs.as_ref(), TEST_ARGS, $stricts, |mf| {
                    $type::deduce(mf, &$vars)
                }) {
                    println!("Testing {:?}", result);
                    match result {
                        $pat => assert!(true),
                        _ => assert!(false),
                    }
                }
            }
        };
    }

    mod size_formats {
        use super::*;

        // Default behaviour
        test!(empty:   SizeFormat <- [];                       Both => Ok(SizeFormat::DecimalBytes));

        // Individual flags
        test!(binary:  SizeFormat <- ["--binary"];             Both => Ok(SizeFormat::BinaryBytes));
        test!(bytes:   SizeFormat <- ["--bytes"];              Both => Ok(SizeFormat::JustBytes));

        // Overriding
        test!(both_1:  SizeFormat <- ["--binary", "--binary"];  Last => Ok(SizeFormat::BinaryBytes));
        test!(both_2:  SizeFormat <- ["--bytes",  "--binary"];  Last => Ok(SizeFormat::BinaryBytes));
        test!(both_3:  SizeFormat <- ["--binary", "--bytes"];   Last => Ok(SizeFormat::JustBytes));
        test!(both_4:  SizeFormat <- ["--bytes",  "--bytes"];   Last => Ok(SizeFormat::JustBytes));

        test!(both_5:  SizeFormat <- ["--binary", "--binary"];  Complain => err OptionsError::Duplicate(Flag::Long("binary"), Flag::Long("binary")));
        test!(both_6:  SizeFormat <- ["--bytes",  "--binary"];  Complain => err OptionsError::Duplicate(Flag::Long("bytes"),  Flag::Long("binary")));
        test!(both_7:  SizeFormat <- ["--binary", "--bytes"];   Complain => err OptionsError::Duplicate(Flag::Long("binary"), Flag::Long("bytes")));
        test!(both_8:  SizeFormat <- ["--bytes",  "--bytes"];   Complain => err OptionsError::Duplicate(Flag::Long("bytes"),  Flag::Long("bytes")));
    }

    mod time_formats {
        use super::*;

        // These tests use pattern matching because TimeFormat doesn’t
        // implement PartialEq.

        // Default behaviour
        test!(empty:     TimeFormat <- [], None;                            Both => like Ok(TimeFormat::DefaultFormat));

        // Individual settings
        test!(default:                TimeFormat <- ["--time-style=default"], None;               Both => like Ok(TimeFormat::DefaultFormat));
        test!(iso:                    TimeFormat <- ["--time-style", "iso"], None;                Both => like Ok(TimeFormat::ISOFormat));
        test!(relative:               TimeFormat <- ["--time-style", "relative"], None;           Both => like Ok(TimeFormat::Relative));
        test!(long_iso:               TimeFormat <- ["--time-style=long-iso"], None;              Both => like Ok(TimeFormat::LongISO));
        test!(full_iso:               TimeFormat <- ["--time-style", "full-iso"], None;           Both => like Ok(TimeFormat::FullISO));
        test!(custom_style:           TimeFormat <- ["--time-style", "+%Y/%m/%d"], None;          Both => like Ok(TimeFormat::Custom { recent: None, .. }));
        test!(custom_style_multiline: TimeFormat <- ["--time-style", "+%Y/%m/%d\n--%m-%d"], None; Both => like Ok(TimeFormat::Custom { recent: Some(_), .. }));
        test!(bad_custom_style:       TimeFormat <- ["--time-style", "%Y/%m/%d"], None;           Both => err OptionsError::BadArgument(&flags::TIME_STYLE, OsString::from("%Y/%m/%d")));

        // Overriding
        test!(actually:  TimeFormat <- ["--time-style=default", "--time-style", "iso"], None;  Last => like Ok(TimeFormat::ISOFormat));
        test!(actual_2:  TimeFormat <- ["--time-style=default", "--time-style", "iso"], None;  Complain => err OptionsError::Duplicate(Flag::Long("time-style"), Flag::Long("time-style")));

        test!(nevermind: TimeFormat <- ["--time-style", "long-iso", "--time-style=full-iso"], None;  Last => like Ok(TimeFormat::FullISO));
        test!(nevermore: TimeFormat <- ["--time-style", "long-iso", "--time-style=full-iso"], None;  Complain => err OptionsError::Duplicate(Flag::Long("time-style"), Flag::Long("time-style")));

        // Errors
        test!(daily:     TimeFormat <- ["--time-style=24-hour"], None;  Both => err OptionsError::BadArgument(&flags::TIME_STYLE, OsString::from("24-hour")));

        // `TIME_STYLE` environment variable is defined.
        // If the time-style argument is not given, `TIME_STYLE` is used.
        test!(use_env:     TimeFormat <- [], Some("long-iso".into());  Both => like Ok(TimeFormat::LongISO));

        // If the time-style argument is given, `TIME_STYLE` is overriding.
        test!(override_env:     TimeFormat <- ["--time-style=full-iso"], Some("long-iso".into());  Both => like Ok(TimeFormat::FullISO));
    }

    mod time_types {
        use super::*;

        // Default behaviour
        test!(empty:     TimeTypes <- [];                      Both => Ok(TimeTypes::default()));

        // Modified
        test!(modified:  TimeTypes <- ["--modified"];          Both => Ok(TimeTypes { modified: true,  changed: false, accessed: false, created: false }));
        test!(m:         TimeTypes <- ["-m"];                  Both => Ok(TimeTypes { modified: true,  changed: false, accessed: false, created: false }));
        test!(time_mod:  TimeTypes <- ["--time=modified"];     Both => Ok(TimeTypes { modified: true,  changed: false, accessed: false, created: false }));
        test!(t_m:       TimeTypes <- ["-tmod"];               Both => Ok(TimeTypes { modified: true,  changed: false, accessed: false, created: false }));

        // Changed
        #[cfg(target_family = "unix")]
        test!(changed:   TimeTypes <- ["--changed"];           Both => Ok(TimeTypes { modified: false, changed: true,  accessed: false, created: false }));
        #[cfg(target_family = "unix")]
        test!(time_ch:   TimeTypes <- ["--time=changed"];      Both => Ok(TimeTypes { modified: false, changed: true,  accessed: false, created: false }));
        #[cfg(target_family = "unix")]
        test!(t_ch:    TimeTypes <- ["-t", "ch"];              Both => Ok(TimeTypes { modified: false, changed: true,  accessed: false, created: false }));

        // Accessed
        test!(acc:       TimeTypes <- ["--accessed"];          Both => Ok(TimeTypes { modified: false, changed: false, accessed: true,  created: false }));
        test!(a:         TimeTypes <- ["-u"];                  Both => Ok(TimeTypes { modified: false, changed: false, accessed: true,  created: false }));
        test!(time_acc:  TimeTypes <- ["--time", "accessed"];  Both => Ok(TimeTypes { modified: false, changed: false, accessed: true,  created: false }));
        test!(time_a:    TimeTypes <- ["-t", "acc"];           Both => Ok(TimeTypes { modified: false, changed: false, accessed: true,  created: false }));

        // Created
        test!(cr:        TimeTypes <- ["--created"];           Both => Ok(TimeTypes { modified: false, changed: false, accessed: false, created: true  }));
        test!(c:         TimeTypes <- ["-U"];                  Both => Ok(TimeTypes { modified: false, changed: false, accessed: false, created: true  }));
        test!(time_cr:   TimeTypes <- ["--time=created"];      Both => Ok(TimeTypes { modified: false, changed: false, accessed: false, created: true  }));
        test!(t_cr:      TimeTypes <- ["-tcr"];                Both => Ok(TimeTypes { modified: false, changed: false, accessed: false, created: true  }));

        // Multiples
        test!(time_uu:   TimeTypes <- ["-u", "--modified"];    Both => Ok(TimeTypes { modified: true,  changed: false, accessed: true,  created: false }));

        // Errors
        test!(time_tea:  TimeTypes <- ["--time=tea"];          Both => err OptionsError::BadArgument(&flags::TIME, OsString::from("tea")));
        test!(t_ea:      TimeTypes <- ["-tea"];                Both => err OptionsError::BadArgument(&flags::TIME, OsString::from("ea")));

        // Overriding
        test!(overridden:   TimeTypes <- ["-tcr", "-tmod"];    Last => Ok(TimeTypes { modified: true,  changed: false, accessed: false, created: false }));
        test!(overridden_2: TimeTypes <- ["-tcr", "-tmod"];    Complain => err OptionsError::Duplicate(Flag::Short(b't'), Flag::Short(b't')));
    }

    mod views {
        use super::*;

        use crate::output::grid::Options as GridOptions;

        // Default
        test!(empty:         Mode <- [], None;            Both => like Ok(Mode::Grid(_)));

        // Grid views
        test!(original_g:    Mode <- ["-G"], None;        Both => like Ok(Mode::Grid(GridOptions { across: false, .. })));
        test!(grid:          Mode <- ["--grid"], None;    Both => like Ok(Mode::Grid(GridOptions { across: false, .. })));
        test!(across:        Mode <- ["--across"], None;  Both => like Ok(Mode::Grid(GridOptions { across: true,  .. })));
        test!(gracross:      Mode <- ["-xG"], None;       Both => like Ok(Mode::Grid(GridOptions { across: true,  .. })));

        // Lines views
        test!(lines:         Mode <- ["--oneline"], None;     Both => like Ok(Mode::Lines));
        test!(prima:         Mode <- ["-1"], None;            Both => like Ok(Mode::Lines));

        // Details views
        test!(long:          Mode <- ["--long"], None;    Both => like Ok(Mode::Details(_)));
        test!(ell:           Mode <- ["-l"], None;        Both => like Ok(Mode::Details(_)));

        // Grid-details views
        test!(lid:           Mode <- ["--long", "--grid"], None;  Both => like Ok(Mode::GridDetails(_)));
        test!(leg:           Mode <- ["-lG"], None;               Both => like Ok(Mode::GridDetails(_)));

        // Options that do nothing with --long
        test!(long_across:   Mode <- ["--long", "--across"],   None;  Last => like Ok(Mode::Details(_)));

        // Options that do nothing without --long
        test!(just_header:   Mode <- ["--header"],    None;  Last => like Ok(Mode::Grid(_)));
        test!(just_group:    Mode <- ["--group"],     None;  Last => like Ok(Mode::Grid(_)));
        test!(just_inode:    Mode <- ["--inode"],     None;  Last => like Ok(Mode::Grid(_)));
        test!(just_links:    Mode <- ["--links"],     None;  Last => like Ok(Mode::Grid(_)));
        test!(just_blocks:   Mode <- ["--blocksize"], None;  Last => like Ok(Mode::Grid(_)));
        test!(just_binary:   Mode <- ["--binary"],    None;  Last => like Ok(Mode::Grid(_)));
        test!(just_bytes:    Mode <- ["--bytes"],     None;  Last => like Ok(Mode::Grid(_)));
        test!(just_numeric:  Mode <- ["--numeric"],   None;  Last => like Ok(Mode::Grid(_)));

        #[cfg(feature = "git")]
        test!(just_git:      Mode <- ["--git"],       None;  Last => like Ok(Mode::Grid(_)));

        //test!(just_header_2: Mode <- ["--header"],    None;  Complain => err OptionsError::Useless(&flags::HEADER,  false, &flags::LONG));
        test!(just_group_2:  Mode <- ["--group"],     None;  Complain => err OptionsError::Useless(&flags::GROUP,   false, &flags::LONG));
        test!(just_inode_2:  Mode <- ["--inode"],     None;  Complain => err OptionsError::Useless(&flags::INODE,   false, &flags::LONG));
        test!(just_links_2:  Mode <- ["--links"],     None;  Complain => err OptionsError::Useless(&flags::LINKS,   false, &flags::LONG));
        test!(just_blocks_2: Mode <- ["--blocksize"], None;  Complain => err OptionsError::Useless(&flags::BLOCKSIZE,  false, &flags::LONG));
        test!(just_binary_2: Mode <- ["--binary"],    None;  Complain => err OptionsError::Useless(&flags::BINARY,  false, &flags::LONG));
        test!(just_bytes_2:  Mode <- ["--bytes"],     None;  Complain => err OptionsError::Useless(&flags::BYTES,   false, &flags::LONG));
        test!(just_numeric2: Mode <- ["--numeric"],   None;  Complain => err OptionsError::Useless(&flags::NUMERIC, false, &flags::LONG));

        #[cfg(feature = "git")]
        test!(just_git_2:    Mode <- ["--git"],    None;  Complain => err OptionsError::Useless(&flags::GIT,    false, &flags::LONG));

        // Contradictions and combinations
        test!(lgo:           Mode <- ["--long", "--grid", "--oneline"], None;  Both => like Ok(Mode::Lines));
        test!(lgt:           Mode <- ["--long", "--grid", "--tree"],    None;  Both => like Ok(Mode::Details(_)));
        test!(tgl:           Mode <- ["--tree", "--grid", "--long"],    None;  Both => like Ok(Mode::GridDetails(_)));
        test!(tlg:           Mode <- ["--tree", "--long", "--grid"],    None;  Both => like Ok(Mode::GridDetails(_)));
        test!(ot:            Mode <- ["--oneline", "--tree"],           None;  Both => like Ok(Mode::Details(_)));
        test!(og:            Mode <- ["--oneline", "--grid"],           None;  Both => like Ok(Mode::Grid(_)));
        test!(tg:            Mode <- ["--tree", "--grid"],              None;  Both => like Ok(Mode::Grid(_)));
    }
}
