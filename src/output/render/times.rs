use std::sync::OnceLock;

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

// Assume that the timezone is constant for the duration of the program.
static INITIAL_TIMEZONE: OnceLock<Option<Tz>> = OnceLock::new();

fn initialize_timezone() -> Option<Tz> {
    let timezone_str = iana_time_zone::get_timezone();
    timezone_str.map_or(None, |tz_str| {
        Some(
            tz_str
                .parse()
                .unwrap_or_else(|_| panic!("The timezone cannot be parsed: {tz_str}")),
        )
    })
}

impl Render for Option<NaiveDateTime> {
    fn render(self, style: Style, time_format: TimeFormat) -> TextCell {
        let datestamp = if let Some(timezone) = INITIAL_TIMEZONE.get_or_init(initialize_timezone) {
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
