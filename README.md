<!--
SPDX-FileCopyrightText: 2023-2024 Christina Sørensen
SPDX-FileContributor: Christina Sørensen

SPDX-License-Identifier: EUPL-1.2
-->
<div align="center">

# eza

A modern replacement for ls.

<a href="https://matrix.to/#/#eza-community:gitter.im"><img alt="Gitter" src="https://img.shields.io/gitter/room/eza-community/eza?logo=element&link=https%3A%2F%2Fapp.gitter.im%2F%23%2Froom%2F%23eza%3Agitter.im&link=Gitter%20matrix%20room%20for%20Eza" width=200></a>

[![Built with Nix](https://img.shields.io/badge/Built_With-Nix-5277C3.svg?logo=nixos&labelColor=73C3D5)](https://nixos.org)
[![Contributor Covenant](https://img.shields.io/badge/Contributor%20Covenant-2.1-4baaaa.svg)](CODE_OF_CONDUCT.md)

[![Unit tests](https://github.com/eza-community/eza/actions/workflows/unit-tests.yml/badge.svg)](https://github.com/eza-community/eza/actions/workflows/unit-tests.yml)
![Crates.io](https://img.shields.io/crates/v/eza?link=https%3A%2F%2Fcrates.io%2Fcrates%2Feza)
![Crates.io](https://img.shields.io/crates/l/eza?link=https%3A%2F%2Fgithub.com%2Feza-community%2Feza%2Fblob%2Fmain%2FLICENCE)

</div>

![eza demo gif](docs/images/screenshots.png)

---

**eza** is a modern alternative for the venerable file-listing command-line program `ls` that ships with Unix and Linux operating systems, giving it more features and better defaults.
It uses colours to distinguish file types and metadata.
It knows about symlinks, extended attributes, and Git.
And it’s **small**, **fast**, and just **one single binary**.

By deliberately making some decisions differently, eza attempts to be a more featureful, more user-friendly version of `ls`.

---

**eza** features not in exa (non-exhaustive):

- Fixes [“The Grid Bug”](https://github.com/eza-community/eza/issues/66#issuecomment-1656758327) introduced in exa 2021.
- Hyperlink support.
- Mount point details.
- Selinux context output.
- Git repo status output.
- Human readable relative dates.
- Several security fixes.
- Support for `bright` terminal colours.
- Many smaller bug fixes/changes!
- Configuration `theme.yml` file for customization of colors and icons.

...and like, so much more that it became exhausting to update this all the time.
Like seriously, we have a lot of good stuff.

---

<a id="try-it">
<h1>Try it!</h1>
</a>

### Nix ❄️

If you already have Nix setup with flake support, you can try out eza with the `nix run` command:

    nix run github:eza-community/eza

Nix will build eza and run it.

If you want to pass arguments this way, use e.g. `nix run github:eza-community/eza -- -ol`.

# Installation

eza is available for Windows, macOS and Linux. Platform and distribution
specific installation instructions can be found in [INSTALL.md](INSTALL.md).

[![Packaging status](https://repology.org/badge/vertical-allrepos/eza.svg?columns=3)](https://repology.org/project/eza/versions)

---

<a id="options">
<h1>Command-line options</h1>
</a>

eza’s options are almost, but not quite, entirely unlike `ls`’s. Quick overview:

## Display options

<details>
<summary>Click to expand</summary>

- **-1**, **--oneline**: display one entry per line
- **-G**, **--grid**: display entries as a grid (default)
- **-l**, **--long**: display extended details and attributes
- **-R**, **--recurse**: recurse into directories
- **-T**, **--tree**: recurse into directories as a tree
- **-x**, **--across**: sort the grid across, rather than downwards
- **-F**, **--classify=(when)**: display type indicator by file names (always, auto, never)
- **--colo[u]r=(when)**: when to use terminal colours (always, auto, never)
- **--colo[u]r-scale=(field)**: highlight levels of `field` distinctly(all, age, size)
- **--color-scale-mode=(mode)**: use gradient or fixed colors in --color-scale. valid options are `fixed` or `gradient`
- **--icons=(when)**: when to display icons (always, auto, never)
- **--hyperlink**: display entries as hyperlinks
- **--absolute=(mode)**: display entries with their absolute path (on, follow, off)
- **-w**, **--width=(columns)**: set screen width in columns

</details>

## Filtering options

<details>
<summary>Click to expand</summary>

- **-a**, **--all**: show hidden and 'dot' files
- **-d**, **--list-dirs**: list directories like regular files
- **-L**, **--level=(depth)**: limit the depth of recursion
- **-r**, **--reverse**: reverse the sort order
- **-s**, **--sort=(field)**: which field to sort by
- **--group-directories-first**: list directories before other files
- **--group-directories-last**: list directories after other files
- **-D**, **--only-dirs**: list only directories
- **-f**, **--only-files**: list only files
- **--no-symlinks**: don't show symbolic links
- **--show-symlinks**: explicitly show links (with `--only-dirs`, `--only-files`, to show symlinks that match the filter)
- **--git-ignore**: ignore files mentioned in `.gitignore`
- **--ignore-submodule-contents**: do not list contents of submodules
- **-I**, **--ignore-glob=(globs)**: glob patterns (pipe-separated) of files to ignore

Pass the `--all` option twice to also show the `.` and `..` directories.

</details>

## Long view options

<details>
<summary>Click to expand</summary>

These options are available when running with `--long` (`-l`):

- **-b**, **--binary**: list file sizes with binary prefixes
- **-B**, **--bytes**: list file sizes in bytes, without any prefixes
- **-g**, **--group**: list each file’s group
- **-h**, **--header**: add a header row to each column
- **-H**, **--links**: list each file’s number of hard links
- **-i**, **--inode**: list each file’s inode number
- **-m**, **--modified**: use the modified timestamp field
- **-M**, **--mounts**: Show mount details (Linux and MacOS only).
- **-S**, **--blocksize**: show size of allocated file system blocks
- **-t**, **--time=(field)**: which timestamp field to use
- **-u**, **--accessed**: use the accessed timestamp field
- **-U**, **--created**: use the created timestamp field
- **-X**, **--dereference**: dereference symlinks for file information
- **-Z**, **--context**: list each file’s security context
- **-@**, **--extended**: list each file’s extended attributes and sizes
- **--changed**: use the changed timestamp field
- **--git**: list each file’s Git status, if tracked or ignored
- **--git-repos**: list each directory’s Git status, if tracked
- **--git-repos-no-status**: list whether a directory is a Git repository, but not its status (faster)
- **--no-git**: suppress Git status (always overrides `--git`, `--git-repos`, `--git-repos-no-status`)
- **--time-style**: how to format timestamps. valid timestamp styles are ‘`default`’, ‘`iso`’, ‘`long-iso`’, ‘`full-iso`’, ‘`relative`’, or a custom style ‘`+<FORMAT>`’ (E.g., ‘`+%Y-%m-%d %H:%M`’ => ‘`2023-09-30 13:00`’. For more specifications on the format string, see the _`eza(1)` manual page_ and [chrono documentation](https://docs.rs/chrono/latest/chrono/format/strftime/index.html).).
- **--total-size**: show recursive directory size
- **--no-permissions**: suppress the permissions field
- **-o**, **--octal-permissions**: list each file's permission in octal format
- **--no-filesize**: suppress the filesize field
- **--no-user**: suppress the user field
- **--no-time**: suppress the time field
- **--stdin**: read file names from stdin

Some of the options accept parameters:

- Valid **--colo\[u\]r** options are **always**, **automatic** (or **auto** for short), and **never**.
- Valid sort fields are **accessed**, **changed**, **created**, **extension**, **Extension**, **inode**, **modified**, **name**, **Name**, **size**, **type**, and **none**. Fields starting with a capital letter sort uppercase before lowercase. The modified field has the aliases **date**, **time**, and **newest**, while its reverse has the aliases **age** and **oldest**.
- Valid time fields are **modified**, **changed**, **accessed**, and **created**.
- Valid time styles are **default**, **iso**, **long-iso**, **full-iso**, and **relative**.



See the `man` pages for further documentation of usage. They are available
- online [in the repo](https://github.com/eza-community/eza/tree/main/man)
- in your terminal via `man eza`, as of version [`[0.18.13] - 2024-04-25`](https://github.com/eza-community/eza/blob/main/CHANGELOG.md#01813---2024-04-25)
</details>


## Custom Themes
<details>
<summary>Click to expand</summary>

**Eza** has recently added support for a `theme.yml` file, where you can specify all of the existing theme-ing options
available for the `LS_COLORS` and `EXA_COLORS` environment variables, as well as the option to specify different icons
for different file types and extensions. Any existing environment variables set will continue to work and will take
precedence for backwards compatibility.

#### **New** Pre-made themes
Check out the themes available in the official [eza-themes](https://github.com/eza-community/eza-themes) repository, or contribute your own.

An example theme file is available in `docs/theme.yml`, and needs to either be placed in a directory specified by the 
environment variable `EZA_CONFIG_DIR`, or will looked for by default in `$XDG_CONFIG_HOME/eza`.

Full details are available on the [man page](https://github.com/eza-community/eza/tree/main/man/eza_colors-explanation.5.md) and an example theme file is included [here](https://github.com/eza-community/eza/tree/main/docs/theme.yml)

</details>


# Hacking on eza

If you wanna contribute to eza, firstly, you're expected to follow our 
[code of conduct](https://github.com/eza-community/eza/blob/main/CODE_OF_CONDUCT.md). 
After having understood the code of conduct, you can have a look at our
[CONTRIBUTING.md](https://github.com/eza-community/eza/blob/main/CONTRIBUTING.md) 
for more info about actual hacking.

[![Star History Chart](https://api.star-history.com/svg?repos=eza-community/eza&type=Date)](https://star-history.com/#eza-community/eza&Date)
