// SPDX-FileCopyrightText: 2024 Christina Sørensen
// SPDX-License-Identifier: EUPL-1.2
//
// SPDX-FileCopyrightText: 2023-2024 Christina Sørensen, eza contributors
// SPDX-FileCopyrightText: 2014 Benjamin Sago
// SPDX-License-Identifier: MIT
use log::trace;
use nu_ansi_term::{Color as Colour, Style};
use palette::{FromColor, LinSrgb, Oklab, Srgb};

use crate::{
    fs::{dir_action::RecurseOptions, feature::git::GitCache, fields::Size, DotFilter, File},
    output::{table::TimeType, tree::TreeDepth},
};

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub struct ColorScaleOptions {
    pub mode: ColorScaleMode,
    pub min_luminance: isize,
    pub size: bool,
    pub age: bool,
}

impl Default for ColorScaleOptions {
    fn default() -> Self {
        Self {
            mode: ColorScaleMode::Fixed,
            min_luminance: 50,
            size: false,
            age: false,
        }
    }
}

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum ColorScaleMode {
    Fixed,
    Gradient,
}

#[derive(Copy, Clone, Debug)]
pub struct ColorScaleInformation {
    pub options: ColorScaleOptions,

    pub accessed: Option<Extremes>,
    pub changed: Option<Extremes>,
    pub created: Option<Extremes>,
    pub modified: Option<Extremes>,

    pub size: Option<Extremes>,
}

impl ColorScaleInformation {
    pub fn from_color_scale(
        color_scale: ColorScaleOptions,
        files: &[File<'_>],
        dot_filter: DotFilter,
        git: Option<&GitCache>,
        git_ignoring: bool,
        ignoring_submodule_contents: bool,
        r: Option<RecurseOptions>,
    ) -> Option<Self> {
        if color_scale.mode == ColorScaleMode::Fixed {
            None
        } else {
            let mut information = Self {
                options: color_scale,
                accessed: None,
                changed: None,
                created: None,
                modified: None,
                size: None,
            };

            update_information_recursively(
                &mut information,
                files,
                dot_filter,
                git,
                git_ignoring,
                ignoring_submodule_contents,
                TreeDepth::root(),
                r,
            );

            Some(information)
        }
    }

    pub fn adjust_style(&self, mut style: Style, value: f32, range: Option<Extremes>) -> Style {
        if let (Some(fg), Some(range)) = (style.foreground, range) {
            let mut ratio = ((value - range.min) / (range.max - range.min)).clamp(0.0, 1.0);
            if ratio.is_nan() {
                ratio = 1.0;
            }

            style.foreground = Some(adjust_luminance(
                fg,
                ratio,
                self.options.min_luminance as f32 / 100.0,
            ));
        }

        style
    }

    pub fn apply_time_gradient(&self, style: Style, file: &File<'_>, time_type: TimeType) -> Style {
        let range = match time_type {
            TimeType::Modified => self.modified,
            TimeType::Changed => self.changed,
            TimeType::Accessed => self.accessed,
            TimeType::Created => self.created,
        };

        if let Some(file_time) = time_type.get_corresponding_time(file) {
            self.adjust_style(style, file_time.and_utc().timestamp_millis() as f32, range)
        } else {
            style
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn update_information_recursively(
    information: &mut ColorScaleInformation,
    files: &[File<'_>],
    dot_filter: DotFilter,
    git: Option<&GitCache>,
    git_ignoring: bool,
    ignoring_submodule_contents: bool,
    depth: TreeDepth,
    r: Option<RecurseOptions>,
) {
    for file in files {
        if information.options.age {
            Extremes::update(
                file.created_time()
                    .map(|x| x.and_utc().timestamp_millis() as f32),
                &mut information.created,
            );
            Extremes::update(
                file.modified_time()
                    .map(|x| x.and_utc().timestamp_millis() as f32),
                &mut information.modified,
            );
            Extremes::update(
                file.accessed_time()
                    .map(|x| x.and_utc().timestamp_millis() as f32),
                &mut information.accessed,
            );
            Extremes::update(
                file.changed_time()
                    .map(|x| x.and_utc().timestamp_millis() as f32),
                &mut information.changed,
            );
        }

        if information.options.size {
            let size = match file.size() {
                Size::Some(size) => Some(size as f32),
                _ => None,
            };
            Extremes::update(size, &mut information.size);
        }

        // We don't want to recurse into . and .., but still want to list them, therefore bypass
        // the dot_filter.
        if file.is_directory()
            && r.is_some_and(|x| !x.is_too_deep(depth.0))
            && file.name != "."
            && file.name != ".."
        {
            match file.to_dir() {
                Ok(dir) => {
                    if ignoring_submodule_contents
                        && git
                            .as_ref()
                            .map(|g| g.has_in_submodule(&file.path))
                            .unwrap_or(false)
                    {
                        continue;
                    }

                    let files: Vec<File<'_>> = dir
                        .files(dot_filter, git, git_ignoring, false, false)
                        .collect();

                    update_information_recursively(
                        information,
                        &files,
                        dot_filter,
                        git,
                        git_ignoring,
                        ignoring_submodule_contents,
                        depth.deeper(),
                        r,
                    );
                }
                Err(e) => trace!("Unable to access directory {}: {}", file.name, e),
            }
        };
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Extremes {
    max: f32,
    min: f32,
}

impl Extremes {
    fn update(maybe_value: Option<f32>, maybe_range: &mut Option<Extremes>) {
        match (maybe_value, maybe_range) {
            (Some(value), Some(range)) => {
                if value > range.max {
                    range.max = value;
                } else if value < range.min {
                    range.min = value;
                };
            }
            (Some(value), rel) => {
                let _ = rel.insert({
                    Extremes {
                        max: value,
                        min: value,
                    }
                });
            }
            _ => (),
        };
    }
}

fn adjust_luminance(color: Colour, x: f32, min_l: f32) -> Colour {
    let rgb_color = match color {
        Colour::Rgb(r, g, b) => LinSrgb::new(
            f32::from(r) / 255.0,
            f32::from(g) / 255.0,
            f32::from(b) / 255.0,
        ),

        Colour::Black => LinSrgb::new(0.0, 0.0, 0.0),

        Colour::Green | Colour::LightGreen => LinSrgb::new(0.0, 1.0, 0.0),

        Colour::Yellow | Colour::LightYellow => LinSrgb::new(1.0, 1.0, 0.0),

        Colour::Blue | Colour::LightBlue => LinSrgb::new(0.0, 0.0, 1.0),

        Colour::Magenta | Colour::LightMagenta => LinSrgb::new(1.0, 0.0, 1.0),

        Colour::Cyan | Colour::LightCyan => LinSrgb::new(0.0, 1.0, 1.0),

        Colour::White => LinSrgb::new(1.0, 1.0, 1.0),

        Colour::LightGray => LinSrgb::new(0.5, 0.5, 0.5),

        Colour::LightRed | Colour::Red => LinSrgb::new(1.0, 0.0, 0.0),

        Colour::DarkGray => LinSrgb::new(0.25, 0.25, 0.25),

        Colour::LightPurple | Colour::Purple => LinSrgb::new(0.5, 0.0, 0.5),

        _ => LinSrgb::new(1.0, 1.0, 1.0),
    };

    let mut lab: Oklab = Oklab::from_color(rgb_color);
    lab.l = (min_l + (1.0 - min_l) * (-4.0 * (1.0 - x)).exp()).clamp(0.0, 1.0);

    let adjusted_rgb: Srgb<f32> = Srgb::from_color(lab);
    Colour::Rgb(
        (adjusted_rgb.red * 255.0).round() as u8,
        (adjusted_rgb.green * 255.0).round() as u8,
        (adjusted_rgb.blue * 255.0).round() as u8,
    )
}
