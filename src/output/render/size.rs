// SPDX-FileCopyrightText: 2024 Christina Sørensen
// SPDX-License-Identifier: EUPL-1.2
//
// SPDX-FileCopyrightText: 2023-2024 Christina Sørensen, eza contributors
// SPDX-FileCopyrightText: 2014 Benjamin Sago
// SPDX-License-Identifier: MIT
use locale::Numeric as NumericLocale;
use nu_ansi_term::Style;
use number_prefix::Prefix;

use crate::fs::fields as f;
use crate::output::cell::{DisplayWidth, TextCell};
use crate::output::color_scale::{ColorScaleInformation, ColorScaleMode};
use crate::output::table::SizeFormat;

impl f::Size {
    pub fn render<C: Colours>(
        self,
        colours: &C,
        size_format: SizeFormat,
        numerics: &NumericLocale,
        color_scale_info: Option<ColorScaleInformation>,
    ) -> TextCell {
        use number_prefix::NumberPrefix;

        let size = match self {
            Self::Some(s) => s,
            Self::None => return TextCell::blank(colours.no_size()),
            Self::DeviceIDs(ref ids) => return ids.render(colours),
        };

        let is_gradient_mode =
            color_scale_info.is_some_and(|csi| csi.options.mode == ColorScaleMode::Gradient);

        #[rustfmt::skip]
        let result = match size_format {
            SizeFormat::DecimalBytes  => NumberPrefix::decimal(size as f64),
            SizeFormat::BinaryBytes   => NumberPrefix::binary(size as f64),
            SizeFormat::JustBytes     => {
                // Use the binary prefix to select a style.
                let prefix = match NumberPrefix::binary(size as f64) {
                    NumberPrefix::Standalone(_) => None,
                    NumberPrefix::Prefixed(p, _) => Some(p),
                };

                // But format the number directly using the locale.
                let string = numerics.format_int(size);

                return if is_gradient_mode {
                    let csi = color_scale_info.unwrap();
                    TextCell::paint(
                        csi.adjust_style(colours.size(prefix), size as f32, csi.size),
                        string,
                    )
                } else {
                    TextCell::paint(colours.size(prefix), string)
                }
            }
        };

        #[rustfmt::skip]
        let (prefix, n) = match result {
            NumberPrefix::Standalone(b) => {
                return if is_gradient_mode {
                    let csi = color_scale_info.unwrap();
                    TextCell::paint(
                        csi.adjust_style(colours.size(None), size as f32, csi.size),
                        numerics.format_int(b),
                    )
                } else {
                    TextCell::paint(colours.size(None), numerics.format_int(b))
                }
            }
            NumberPrefix::Prefixed(p, n)  => (p, n),
        };

        let symbol = prefix.symbol();
        let number = if n < 10_f64 {
            numerics.format_float(n, 1)
        } else {
            numerics.format_int(n.round() as isize)
        };

        TextCell {
            // symbol is guaranteed to be ASCII since unit prefixes are hardcoded.
            width: DisplayWidth::from(&*number) + symbol.len(),
            contents: if is_gradient_mode {
                let csi = color_scale_info.unwrap();
                vec![
                    csi.adjust_style(colours.size(Some(prefix)), size as f32, csi.size)
                        .paint(number),
                    csi.adjust_style(colours.unit(Some(prefix)), size as f32, csi.size)
                        .paint(symbol),
                ]
            } else {
                vec![
                    colours.size(Some(prefix)).paint(number),
                    colours.unit(Some(prefix)).paint(symbol),
                ]
            }
            .into(),
        }
    }
}

impl f::DeviceIDs {
    fn render<C: Colours>(self, colours: &C) -> TextCell {
        let major = self.major.to_string();
        let minor = self.minor.to_string();

        TextCell {
            width: DisplayWidth::from(major.len() + 1 + minor.len()),
            contents: vec![
                colours.major().paint(major),
                colours.comma().paint(","),
                colours.minor().paint(minor),
            ]
            .into(),
        }
    }
}

pub trait Colours {
    fn size(&self, prefix: Option<Prefix>) -> Style;
    fn unit(&self, prefix: Option<Prefix>) -> Style;
    fn no_size(&self) -> Style;

    fn major(&self) -> Style;
    fn comma(&self) -> Style;
    fn minor(&self) -> Style;
}

#[cfg(test)]
pub mod test {
    use super::Colours;
    use crate::fs::fields as f;
    use crate::output::cell::{DisplayWidth, TextCell};
    use crate::output::table::SizeFormat;

    use locale::Numeric as NumericLocale;
    use nu_ansi_term::Color::*;
    use nu_ansi_term::Style;
    use number_prefix::Prefix;

    struct TestColours;

    #[rustfmt::skip]
    impl Colours for TestColours {
        fn size(&self, _prefix: Option<Prefix>) -> Style { Fixed(66).normal() }
        fn unit(&self, _prefix: Option<Prefix>) -> Style { Fixed(77).bold() }
        fn no_size(&self)                       -> Style { Black.italic() }

        fn major(&self) -> Style { Blue.on(Red) }
        fn comma(&self) -> Style { Green.italic() }
        fn minor(&self) -> Style { Cyan.on(Yellow) }
    }

    #[test]
    fn directory() {
        let directory = f::Size::None;
        let expected = TextCell::blank(Black.italic());
        assert_eq!(
            expected,
            directory.render(
                &TestColours,
                SizeFormat::JustBytes,
                &NumericLocale::english(),
                None
            )
        );
    }

    #[test]
    fn file_decimal() {
        let directory = f::Size::Some(2_100_000);
        let expected = TextCell {
            width: DisplayWidth::from(4),
            contents: vec![Fixed(66).paint("2.1"), Fixed(77).bold().paint("M")].into(),
        };

        assert_eq!(
            expected,
            directory.render(
                &TestColours,
                SizeFormat::DecimalBytes,
                &NumericLocale::english(),
                None
            )
        );
    }

    #[test]
    fn file_binary() {
        let directory = f::Size::Some(1_048_576);
        let expected = TextCell {
            width: DisplayWidth::from(5),
            contents: vec![Fixed(66).paint("1.0"), Fixed(77).bold().paint("Mi")].into(),
        };

        assert_eq!(
            expected,
            directory.render(
                &TestColours,
                SizeFormat::BinaryBytes,
                &NumericLocale::english(),
                None
            )
        );
    }

    #[test]
    fn file_bytes() {
        let directory = f::Size::Some(1_048_576);
        let expected = TextCell {
            width: DisplayWidth::from(9),
            contents: vec![Fixed(66).paint("1,048,576")].into(),
        };

        assert_eq!(
            expected,
            directory.render(
                &TestColours,
                SizeFormat::JustBytes,
                &NumericLocale::english(),
                None
            )
        );
    }

    #[test]
    fn device_ids() {
        let directory = f::Size::DeviceIDs(f::DeviceIDs {
            major: 10,
            minor: 80,
        });
        let expected = TextCell {
            width: DisplayWidth::from(5),
            contents: vec![
                Blue.on(Red).paint("10"),
                Green.italic().paint(","),
                Cyan.on(Yellow).paint("80"),
            ]
            .into(),
        };

        assert_eq!(
            expected,
            directory.render(
                &TestColours,
                SizeFormat::JustBytes,
                &NumericLocale::english(),
                None
            )
        );
    }
}
