// SPDX-FileCopyrightText: 2024 Christina Sørensen
// SPDX-License-Identifier: EUPL-1.2
//
// SPDX-FileCopyrightText: 2023-2024 Christina Sørensen, eza contributors
// SPDX-FileCopyrightText: 2014 Benjamin Sago
// SPDX-License-Identifier: MIT
use crate::output::cell::TextCell;
use crate::output::time::TimeFormat;

use chrono::prelude::*;
use chrono_tz::Tz;
use nu_ansi_term::Style;

pub trait Render {
    fn render(self, style: Style, time_format: TimeFormat) -> TextCell;
}

impl Render for Option<NaiveDateTime> {
    fn render(self, style: Style, time_format: TimeFormat) -> TextCell {
        let datestamp = if let Ok(timezone_str) = iana_time_zone::get_timezone() {
            let timezone: Tz = timezone_str
                .parse()
                .unwrap_or_else(|_| panic!("The timezone cannot be parsed: {}", timezone_str));
            if let Some(time) = self {
                let time_offset = timezone.offset_from_utc_datetime(&time).fix();
                time_format.format(&DateTime::<FixedOffset>::from_naive_utc_and_offset(
                    time,
                    time_offset,
                ))
            } else {
                String::from("-")
            }
        } else if let Some(time) = self {
            // This is the next best thing, use the timezone now, instead of at the time of the
            // timestamp.
            let time_offset: FixedOffset = *Local::now().offset();
            time_format.format(&DateTime::<FixedOffset>::from_naive_utc_and_offset(
                time,
                time_offset,
            ))
        } else {
            String::from("-")
        };
        TextCell::paint(style, datestamp)
    }
}
