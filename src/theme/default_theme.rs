// SPDX-FileCopyrightText: 2024 Christina Sørensen
// SPDX-License-Identifier: EUPL-1.2
//
// SPDX-FileCopyrightText: 2023-2024 Christina Sørensen, eza contributors
// SPDX-FileCopyrightText: 2014 Benjamin Sago
// SPDX-License-Identifier: MIT
use nu_ansi_term::Color::*;
use nu_ansi_term::Style;
use std::default::Default;

use crate::output::color_scale::{ColorScaleMode, ColorScaleOptions};
use crate::theme::ui_styles::*;
impl UiStyles {
    pub fn default_theme(scale: ColorScaleOptions) -> Self {
        Self {
            size: Some(Size::colourful(scale)),
            ..Self::default()
        }
    }
}

impl Default for UiStyles {
    fn default() -> Self {
        Self {
            colourful: Some(true),

            #[rustfmt::skip]
            filekinds: Some(FileKinds {
            normal: Some(Style::default()),
            directory: Some(Blue.bold()),
            symlink: Some(Cyan.normal()),
            pipe: Some(Yellow.normal()),
            block_device: Some(Yellow.bold()),
            char_device: Some(Yellow.bold()),
            socket: Some(Red.bold()),
            special: Some(Yellow.normal()),
            executable: Some(Green.bold()),
            mount_point: Some(Blue.bold().underline()),
            btrfs_subvol: Some(Blue.underline()),
            }),

            #[rustfmt::skip]
            perms: Some(Permissions {
                user_read:           Some(Yellow.bold()),
                user_write:          Some(Red.bold()),
                user_execute_file:   Some(Green.bold().underline()),
                user_execute_other:  Some(Green.bold()),

                group_read:          Some(Yellow.normal()),
                group_write:         Some(Red.normal()),
                group_execute:       Some(Green.normal()),

                other_read:          Some(Yellow.normal()),
                other_write:         Some(Red.normal()),
                other_execute:       Some(Green.normal()),

                special_user_file:   Some(Purple.normal()),
                special_other:       Some(Purple.normal()),

                attribute:           Some(Style::default()),
            }),

            size: Some(Size::colourful(ColorScaleOptions::default())),

            #[rustfmt::skip]
            users:Some(Users {
                user_you:                       Some(Yellow.bold()),
                user_other:                     Some(Style::default()),
                user_root:                      Some(Style::default()),
                group_yours:                    Some(Yellow.bold()),
                group_other:                    Some(Style::default()),
                group_root:                     Some(Style::default()),
            }),

            #[rustfmt::skip]
            links: Some(Links {
                normal:          Some(Red.bold()),
                multi_link_file: Some(Red.on(Yellow)),
            }),

            #[rustfmt::skip]
            git: Some(Git {
                new:         Some(Green.normal()),
                modified:    Some(Blue.normal()),
                deleted:     Some(Red.normal()),
                renamed:     Some(Yellow.normal()),
                typechange:  Some(Purple.normal()),
                ignored:     Some(Style::default().dimmed()),
                conflicted:  Some(Red.normal()),
            }),

            git_repo: Some(GitRepo {
                branch_main: Some(Green.normal()),
                branch_other: Some(Yellow.normal()),
                git_clean: Some(Green.normal()),
                git_dirty: Some(Yellow.bold()),
            }),

            security_context: Some(SecurityContext {
                none: Some(Style::default()),
                #[rustfmt::skip]
                selinux: Some(SELinuxContext {
                    colon: Some(Style::default().dimmed()),
                    user:  Some(Blue.normal()),
                    role:  Some(Green.normal()),
                    typ:   Some(Yellow.normal()),
                    range: Some(Cyan.normal()),
                }),
            }),

            #[rustfmt::skip]
            file_type: Some(FileType {
                image:      Some(Purple.normal()),
                video:      Some(Purple.bold()),
                music:      Some(Cyan.normal()),
                lossless:   Some(Cyan.bold()),
                crypto:     Some(Green.bold()),
                document:   Some(Green.normal()),
                compressed: Some(Red.normal()),
                temp:       Some(White.normal()),
                compiled:   Some(Yellow.normal()),
                build:      Some(Yellow.bold().underline()),
                source:     Some(Yellow.bold()), // Need to discuss color
            }),

            punctuation: Some(DarkGray.bold()),
            date: Some(Blue.normal()),
            inode: Some(Purple.normal()),
            blocks: Some(Cyan.normal()),
            octal: Some(Purple.normal()),
            flags: Some(Style::default()),
            header: Some(Style::default().underline()),

            symlink_path: Some(Cyan.normal()),
            control_char: Some(Red.normal()),
            broken_symlink: Some(Red.normal()),
            broken_path_overlay: Some(Style::default().underline()),

            filenames: None,
            extensions: None,
        }
    }
}

impl Size {
    pub fn colourful(scale: ColorScaleOptions) -> Self {
        if scale.size && scale.mode == ColorScaleMode::Fixed {
            Self::colourful_fixed()
        } else {
            Self::colourful_gradient()
        }
    }

    fn colourful_fixed() -> Self {
        Self {
            major: Some(Green.bold()),
            minor: Some(Green.normal()),

            number_byte: Some(Green.bold()),
            number_kilo: Some(Green.bold()),
            number_mega: Some(Green.bold()),
            number_giga: Some(Green.bold()),
            number_huge: Some(Green.bold()),

            unit_byte: Some(Green.normal()),
            unit_kilo: Some(Green.normal()),
            unit_mega: Some(Green.normal()),
            unit_giga: Some(Green.normal()),
            unit_huge: Some(Green.normal()),
        }
    }

    fn colourful_gradient() -> Self {
        Self {
            major: Some(Green.bold()),
            minor: Some(Green.normal()),

            number_byte: Some(Green.normal()),
            number_kilo: Some(Green.bold()),
            number_mega: Some(Yellow.normal()),
            number_giga: Some(Red.normal()),
            number_huge: Some(Purple.normal()),

            unit_byte: Some(Green.normal()),
            unit_kilo: Some(Green.bold()),
            unit_mega: Some(Yellow.normal()),
            unit_giga: Some(Red.normal()),
            unit_huge: Some(Purple.normal()),
        }
    }
}
