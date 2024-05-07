use nu_ansi_term::Color::*;
use nu_ansi_term::Style;
use std::default::Default;

use crate::output::color_scale::{ColorScaleMode, ColorScaleOptions};
use crate::theme::ui_styles::*;

impl UiStyles {
    pub fn default_theme(scale: ColorScaleOptions) -> Self {
        Self {
            colourful: true,

            #[rustfmt::skip]
            filekinds: FileKinds {
                normal:       Style::default(),
                directory:    Blue.bold(),
                symlink:      Cyan.normal(),
                pipe:         Yellow.normal(),
                block_device: Yellow.bold(),
                char_device:  Yellow.bold(),
                socket:       Red.bold(),
                special:      Yellow.normal(),
                executable:   Green.bold(),
                mount_point:  Blue.bold().underline(),
            },

            #[rustfmt::skip]
            perms: Permissions {
                user_read:           Yellow.bold(),
                user_write:          Red.bold(),
                user_execute_file:   Green.bold().underline(),
                user_execute_other:  Green.bold(),

                group_read:          Yellow.normal(),
                group_write:         Red.normal(),
                group_execute:       Green.normal(),

                other_read:          Yellow.normal(),
                other_write:         Red.normal(),
                other_execute:       Green.normal(),

                special_user_file:   Purple.normal(),
                special_other:       Purple.normal(),

                attribute:           Style::default(),
            },

            size: Size::colourful(scale),

            #[rustfmt::skip]
            users: Users {
                user_you:                       Yellow.bold(),
                user_other:                     Style::default(),
                user_root:                      Style::default(),
                group_yours:                    Yellow.bold(),
                group_other:                    Style::default(),
                group_root:                     Style::default(),
            },

            #[rustfmt::skip]
            links: Links {
                normal:          Red.bold(),
                multi_link_file: Red.on(Yellow),
            },

            #[rustfmt::skip]
            git: Git {
                new:         Green.normal(),
                modified:    Blue.normal(),
                deleted:     Red.normal(),
                renamed:     Yellow.normal(),
                typechange:  Purple.normal(),
                ignored:     Style::default().dimmed(),
                conflicted:  Red.normal(),
            },

            git_repo: GitRepo {
                branch_main: Green.normal(),
                branch_other: Yellow.normal(),
                git_clean: Green.normal(),
                git_dirty: Yellow.bold(),
            },

            security_context: SecurityContext {
                none: Style::default(),
                #[rustfmt::skip]
                selinux: SELinuxContext {
                    colon: Style::default().dimmed(),
                    user:  Blue.normal(),
                    role:  Green.normal(),
                    typ:   Yellow.normal(),
                    range: Cyan.normal(),
                },
            },

            #[rustfmt::skip]
            file_type: FileType {
                image:      Purple.normal(),
                video:      Purple.bold(),
                music:      Cyan.normal(),
                lossless:   Cyan.bold(),
                crypto:     Green.bold(),
                document:   Green.normal(),
                compressed: Red.normal(),
                temp:       White.normal(),
                compiled:   Yellow.normal(),
                build:      Yellow.bold().underline(),
                source:     Yellow.bold(), // Need to discuss color
            },

            punctuation: DarkGray.bold(),
            date: Blue.normal(),
            inode: Purple.normal(),
            blocks: Cyan.normal(),
            octal: Purple.normal(),
            flags: Style::default(),
            header: Style::default().underline(),

            symlink_path: Cyan.normal(),
            control_char: Red.normal(),
            broken_symlink: Red.normal(),
            broken_path_overlay: Style::default().underline(),
        }
    }
}

impl Size {
    pub fn colourful(scale: ColorScaleOptions) -> Self {
        if scale.mode == ColorScaleMode::Fixed {
            Self::colourful_fixed()
        } else {
            Self::colourful_gradient()
        }
    }

    fn colourful_fixed() -> Self {
        Self {
            major: Green.bold(),
            minor: Green.normal(),

            number_byte: Green.bold(),
            number_kilo: Green.bold(),
            number_mega: Green.bold(),
            number_giga: Green.bold(),
            number_huge: Green.bold(),

            unit_byte: Green.normal(),
            unit_kilo: Green.normal(),
            unit_mega: Green.normal(),
            unit_giga: Green.normal(),
            unit_huge: Green.normal(),
        }
    }

    fn colourful_gradient() -> Self {
        Self {
            major: Green.bold(),
            minor: Green.normal(),

            number_byte: Green.normal(),
            number_kilo: Green.bold(),
            number_mega: Yellow.normal(),
            number_giga: Red.normal(),
            number_huge: Purple.normal(),

            unit_byte: Green.normal(),
            unit_kilo: Green.bold(),
            unit_mega: Yellow.normal(),
            unit_giga: Red.normal(),
            unit_huge: Purple.normal(),
        }
    }
}
