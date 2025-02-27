// SPDX-FileCopyrightText: 2024 Christina Sørensen
// SPDX-License-Identifier: EUPL-1.2
//
// SPDX-FileCopyrightText: 2023-2024 Christina Sørensen, eza contributors
// SPDX-FileCopyrightText: 2014 Benjamin Sago
// SPDX-License-Identifier: MIT
//! Parsing the options for `FileFilter`.

use crate::fs::filter::{
    FileFilter, FileFilterFlags, GitIgnore, IgnorePatterns, SortCase, SortField,
};
use crate::fs::DotFilter;

use crate::options::parser::MatchedFlags;
use crate::options::{flags, OptionsError};

impl FileFilter {
    /// Determines which of all the file filter options to use.
    pub fn deduce(matches: &MatchedFlags<'_>) -> Result<Self, OptionsError> {
        use FileFilterFlags as FFF;
        let mut filter_flags: Vec<FileFilterFlags> = vec![];

        for (has, flag) in &[
            (matches.has(&flags::REVERSE)?, FFF::Reverse),
            (matches.has(&flags::ONLY_DIRS)?, FFF::OnlyDirs),
            (matches.has(&flags::ONLY_FILES)?, FFF::OnlyFiles),
            (matches.has(&flags::NO_SYMLINKS)?, FFF::NoSymlinks),
            (matches.has(&flags::SHOW_SYMLINKS)?, FFF::ShowSymlinks),
            (matches.has(&flags::DIRS_LAST)?, FFF::ListDirsLast),
            (matches.has(&flags::DIRS_FIRST)?, FFF::ListDirsFirst),
        ] {
            if *has {
                filter_flags.push(flag.clone());
            }
        }

        #[rustfmt::skip]
        return Ok(Self {
            no_symlinks:      filter_flags.contains(&FFF::NoSymlinks),
            show_symlinks:    filter_flags.contains(&FFF::ShowSymlinks),
            flags:            filter_flags,
            sort_field:       SortField::deduce(matches)?,
            dot_filter:       DotFilter::deduce(matches)?,
            ignore_patterns:  IgnorePatterns::deduce(matches)?,
            git_ignore:       GitIgnore::deduce(matches)?,
        });
    }
}

impl SortField {
    /// Determines which sort field to use based on the `--sort` argument.
    /// This argument’s value can be one of several flags, listed above.
    /// Returns the default sort field if none is given, or `Err` if the
    /// value doesn’t correspond to a sort field we know about.
    fn deduce(matches: &MatchedFlags<'_>) -> Result<Self, OptionsError> {
        let Some(word) = matches.get(&flags::SORT)? else {
            return Ok(Self::default());
        };

        // Get String because we can’t match an OsStr
        let Some(word) = word.to_str() else {
            return Err(OptionsError::BadArgument(&flags::SORT, word.into()));
        };

        let field = match word {
            "name" | "filename" => Self::Name(SortCase::AaBbCc),
            "Name" | "Filename" => Self::Name(SortCase::ABCabc),
            ".name" | ".filename" => Self::NameMixHidden(SortCase::AaBbCc),
            ".Name" | ".Filename" => Self::NameMixHidden(SortCase::ABCabc),
            "size" | "filesize" => Self::Size,
            #[cfg(unix)]
            "block" | "blocks" | "blocksize" => Self::BlockSize,
            "ext" | "extension" => Self::Extension(SortCase::AaBbCc),
            "Ext" | "Extension" => Self::Extension(SortCase::ABCabc),

            // “new” sorts oldest at the top and newest at the bottom; “old”
            // sorts newest at the top and oldest at the bottom. I think this
            // is the right way round to do this: “size” puts the smallest at
            // the top and the largest at the bottom, doesn’t it?
            "date" | "time" | "mod" | "modified" | "new" | "newest" => Self::ModifiedDate,

            // Similarly, “age” means that files with the least age (the
            // newest files) get sorted at the top, and files with the most
            // age (the oldest) at the bottom.
            "age" | "old" | "oldest" => Self::ModifiedAge,

            "ch" | "changed" => Self::ChangedDate,
            "acc" | "accessed" => Self::AccessedDate,
            "cr" | "created" => Self::CreatedDate,
            #[cfg(unix)]
            "inode" => Self::FileInode,
            "type" => Self::FileType,
            "none" => Self::Unsorted,
            _ => {
                return Err(OptionsError::BadArgument(&flags::SORT, word.into()));
            }
        };

        Ok(field)
    }
}

// I’ve gone back and forth between whether to sort case-sensitively or
// insensitively by default. The default string sort in most programming
// languages takes each character’s ASCII value into account, sorting
// “Documents” before “apps”, but there’s usually an option to ignore
// characters’ case, putting “apps” before “Documents”.
//
// The argument for following case is that it’s easy to forget whether an item
// begins with an uppercase or lowercase letter and end up having to scan both
// the uppercase and lowercase sub-lists to find the item you want. If you
// happen to pick the sublist it’s not in, it looks like it’s missing, which
// is worse than if you just take longer to find it.
// (https://ux.stackexchange.com/a/79266)
//
// The argument for ignoring case is that it makes exa sort files differently
// from shells. A user would expect a directory’s files to be in the same
// order if they used “exa ~/directory” or “exa ~/directory/*”, but exa sorts
// them in the first case, and the shell in the second case, so they wouldn’t
// be exactly the same if exa does something non-conventional.
//
// However, exa already sorts files differently: it uses natural sorting from
// the natord crate, sorting the string “2” before “10” because the number’s
// smaller, because that’s usually what the user expects to happen. Users will
// name their files with numbers expecting them to be treated like numbers,
// rather than lists of numeric characters.
//
// In the same way, users will name their files with letters expecting the
// order of the letters to matter, rather than each letter’s character’s ASCII
// value. So exa breaks from tradition and ignores case while sorting:
// “apps” first, then “Documents”.
//
// You can get the old behaviour back by sorting with `--sort=Name`.
impl Default for SortField {
    fn default() -> Self {
        Self::Name(SortCase::AaBbCc)
    }
}

impl DotFilter {
    /// Determines the dot filter based on how many `--all` options were
    /// given: one will show dotfiles, but two will show `.` and `..` too.
    /// --almost-all is equivalent to --all, included for compatibility with
    /// `ls -A`.
    ///
    /// It also checks for the `--tree` option, because of a special case
    /// where `--tree --all --all` won’t work: listing the parent directory
    /// in tree mode would loop onto itself!
    ///
    /// `--almost-all` binds stronger than multiple `--all` as we currently do not take the order
    /// of arguments into account and it is the safer option (does not clash with `--tree`)
    pub fn deduce(matches: &MatchedFlags<'_>) -> Result<Self, OptionsError> {
        let all_count = matches.count(&flags::ALL);
        let has_almost_all = matches.has(&flags::ALMOST_ALL)?;

        match (all_count, has_almost_all) {
            (0, false) => Ok(Self::JustFiles),

            // either a single --all or at least one --almost-all is given
            (1, _) | (0, true) => Ok(Self::Dotfiles),
            // more than one --all
            (c, _) => {
                if matches.count(&flags::TREE) > 0 {
                    Err(OptionsError::TreeAllAll)
                } else if matches.is_strict() && c > 2 {
                    Err(OptionsError::Conflict(&flags::ALL, &flags::ALL))
                } else {
                    Ok(Self::DotfilesAndDots)
                }
            }
        }
    }
}

impl IgnorePatterns {
    /// Determines the set of glob patterns to use based on the
    /// `--ignore-glob` argument’s value. This is a list of strings
    /// separated by pipe (`|`) characters, given in any order.
    pub fn deduce(matches: &MatchedFlags<'_>) -> Result<Self, OptionsError> {
        // If there are no inputs, we return a set of patterns that doesn’t
        // match anything, rather than, say, `None`.
        let Some(inputs) = matches.get(&flags::IGNORE_GLOB)? else {
            return Ok(Self::empty());
        };

        // Awkwardly, though, a glob pattern can be invalid, and we need to
        // deal with invalid patterns somehow.
        let (patterns, mut errors) = Self::parse_from_iter(inputs.to_string_lossy().split('|'));

        // It can actually return more than one glob error,
        // but we only use one. (TODO)
        match errors.pop() {
            Some(e) => Err(e.into()),
            None => Ok(patterns),
        }
    }
}

impl GitIgnore {
    pub fn deduce(matches: &MatchedFlags<'_>) -> Result<Self, OptionsError> {
        if matches.has(&flags::GIT_IGNORE)? {
            Ok(Self::CheckAndIgnore)
        } else {
            Ok(Self::Off)
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::options::flags;
    use crate::options::parser::Flag;
    use std::ffi::OsString;

    macro_rules! test {
        ($name:ident: $type:ident <- $inputs:expr; $stricts:expr => $result:expr) => {
            #[test]
            fn $name() {
                use crate::options::parser::Arg;
                use crate::options::test::parse_for_test;
                use crate::options::test::Strictnesses::*;

                static TEST_ARGS: &[&Arg] = &[
                    &flags::SORT,
                    &flags::ALL,
                    &flags::ALMOST_ALL,
                    &flags::TREE,
                    &flags::IGNORE_GLOB,
                    &flags::GIT_IGNORE,
                ];
                for result in parse_for_test($inputs.as_ref(), TEST_ARGS, $stricts, |mf| {
                    $type::deduce(mf)
                }) {
                    assert_eq!(result, $result);
                }
            }
        };
    }

    mod sort_fields {
        use super::*;

        // Default behaviour
        test!(empty:         SortField <- [];                  Both => Ok(SortField::default()));

        // Sort field arguments
        test!(one_arg:       SortField <- ["--sort=mod"];      Both => Ok(SortField::ModifiedDate));
        test!(one_long:      SortField <- ["--sort=size"];     Both => Ok(SortField::Size));
        test!(one_short:     SortField <- ["-saccessed"];      Both => Ok(SortField::AccessedDate));
        test!(lowercase:     SortField <- ["--sort", "name"];  Both => Ok(SortField::Name(SortCase::AaBbCc)));
        test!(uppercase:     SortField <- ["--sort", "Name"];  Both => Ok(SortField::Name(SortCase::ABCabc)));
        test!(old:           SortField <- ["--sort", "new"];   Both => Ok(SortField::ModifiedDate));
        test!(oldest:        SortField <- ["--sort=newest"];   Both => Ok(SortField::ModifiedDate));
        test!(new:           SortField <- ["--sort", "old"];   Both => Ok(SortField::ModifiedAge));
        test!(newest:        SortField <- ["--sort=oldest"];   Both => Ok(SortField::ModifiedAge));
        test!(age:           SortField <- ["-sage"];           Both => Ok(SortField::ModifiedAge));

        test!(mix_hidden_lowercase:     SortField <- ["--sort", ".name"];  Both => Ok(SortField::NameMixHidden(SortCase::AaBbCc)));
        test!(mix_hidden_uppercase:     SortField <- ["--sort", ".Name"];  Both => Ok(SortField::NameMixHidden(SortCase::ABCabc)));

        // Errors
        test!(error:         SortField <- ["--sort=colour"];   Both => Err(OptionsError::BadArgument(&flags::SORT, OsString::from("colour"))));

        // Overriding
        test!(overridden:    SortField <- ["--sort=cr",       "--sort", "mod"];     Last => Ok(SortField::ModifiedDate));
        test!(overridden_2:  SortField <- ["--sort", "none",  "--sort=Extension"];  Last => Ok(SortField::Extension(SortCase::ABCabc)));
        test!(overridden_3:  SortField <- ["--sort=cr",       "--sort", "mod"];     Complain => Err(OptionsError::Duplicate(Flag::Long("sort"), Flag::Long("sort"))));
        test!(overridden_4:  SortField <- ["--sort", "none",  "--sort=Extension"];  Complain => Err(OptionsError::Duplicate(Flag::Long("sort"), Flag::Long("sort"))));
    }

    mod dot_filters {
        use super::*;

        // Default behaviour
        test!(empty:        DotFilter <- [];               Both => Ok(DotFilter::JustFiles));

        // --all
        test!(all:              DotFilter <- ["--all"];        Both => Ok(DotFilter::Dotfiles));
        test!(all_all:          DotFilter <- ["--all", "-a"];  Both => Ok(DotFilter::DotfilesAndDots));
        test!(all_all_2:        DotFilter <- ["-aa"];          Both => Ok(DotFilter::DotfilesAndDots));

        test!(all_all_3:        DotFilter <- ["-aaa"];         Last => Ok(DotFilter::DotfilesAndDots));
        test!(all_all_4:        DotFilter <- ["-aaa"];         Complain => Err(OptionsError::Conflict(&flags::ALL, &flags::ALL)));

        // --all and --tree
        test!(tree_a:           DotFilter <- ["-Ta"];          Both => Ok(DotFilter::Dotfiles));
        test!(tree_aa:          DotFilter <- ["-Taa"];         Both => Err(OptionsError::TreeAllAll));
        test!(tree_aaa:         DotFilter <- ["-Taaa"];        Both => Err(OptionsError::TreeAllAll));

        // --almost-all
        test!(almost_all:       DotFilter <- ["--almost-all"]; Both => Ok(DotFilter::Dotfiles));
        test!(almost_all_all:   DotFilter <- ["-Aa"];          Both => Ok(DotFilter::Dotfiles));
        test!(almost_all_all_2: DotFilter <- ["-Aaa"];         Both => Ok(DotFilter::DotfilesAndDots));
    }

    mod ignore_patterns {
        use super::*;
        use std::iter::FromIterator;

        fn pat(string: &'static str) -> glob::Pattern {
            glob::Pattern::new(string).unwrap()
        }

        // Various numbers of globs
        test!(none:   IgnorePatterns <- [];                                        Both => Ok(IgnorePatterns::empty()));
        test!(one:    IgnorePatterns <- ["--ignore-glob", "*.ogg"];                Both => Ok(IgnorePatterns::from_iter(vec![ pat("*.ogg") ])));
        test!(two:    IgnorePatterns <- ["--ignore-glob=*.ogg|*.MP3"];             Both => Ok(IgnorePatterns::from_iter(vec![ pat("*.ogg"), pat("*.MP3") ])));
        test!(loads:  IgnorePatterns <- ["-I*|?|.|*"];                             Both => Ok(IgnorePatterns::from_iter(vec![ pat("*"), pat("?"), pat("."), pat("*") ])));

        // Overriding
        test!(overridden:   IgnorePatterns <- ["-I=*.ogg",    "-I", "*.mp3"];      Last => Ok(IgnorePatterns::from_iter(vec![ pat("*.mp3") ])));
        test!(overridden_2: IgnorePatterns <- ["-I", "*.OGG", "-I*.MP3"];          Last => Ok(IgnorePatterns::from_iter(vec![ pat("*.MP3") ])));
        test!(overridden_3: IgnorePatterns <- ["-I=*.ogg",    "-I", "*.mp3"];  Complain => Err(OptionsError::Duplicate(Flag::Short(b'I'), Flag::Short(b'I'))));
        test!(overridden_4: IgnorePatterns <- ["-I", "*.OGG", "-I*.MP3"];      Complain => Err(OptionsError::Duplicate(Flag::Short(b'I'), Flag::Short(b'I'))));
    }

    mod git_ignores {
        use super::*;

        test!(off:  GitIgnore <- [];                Both => Ok(GitIgnore::Off));
        test!(on:   GitIgnore <- ["--git-ignore"];  Both => Ok(GitIgnore::CheckAndIgnore));
    }
}
