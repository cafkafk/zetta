// SPDX-FileCopyrightText: 2024 Christina Sørensen
// SPDX-License-Identifier: EUPL-1.2
//
// SPDX-FileCopyrightText: 2023-2024 Christina Sørensen, eza contributors
// SPDX-FileCopyrightText: 2014 Benjamin Sago
// SPDX-License-Identifier: MIT
//! Tests for various types of file (video, image, compressed, etc).
//!
//! Currently this is dependent on the file’s name and extension, because
//! those are the only metadata that we have access to without reading the
//! file’s contents.
//!
//! # Contributors
//! Please keep these lists sorted. If you're using vim, :sort i

use phf::{phf_map, Map};

use crate::fs::File;

#[derive(Debug, Clone)]
pub enum FileType {
    Image,
    Video,
    Music,
    Lossless, // Lossless music, rather than any other kind of data...
    Crypto,
    Document,
    Compressed,
    Temp,
    Compiled,
    Build, // A “build file is something that can be run or activated somehow in order to
    // kick off the build of a project. It’s usually only present in directories full of
    // source code.
    Source,
}

/// Mapping from full filenames to file type.
const FILENAME_TYPES: Map<&'static str, FileType> = phf_map! {
    /* Immediate file - kick off the build of a project */
    "Android.bp"         => FileType::Build,
    "Brewfile"           => FileType::Build,
    "bsconfig.json"      => FileType::Build,
    "BUILD"              => FileType::Build,
    "BUILD.bazel"        => FileType::Build,
    "build.gradle"       => FileType::Build,
    "build.sbt"          => FileType::Build,
    "build.xml"          => FileType::Build,
    "Cargo.toml"         => FileType::Build,
    "CMakeLists.txt"     => FileType::Build,
    "composer.json"      => FileType::Build,
    "configure"          => FileType::Build,
    "Containerfile"      => FileType::Build,
    "Dockerfile"         => FileType::Build,
    "Earthfile"          => FileType::Build,
    "flake.nix"          => FileType::Build,
    "Gemfile"            => FileType::Build,
    "GNUmakefile"        => FileType::Build,
    "Gruntfile.coffee"   => FileType::Build,
    "Gruntfile.js"       => FileType::Build,
    "jsconfig.json"      => FileType::Build,
    "Justfile"           => FileType::Build,
    "justfile"           => FileType::Build,
    "Makefile"           => FileType::Build,
    "makefile"           => FileType::Build,
    "meson.build"        => FileType::Build,
    "mix.exs"            => FileType::Build,
    "package.json"       => FileType::Build,
    "Pipfile"            => FileType::Build,
    "PKGBUILD"           => FileType::Build,
    "Podfile"            => FileType::Build,
    "pom.xml"            => FileType::Build,
    "Procfile"           => FileType::Build,
    "pyproject.toml"     => FileType::Build,
    "Rakefile"           => FileType::Build,
    "RoboFile.php"       => FileType::Build,
    "SConstruct"         => FileType::Build,
    "tsconfig.json"      => FileType::Build,
    "Vagrantfile"        => FileType::Build,
    "webpack.config.cjs" => FileType::Build,
    "webpack.config.js"  => FileType::Build,
    "WORKSPACE"          => FileType::Build,
    /* Cryptology files */
    "id_dsa"             => FileType::Crypto,
    "id_ecdsa"           => FileType::Crypto,
    "id_ecdsa_sk"        => FileType::Crypto,
    "id_ed25519"         => FileType::Crypto,
    "id_ed25519_sk"      => FileType::Crypto,
    "id_rsa"             => FileType::Crypto,
};

/// Mapping from lowercase file extension to file type.  If an image, video, music, or lossless
/// extension is added also update the extension icon map.
const EXTENSION_TYPES: Map<&'static str, FileType> = phf_map! {
    /* Immediate file - kick off the build of a project */
    "ninja"      => FileType::Build,
    /* Image files */
    "arw"        => FileType::Image,
    "avif"       => FileType::Image,
    "bmp"        => FileType::Image,
    "cbr"        => FileType::Image,
    "cbz"        => FileType::Image,
    "cr2"        => FileType::Image,
    "dvi"        => FileType::Image,
    "eps"        => FileType::Image,
    "fodg"       => FileType::Image,
    "gif"        => FileType::Image,
    "heic"       => FileType::Image,
    "heif"       => FileType::Image,
    "ico"        => FileType::Image,
    "j2c"        => FileType::Image,
    "j2k"        => FileType::Image,
    "jfi"        => FileType::Image,
    "jfif"       => FileType::Image,
    "jif"        => FileType::Image,
    "jp2"        => FileType::Image,
    "jpe"        => FileType::Image,
    "jpeg"       => FileType::Image,
    "jpf"        => FileType::Image,
    "jpg"        => FileType::Image,
    "jpx"        => FileType::Image,
    "jxl"        => FileType::Image,
    "kra"        => FileType::Image,
    "krz"        => FileType::Image,
    "nef"        => FileType::Image,
    "odg"        => FileType::Image,
    "orf"        => FileType::Image,
    "pbm"        => FileType::Image,
    "pgm"        => FileType::Image,
    "png"        => FileType::Image,
    "pnm"        => FileType::Image,
    "ppm"        => FileType::Image,
    "ps"         => FileType::Image,
    "psd"        => FileType::Image,
    "pxm"        => FileType::Image,
    "raw"        => FileType::Image,
    "qoi"        => FileType::Image,
    "svg"        => FileType::Image,
    "tif"        => FileType::Image,
    "tiff"       => FileType::Image,
    "webp"       => FileType::Image,
    "xcf"        => FileType::Image,
    "xpm"        => FileType::Image,
    /* Video files */
    "avi"        => FileType::Video,
    "flv"        => FileType::Video,
    "h264"       => FileType::Video,
    "heics"      => FileType::Video,
    "m2ts"       => FileType::Video,
    "m2v"        => FileType::Video,
    "m4v"        => FileType::Video,
    "mkv"        => FileType::Video,
    "mov"        => FileType::Video,
    "mp4"        => FileType::Video,
    "mpeg"       => FileType::Video,
    "mpg"        => FileType::Video,
    "ogm"        => FileType::Video,
    "ogv"        => FileType::Video,
    "video"      => FileType::Video,
    "vob"        => FileType::Video,
    "webm"       => FileType::Video,
    "wmv"        => FileType::Video,
    /* Music files */
    "aac"        => FileType::Music, // Advanced Audio Coding
    "m4a"        => FileType::Music,
    "mka"        => FileType::Music,
    "mp2"        => FileType::Music,
    "mp3"        => FileType::Music,
    "ogg"        => FileType::Music,
    "opus"       => FileType::Music,
    "wma"        => FileType::Music,
    /* Lossless music, rather than any other kind of data... */
    "aif"        => FileType::Lossless,
    "aifc"       => FileType::Lossless,
    "aiff"       => FileType::Lossless,
    "alac"       => FileType::Lossless,
    "ape"        => FileType::Lossless,
    "flac"       => FileType::Lossless,
    "pcm"        => FileType::Lossless,
    "wav"        => FileType::Lossless,
    "wv"         => FileType::Lossless,
    /* Cryptology files */
    "age"        => FileType::Crypto, // age encrypted file
    "asc"        => FileType::Crypto, // GnuPG ASCII armored file
    "cer"        => FileType::Crypto,
    "crt"        => FileType::Crypto,
    "csr"        => FileType::Crypto, // PKCS#10 Certificate Signing Request
    "gpg"        => FileType::Crypto, // GnuPG encrypted file
    "kbx"        => FileType::Crypto, // GnuPG keybox
    "md5"        => FileType::Crypto, // MD5 checksum
    "p12"        => FileType::Crypto, // PKCS#12 certificate (Netscape)
    "pem"        => FileType::Crypto, // Privacy-Enhanced Mail certificate
    "pfx"        => FileType::Crypto, // PKCS#12 certificate (Microsoft)
    "pgp"        => FileType::Crypto, // PGP security key
    "pub"        => FileType::Crypto, // Public key
    "sha1"       => FileType::Crypto, // SHA-1 hash
    "sha224"     => FileType::Crypto, // SHA-224 hash
    "sha256"     => FileType::Crypto, // SHA-256 hash
    "sha384"     => FileType::Crypto, // SHA-384 hash
    "sha512"     => FileType::Crypto, // SHA-512 hash
    "sig"        => FileType::Crypto, // GnuPG signed file
    "signature"  => FileType::Crypto, // e-Filing Digital Signature File (India)
    /* Document files */
    "djvu"       => FileType::Document,
    "doc"        => FileType::Document,
    "docx"       => FileType::Document,
    "eml"        => FileType::Document,
    "fodp"       => FileType::Document,
    "fods"       => FileType::Document,
    "fodt"       => FileType::Document,
    "fotd"       => FileType::Document,
    "gdoc"       => FileType::Document,
    "key"        => FileType::Document,
    "keynote"    => FileType::Document,
    "numbers"    => FileType::Document,
    "odp"        => FileType::Document,
    "ods"        => FileType::Document,
    "odt"        => FileType::Document,
    "pages"      => FileType::Document,
    "pdf"        => FileType::Document,
    "ppt"        => FileType::Document,
    "pptx"       => FileType::Document,
    "rtf"        => FileType::Document, // Rich Text Format
    "xls"        => FileType::Document,
    "xlsm"       => FileType::Document,
    "xlsx"       => FileType::Document,
    /* Compressed/archive files */
    "7z"         => FileType::Compressed, // 7-Zip
    "ar"         => FileType::Compressed,
    "arj"        => FileType::Compressed,
    "br"         => FileType::Compressed, // Brotli
    "bz"         => FileType::Compressed, // bzip
    "bz2"        => FileType::Compressed, // bzip2
    "bz3"        => FileType::Compressed, // bzip3
    "cpio"       => FileType::Compressed,
    "deb"        => FileType::Compressed, // Debian
    "dmg"        => FileType::Compressed,
    "gz"         => FileType::Compressed, // gzip
    "iso"        => FileType::Compressed,
    "lz"         => FileType::Compressed,
    "lz4"        => FileType::Compressed,
    "lzh"        => FileType::Compressed,
    "lzma"       => FileType::Compressed,
    "lzo"        => FileType::Compressed,
    "phar"       => FileType::Compressed, // PHP PHAR
    "qcow"       => FileType::Compressed,
    "qcow2"      => FileType::Compressed,
    "rar"        => FileType::Compressed,
    "rpm"        => FileType::Compressed,
    "tar"        => FileType::Compressed,
    "taz"        => FileType::Compressed,
    "tbz"        => FileType::Compressed,
    "tbz2"       => FileType::Compressed,
    "tc"         => FileType::Compressed,
    "tgz"        => FileType::Compressed,
    "tlz"        => FileType::Compressed,
    "txz"        => FileType::Compressed,
    "tz"         => FileType::Compressed,
    "xz"         => FileType::Compressed,
    "vdi"        => FileType::Compressed,
    "vhd"        => FileType::Compressed,
    "vhdx"       => FileType::Compressed,
    "vmdk"       => FileType::Compressed,
    "z"          => FileType::Compressed,
    "zip"        => FileType::Compressed,
    "zst"        => FileType::Compressed, // Zstandard
    /* Temporary files */
    "bak"        => FileType::Temp,
    "bk"         => FileType::Temp,
    "bkp"        => FileType::Temp,
    "crdownload" => FileType::Temp,
    "download"   => FileType::Temp,
    "fcbak"      => FileType::Temp,
    "fcstd1"     => FileType::Temp,
    "fdmdownload"=> FileType::Temp,
    "part"       => FileType::Temp,
    "swn"        => FileType::Temp,
    "swo"        => FileType::Temp,
    "swp"        => FileType::Temp,
    "tmp"        => FileType::Temp,
    /* Compiler output files */
    "a"          => FileType::Compiled, // Unix static library
    "aab"        => FileType::Compiled, // Android App Bundle
    "aar"        => FileType::Compiled, // Android Archive
    "apk"        => FileType::Compiled, // Android PacKage
    "bundle"     => FileType::Compiled, // macOS application bundle
    "class"      => FileType::Compiled, // Java class file
    "cma"        => FileType::Compiled, // OCaml bytecode library
    "cmi"        => FileType::Compiled, // OCaml interface
    "cmo"        => FileType::Compiled, // OCaml bytecode object
    "cmx"        => FileType::Compiled, // OCaml bytecode object for inlining
    "dex"        => FileType::Compiled, // Dalvik bytecode object, for Android apps
    "dll"        => FileType::Compiled, // Windows dynamic link library
    "dylib"      => FileType::Compiled, // Mach-O dynamic library
    "elc"        => FileType::Compiled, // Emacs compiled lisp
    "elf"        => FileType::Compiled, // Executable and Linkable Format
    "ko"         => FileType::Compiled, // Linux kernel module
    "lib"        => FileType::Compiled, // Windows static library
    "o"          => FileType::Compiled, // Compiled object file
    "obj"        => FileType::Compiled, // Compiled object file
    "odex"       => FileType::Compiled, // Optimized DEX bytecode, for Android apps
    "pyc"        => FileType::Compiled, // Python compiled code
    "pyd"        => FileType::Compiled, // Python dynamic module
    "pyo"        => FileType::Compiled, // Python optimized code
    "so"         => FileType::Compiled, // Unix shared library
    "zwc"        => FileType::Compiled, // zsh compiled file
    /* Source code files */
    "aidl"       => FileType::Source, // Android Interface Definition Language
    "applescript"=> FileType::Source, // Apple script
    "as"         => FileType::Source, // Action script
    "asa"        => FileType::Source, // asp
    "awk"        => FileType::Source, // awk
    "c"          => FileType::Source, // C/C++
    "c++"        => FileType::Source, // C/C++
    "c++m"       => FileType::Source, // C/C++ module
    "cabal"      => FileType::Source, // Cabal
    "cc"         => FileType::Source, // C/C++
    "ccm"        => FileType::Source, // C/C++ module
    "clj"        => FileType::Source, // Clojure
    "cp"         => FileType::Source, // C/C++ Xcode
    "cpp"        => FileType::Source, // C/C++
    "cppm"       => FileType::Source, // C/C++ module
    "cr"         => FileType::Source, // Crystal
    "cs"         => FileType::Source, // C#
    "css"        => FileType::Source, // css
    "csx"        => FileType::Source, // C#
    "cu"         => FileType::Source, // CUDA
    "cxx"        => FileType::Source, // C/C++
    "cxxm"       => FileType::Source, // C/C++ module
    "cypher"     => FileType::Source, // Cypher (query language)
    "d"          => FileType::Source, // D
    "dart"       => FileType::Source, // Dart
    "di"         => FileType::Source, // D
    "dpr"        => FileType::Source, // Delphi Pascal
    "el"         => FileType::Source, // Lisp
    "elm"        => FileType::Source, // Elm
    "erl"        => FileType::Source, // Erlang
    "ex"         => FileType::Source, // Elixir
    "exs"        => FileType::Source, // Elixir
    "f"          => FileType::Source, // Fortran
    "f90"        => FileType::Source, // Fortran
    "fcmacro"    => FileType::Source, // FreeCAD macro
    "fcscript"   => FileType::Source, // FreeCAD script
    "fnl"        => FileType::Source, // Fennel
    "for"        => FileType::Source, // Fortran
    "fs"         => FileType::Source, // F#
    "fsh"        => FileType::Source, // Fragment shader
    "fsi"        => FileType::Source, // F#
    "fsx"        => FileType::Source, // F#
    "gd"         => FileType::Source, // GDScript
    "go"         => FileType::Source, // Go
    "gradle"     => FileType::Source, // Gradle
    "groovy"     => FileType::Source, // Groovy
    "gvy"        => FileType::Source, // Groovy
    "h"          => FileType::Source, // C/C++ header
    "h++"        => FileType::Source, // C/C++ header
    "hh"         => FileType::Source, // C/C++ header
    "hidl"       => FileType::Source, // Android Hardware Interface Definition
    "hpp"        => FileType::Source, // C/C++ header
    "hc"         => FileType::Source, // HolyC
    "hs"         => FileType::Source, // Haskell
    "htc"        => FileType::Source, // JavaScript
    "hxx"        => FileType::Source, // C/C++ header
    "inc"        => FileType::Source,
    "inl"        => FileType::Source, // C/C++ Microsoft
    "ino"        => FileType::Source, // Arduino
    "ipynb"      => FileType::Source, // Jupyter Notebook
    "ixx"        => FileType::Source, // C/C++ module
    "java"       => FileType::Source, // Java
    "jl"         => FileType::Source, // Julia
    "js"         => FileType::Source, // JavaScript
    "jsx"        => FileType::Source, // React
    "kt"         => FileType::Source, // Kotlin
    "kts"        => FileType::Source, // Kotlin
    "kusto"      => FileType::Source, // Kusto (query language)
    "less"       => FileType::Source, // less
    "lhs"        => FileType::Source, // Haskell
    "lisp"       => FileType::Source, // Lisp
    "ltx"        => FileType::Source, // LaTeX
    "lua"        => FileType::Source, // Lua
    "m"          => FileType::Source, // Matlab
    "malloy"     => FileType::Source, // Malloy (query language)
    "matlab"     => FileType::Source, // Matlab
    "ml"         => FileType::Source, // OCaml
    "mli"        => FileType::Source, // OCaml
    "mn"         => FileType::Source, // Matlab
    "nb"         => FileType::Source, // Mathematica
    "p"          => FileType::Source, // Pascal
    "pas"        => FileType::Source, // Pascal
    "php"        => FileType::Source, // PHP
    "pl"         => FileType::Source, // Perl
    "pm"         => FileType::Source, // Perl
    "pod"        => FileType::Source, // Perl
    "pp"         => FileType::Source, // Puppet
    "prql"       => FileType::Source, // PRQL
    "ps1"        => FileType::Source, // PowerShell
    "psd1"       => FileType::Source, // PowerShell
    "psm1"       => FileType::Source, // PowerShell
    "purs"       => FileType::Source, // PureScript
    "py"         => FileType::Source, // Python
    "r"          => FileType::Source, // R
    "rb"         => FileType::Source, // Ruby
    "rs"         => FileType::Source, // Rust
    "rq"         => FileType::Source, // SPARQL (query language)
    "sass"       => FileType::Source, // Sass
    "scala"      => FileType::Source, // Scala
    "scm"        => FileType::Source, // Scheme
    "scad"       => FileType::Source, // OpenSCAD
    "scss"       => FileType::Source, // Sass
    "sld"        => FileType::Source, // Scheme Library Definition
    "sql"        => FileType::Source, // SQL
    "ss"         => FileType::Source, // Scheme Source
    "swift"      => FileType::Source, // Swift
    "tcl"        => FileType::Source, // TCL
    "tex"        => FileType::Source, // LaTeX
    "ts"         => FileType::Source, // TypeScript
    "v"          => FileType::Source, // V
    "vb"         => FileType::Source, // Visual Basic
    "vsh"        => FileType::Source, // Vertex shader
    "xml"        => FileType::Source, // eXtensible Markup Language
    "zig"        => FileType::Source, // Zig
};

impl FileType {
    /// Lookup the file type based on the file's name, by the file name
    /// lowercase extension, or if the file could be compiled from related
    /// source code.
    pub(crate) fn get_file_type(file: &File<'_>) -> Option<FileType> {
        // Case-insensitive readme is checked first for backwards compatibility.
        if file.name.to_lowercase().starts_with("readme") {
            return Some(Self::Build);
        }
        if let Some(file_type) = FILENAME_TYPES.get(&file.name) {
            return Some(file_type.clone());
        }
        if let Some(file_type) = file.ext.as_ref().and_then(|ext| EXTENSION_TYPES.get(ext)) {
            return Some(file_type.clone());
        }
        if file.name.ends_with('~') || (file.name.starts_with('#') && file.name.ends_with('#')) {
            return Some(Self::Temp);
        }
        if let Some(dir) = file.parent_dir {
            if file
                .get_source_files()
                .iter()
                .any(|path| dir.contains(path))
            {
                return Some(Self::Compiled);
            }
        }
        None
    }
}
