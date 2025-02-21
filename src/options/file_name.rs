// SPDX-FileCopyrightText: 2024 Christina Sørensen
// SPDX-License-Identifier: EUPL-1.2
//
// SPDX-FileCopyrightText: 2023-2024 Christina Sørensen, eza contributors
// SPDX-FileCopyrightText: 2014 Benjamin Sago
// SPDX-License-Identifier: MIT
use crate::options::parser::MatchedFlags;
use crate::options::vars::{self, Vars};
use crate::options::{flags, NumberSource, OptionsError};

use crate::output::file_name::{
    Absolute, Classify, EmbedHyperlinks, Options, QuoteStyle, ShowIcons,
};

impl Options {
    pub fn deduce<V: Vars>(
        matches: &MatchedFlags<'_>,
        vars: &V,
        is_a_tty: bool,
    ) -> Result<Self, OptionsError> {
        let classify = Classify::deduce(matches)?;
        let show_icons = ShowIcons::deduce(matches, vars)?;

        let quote_style = QuoteStyle::deduce(matches)?;
        let embed_hyperlinks = EmbedHyperlinks::deduce(matches)?;

        let absolute = Absolute::deduce(matches)?;

        Ok(Self {
            classify,
            show_icons,
            quote_style,
            embed_hyperlinks,
            absolute,
            is_a_tty,
        })
    }
}

impl Classify {
    fn deduce(matches: &MatchedFlags<'_>) -> Result<Self, OptionsError> {
        let mode_opt = matches.get(&flags::CLASSIFY)?;

        match mode_opt {
            Some(word) => match word.to_str() {
                Some("always") => Ok(Self::AddFileIndicators),
                Some("auto" | "automatic") => Ok(Self::AutomaticAddFileIndicators),
                Some("never") => Ok(Self::JustFilenames),
                _ => Err(OptionsError::BadArgument(&flags::CLASSIFY, word.into())),
            },
            // No flag given, default to just filenames
            None => Ok(Self::JustFilenames),
        }
    }
}

impl ShowIcons {
    pub fn deduce<V: Vars>(matches: &MatchedFlags<'_>, vars: &V) -> Result<Self, OptionsError> {
        enum AlwaysOrAuto {
            Always,
            Automatic,
        }

        let force_icons = vars.get(vars::EZA_ICONS_AUTO).is_some();
        let mode_opt = matches.get(&flags::ICONS)?;
        if !force_icons && !matches.has(&flags::ICONS)? && mode_opt.is_none() {
            return Ok(Self::Never);
        }

        let mode = match mode_opt {
            Some(word) => match word.to_str() {
                Some("always") => AlwaysOrAuto::Always,
                Some("auto" | "automatic") => AlwaysOrAuto::Automatic,
                Some("never") => return Ok(Self::Never),
                None => AlwaysOrAuto::Automatic,
                _ => return Err(OptionsError::BadArgument(&flags::ICONS, word.into())),
            },
            None => AlwaysOrAuto::Automatic,
        };

        let width = if let Some(columns) = vars
            .get_with_fallback(vars::EXA_ICON_SPACING, vars::EZA_ICON_SPACING)
            .and_then(|s| s.into_string().ok())
        {
            match columns.parse() {
                Ok(width) => width,
                Err(e) => {
                    let source = NumberSource::Env(
                        vars.source(vars::EXA_ICON_SPACING, vars::EZA_ICON_SPACING)
                            .unwrap(),
                    );
                    return Err(OptionsError::FailedParse(columns, source, e));
                }
            }
        } else {
            1
        };

        match mode {
            AlwaysOrAuto::Always => Ok(Self::Always(width)),
            AlwaysOrAuto::Automatic => Ok(Self::Automatic(width)),
        }
    }
}

impl QuoteStyle {
    pub fn deduce(matches: &MatchedFlags<'_>) -> Result<Self, OptionsError> {
        let env = std::env::var("EZA_QUOTING_STYLE");
        let env = env.unwrap_or_else(|_| "auto".to_string());
        let env = match env.to_ascii_lowercase().as_str() {
            "always" => Self::Always,
            "never" => Self::Never,
            _ => Self::Auto,
        };
        if matches.get(&flags::QUOTES)?.is_none() {
            Ok(env)
        } else if let Some(word) = matches.get(&flags::QUOTES)? {
            match word.to_str() {
                Some("always") => Ok(Self::Always),
                Some("never") => Ok(Self::Never),
                Some("auto" | "automatic") => Ok(Self::Auto),
                Some(arg) => Err(OptionsError::BadArgument(&flags::QUOTES, arg.into())),
                _ => Err(OptionsError::BadArgument(&flags::QUOTES, "auto".into())),
            }
        } else {
            Ok(env)
        }
    }
}

impl EmbedHyperlinks {
    fn deduce(matches: &MatchedFlags<'_>) -> Result<Self, OptionsError> {
        let flagged = matches.has(&flags::HYPERLINK)?;

        if flagged {
            Ok(Self::On)
        } else {
            Ok(Self::Off)
        }
    }
}

impl Absolute {
    fn deduce(matches: &MatchedFlags<'_>) -> Result<Self, OptionsError> {
        match matches.get(&flags::ABSOLUTE)? {
            Some(word) => match word.to_str() {
                Some("on" | "yes") => Ok(Self::On),
                Some("follow") => Ok(Self::Follow),
                Some("off" | "no") | None => Ok(Self::Off),
                _ => Err(OptionsError::BadArgument(&flags::ABSOLUTE, word.into())),
            },
            None => Ok(Self::Off),
        }
    }
}
