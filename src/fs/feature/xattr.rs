// SPDX-FileCopyrightText: 2024 Christina Sørensen
// SPDX-License-Identifier: EUPL-1.2
//
// SPDX-FileCopyrightText: 2023-2024 Christina Sørensen, eza contributors
// SPDX-FileCopyrightText: 2014 Benjamin Sago
// SPDX-License-Identifier: MIT
//! Extended attribute support for `NetBSD`, `Darwin`, and `Linux` systems.

#![allow(trivial_casts)] // for ARM

use std::fmt::{Display, Formatter};
use std::io;
use std::path::Path;
use std::str;

pub const ENABLED: bool = cfg!(any(
    target_os = "macos",
    target_os = "linux",
    target_os = "netbsd",
    target_os = "freebsd"
));

#[derive(Debug)]
pub struct Attribute {
    pub name: String,
    pub value: Option<Vec<u8>>,
}

pub trait FileAttributes {
    fn attributes(&self) -> io::Result<Vec<Attribute>>;
    fn symlink_attributes(&self) -> io::Result<Vec<Attribute>>;
}

#[cfg(any(
    target_os = "macos",
    target_os = "linux",
    target_os = "netbsd",
    target_os = "freebsd"
))]
impl FileAttributes for Path {
    fn attributes(&self) -> io::Result<Vec<Attribute>> {
        extended_attrs::attributes(self, true)
    }

    fn symlink_attributes(&self) -> io::Result<Vec<Attribute>> {
        extended_attrs::attributes(self, false)
    }
}

#[cfg(not(any(
    target_os = "macos",
    target_os = "linux",
    target_os = "netbsd",
    target_os = "freebsd"
)))]
impl FileAttributes for Path {
    fn attributes(&self) -> io::Result<Vec<Attribute>> {
        Ok(Vec::new())
    }

    fn symlink_attributes(&self) -> io::Result<Vec<Attribute>> {
        Ok(Vec::new())
    }
}

#[cfg(any(
    target_os = "macos",
    target_os = "linux",
    target_os = "netbsd",
    target_os = "freebsd"
))]
mod extended_attrs {
    use super::Attribute;
    use libc::{c_char, c_void, size_t, ssize_t, ERANGE};
    use std::ffi::{CStr, CString, OsStr, OsString};
    use std::io;
    use std::os::unix::ffi::OsStrExt;
    use std::path::Path;
    use std::ptr::null_mut;

    #[cfg(target_os = "macos")]
    mod os {
        use libc::{
            c_char, c_int, c_void, getxattr, listxattr, size_t, ssize_t, XATTR_NOFOLLOW,
            XATTR_SHOWCOMPRESSION,
        };

        // Options to use for MacOS versions of getxattr and listxattr
        fn get_options(follow_symlinks: bool) -> c_int {
            if follow_symlinks {
                XATTR_SHOWCOMPRESSION
            } else {
                XATTR_NOFOLLOW | XATTR_SHOWCOMPRESSION
            }
        }

        // Wrapper around listxattr that handles symbolic links
        pub(super) fn list_xattr(
            follow_symlinks: bool,
            path: *const c_char,
            namebuf: *mut c_char,
            size: size_t,
        ) -> ssize_t {
            // SAFETY: Calling C function
            unsafe { listxattr(path, namebuf, size, get_options(follow_symlinks)) }
        }

        // Wrapper around getxattr that handles symbolic links
        pub(super) fn get_xattr(
            follow_symlinks: bool,
            path: *const c_char,
            name: *const c_char,
            value: *mut c_void,
            size: size_t,
        ) -> ssize_t {
            // SAFETY: Calling C function
            unsafe { getxattr(path, name, value, size, 0, get_options(follow_symlinks)) }
        }
    }

    #[cfg(target_os = "linux")]
    mod os {
        use libc::{c_char, c_void, size_t, ssize_t};

        use libc::{getxattr, lgetxattr, listxattr, llistxattr};

        // Wrapper around listxattr and llistattr for handling symbolic links
        pub(super) fn list_xattr(
            follow_symlinks: bool,
            path: *const c_char,
            namebuf: *mut c_char,
            size: size_t,
        ) -> ssize_t {
            if follow_symlinks {
                // SAFETY: Calling C function
                unsafe { listxattr(path, namebuf, size) }
            } else {
                // SAFETY: Calling C function
                unsafe { llistxattr(path, namebuf, size) }
            }
        }

        // Wrapper around getxattr and lgetxattr for handling symbolic links
        pub(super) fn get_xattr(
            follow_symlinks: bool,
            path: *const c_char,
            name: *const c_char,
            value: *mut c_void,
            size: size_t,
        ) -> ssize_t {
            if follow_symlinks {
                // SAFETY: Calling C function
                unsafe { getxattr(path, name, value, size) }
            } else {
                // SAFETY: Calling C function
                unsafe { lgetxattr(path, name, value, size) }
            }
        }
    }

    #[cfg(any(target_os = "netbsd", target_os = "freebsd"))]
    mod os {
        use libc::{
            c_char, c_int, c_void, extattr_get_file, extattr_get_link, extattr_list_file,
            extattr_list_link, size_t, ssize_t, EXTATTR_NAMESPACE_SYSTEM, EXTATTR_NAMESPACE_USER,
        };

        // Wrapper around listxattr that handles symbolic links
        fn list_xattr(
            follow_symlinks: bool,
            path: *const c_char,
            namespace: c_int,
            value: *mut c_void,
            size: size_t,
        ) -> ssize_t {
            if follow_symlinks {
                // SAFETY: Calling C function
                unsafe { extattr_list_file(path, namespace, value, size) }
            } else {
                // SAFETY: Calling C function
                unsafe { extattr_list_link(path, namespace, value, size) }
            }
        }

        fn get_xattr(
            follow_symlinks: bool,
            path: *const c_char,
            namespace: c_int,
            name: *const c_char,
            value: *mut c_void,
            size: size_t,
        ) -> ssize_t {
            if follow_symlinks {
                // SAFETY: Calling C function
                unsafe { extattr_get_file(path, namespace, name, value, size) }
            } else {
                // SAFETY: Calling C function
                unsafe { extattr_get_link(path, namespace, name, value, size) }
            }
        }

        pub(super) fn list_system_xattr(
            follow_symlinks: bool,
            path: *const c_char,
            namebuf: *mut c_char,
            size: size_t,
        ) -> ssize_t {
            list_xattr(
                follow_symlinks,
                path,
                EXTATTR_NAMESPACE_SYSTEM,
                namebuf.cast(),
                size,
            )
        }

        pub(super) fn list_user_xattr(
            follow_symlinks: bool,
            path: *const c_char,
            namebuf: *mut c_char,
            size: size_t,
        ) -> ssize_t {
            list_xattr(
                follow_symlinks,
                path,
                EXTATTR_NAMESPACE_USER,
                namebuf.cast(),
                size,
            )
        }

        pub(super) fn get_system_xattr(
            follow_symlinks: bool,
            path: *const c_char,
            name: *const c_char,
            value: *mut c_void,
            size: size_t,
        ) -> ssize_t {
            get_xattr(
                follow_symlinks,
                path,
                EXTATTR_NAMESPACE_SYSTEM,
                name,
                value.cast(),
                size,
            )
        }

        pub(super) fn get_user_xattr(
            follow_symlinks: bool,
            path: *const c_char,
            name: *const c_char,
            value: *mut c_void,
            size: size_t,
        ) -> ssize_t {
            get_xattr(
                follow_symlinks,
                path,
                EXTATTR_NAMESPACE_USER,
                name,
                value.cast(),
                size,
            )
        }
    }

    // Split attribute name list.  Each attribute name is null terminated in the
    // list.
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    fn split_attribute_list(buffer: &[u8]) -> Vec<OsString> {
        buffer[..buffer.len() - 1] // Skip trailing null
            .split(|&c| c == 0)
            .filter(|&s| !s.is_empty())
            .map(OsStr::from_bytes)
            .map(std::borrow::ToOwned::to_owned)
            .collect()
    }

    // Split attribute name list.  Each attribute is a one byte name length
    // followed by the name.
    #[cfg(any(target_os = "netbsd", target_os = "freebsd"))]
    fn split_attribute_list(buffer: &[u8]) -> Vec<OsString> {
        let mut result = Vec::new();
        let mut index = 0;
        let length = buffer.len();

        while index < length {
            let item_length = buffer[index] as usize;
            let start = index + 1;
            let end = start + item_length;
            if end <= length {
                result.push(OsStr::from_bytes(&buffer[start..end]).to_owned());
            }
            index = end;
        }

        result
    }

    // Calling getxattr and listxattr is a two part process.  The first call
    // a null ptr for buffer and a zero buffer size is passed and the function
    // returns the needed buffer size.  The second call the buffer ptr and the
    // buffer size is passed and the buffer is filled.  Care must be taken if
    // the buffer size changes between the first and second call.
    fn get_loop<F: Fn(*mut u8, usize) -> ssize_t>(f: F) -> io::Result<Option<Vec<u8>>> {
        let mut buffer: Vec<u8> = Vec::new();
        loop {
            let buffer_size = match f(null_mut(), 0) {
                -1 => return Err(io::Error::last_os_error()),
                0 => return Ok(None),
                size => size as size_t,
            };

            buffer.resize(buffer_size, 0);

            return match f(buffer.as_mut_ptr(), buffer_size) {
                -1 => {
                    let last_os_error = io::Error::last_os_error();
                    if last_os_error.raw_os_error() == Some(ERANGE) {
                        // Passed buffer was to small so retry again.
                        continue;
                    }
                    Err(last_os_error)
                }
                0 => Ok(None),
                len => {
                    // Just in case the size shrunk
                    buffer.truncate(len as usize);
                    Ok(Some(buffer))
                }
            };
        }
    }

    // Get a list of all attribute names on `path`
    fn list_attributes(
        path: &CStr,
        follow_symlinks: bool,
        lister: fn(
            follow_symlinks: bool,
            path: *const c_char,
            namebuf: *mut c_char,
            size: size_t,
        ) -> ssize_t,
    ) -> io::Result<Vec<OsString>> {
        Ok(
            get_loop(|buf, size| lister(follow_symlinks, path.as_ptr(), buf.cast(), size))?
                .map_or_else(Vec::new, |buffer| split_attribute_list(&buffer)),
        )
    }

    // Get the attribute value `name` on `path`
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    fn get_attribute(
        path: &CStr,
        name: &CStr,
        follow_symlinks: bool,
        getter: fn(
            follow_symlinks: bool,
            path: *const c_char,
            name: *const c_char,
            value: *mut c_void,
            size: size_t,
        ) -> ssize_t,
    ) -> io::Result<Option<Vec<u8>>> {
        use libc::ENODATA;

        get_loop(|buf, size| {
            getter(
                follow_symlinks,
                path.as_ptr(),
                name.as_ptr(),
                buf.cast(),
                size,
            )
        })
        .or_else(|err| {
            if err.raw_os_error() == Some(ENODATA) {
                // This handles the case when the named attribute is not on the
                // path.  This is for mainly handling the special case for the
                // security.selinux attribute mentioned below.  This can
                // also happen when an attribute is deleted between listing
                // the attributes and getting its value.
                Ok(None)
            } else {
                Err(err)
            }
        })
    }

    #[cfg(any(target_os = "netbsd", target_os = "freebsd"))]
    fn get_attribute(
        path: &CStr,
        name: &CStr,
        follow_symlinks: bool,
        getter: fn(
            follow_symlinks: bool,
            path: *const c_char,
            name: *const c_char,
            value: *mut c_void,
            size: size_t,
        ) -> ssize_t,
    ) -> io::Result<Option<Vec<u8>>> {
        get_loop(|buf, size| {
            getter(
                follow_symlinks,
                path.as_ptr(),
                name.as_ptr(),
                buf.cast(),
                size,
            )
        })
    }

    // Specially handle security.linux for filesystem that do not list attributes.
    #[cfg(target_os = "linux")]
    fn get_selinux_attribute(path: &CStr, follow_symlinks: bool) -> io::Result<Vec<Attribute>> {
        const SELINUX_XATTR_NAME: &str = "security.selinux";
        let name = CString::new(SELINUX_XATTR_NAME).unwrap();

        get_attribute(path, &name, follow_symlinks, os::get_xattr).map(|value| {
            if value.is_some() {
                vec![Attribute {
                    name: String::from(SELINUX_XATTR_NAME),
                    value,
                }]
            } else {
                Vec::new()
            }
        })
    }

    // Get a vector of all attribute names and values on `path`
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    pub fn attributes(path: &Path, follow_symlinks: bool) -> io::Result<Vec<Attribute>> {
        let path = CString::new(path.as_os_str().as_bytes())
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        let attr_names = list_attributes(&path, follow_symlinks, os::list_xattr)?;

        #[cfg(target_os = "linux")]
        if attr_names.is_empty() {
            // Some filesystems, like sysfs, return nothing on listxattr, even though the security
            // attribute is set.
            return get_selinux_attribute(&path, follow_symlinks);
        }

        let mut attrs = Vec::with_capacity(attr_names.len());
        for attr_name in attr_names {
            if let Some(name) = attr_name.to_str() {
                let attr_name =
                    CString::new(name).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
                let value = get_attribute(&path, &attr_name, follow_symlinks, os::get_xattr)?;
                attrs.push(Attribute {
                    name: name.to_string(),
                    value,
                });
            }
        }

        Ok(attrs)
    }

    #[cfg(any(target_os = "netbsd", target_os = "freebsd"))]
    fn get_namespace_attributes(
        path: &CStr,
        follow_symlinks: bool,
        attr_names: Vec<OsString>,
        namespace: &str,
        getter: fn(
            follow_symlinks: bool,
            path: *const c_char,
            name: *const c_char,
            value: *mut c_void,
            size: size_t,
        ) -> ssize_t,
        attrs: &mut Vec<Attribute>,
    ) -> io::Result<()> {
        for attr_name in attr_names {
            if let Some(name) = attr_name.to_str() {
                let attr_name =
                    CString::new(name).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
                let value = get_attribute(path, &attr_name, follow_symlinks, getter)?;
                attrs.push(Attribute {
                    name: format!("{namespace}::{name}"),
                    value,
                });
            }
        }
        Ok(())
    }

    #[cfg(any(target_os = "netbsd", target_os = "freebsd"))]
    pub fn attributes(path: &Path, follow_symlinks: bool) -> io::Result<Vec<Attribute>> {
        use libc::EPERM;

        let path = CString::new(path.as_os_str().as_bytes())
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        let attr_names_system = list_attributes(&path, follow_symlinks, os::list_system_xattr)
            .or_else(|err| {
                // Reading of attributes in the system namespace is only supported for root
                if err.raw_os_error() == Some(EPERM) {
                    Ok(Vec::new())
                } else {
                    Err(err)
                }
            })?;
        let attr_names_user = list_attributes(&path, follow_symlinks, os::list_user_xattr)?;

        let mut attrs = Vec::with_capacity(attr_names_system.len() + attr_names_user.len());

        get_namespace_attributes(
            &path,
            follow_symlinks,
            attr_names_system,
            "system",
            os::get_system_xattr,
            &mut attrs,
        )?;
        get_namespace_attributes(
            &path,
            follow_symlinks,
            attr_names_user,
            "user",
            os::get_user_xattr,
            &mut attrs,
        )?;

        Ok(attrs)
    }
}

const ATTRIBUTE_VALUE_MAX_HEX_LENGTH: usize = 16;

// Display for an attribute.  Attribute values that have a custom display are
// enclosed in curley brackets.
impl Display for Attribute {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}: ", self.name))?;
        if let Some(value) = custom_attr_display(self) {
            f.write_fmt(format_args!("<{value}>"))
        } else {
            match &self.value {
                None => f.write_str("<empty>"),
                Some(value) => {
                    if let Some(val) = custom_value_display(value) {
                        f.write_fmt(format_args!("<{val}>"))
                    } else if let Ok(v) = str::from_utf8(value) {
                        f.write_fmt(format_args!("{:?}", v.trim_end_matches(char::from(0))))
                    } else if value.len() <= ATTRIBUTE_VALUE_MAX_HEX_LENGTH {
                        f.write_fmt(format_args!("{value:02x?}"))
                    } else {
                        f.write_fmt(format_args!("<length {}>", value.len()))
                    }
                }
            }
        }
    }
}

struct AttributeDisplay {
    pub attribute: &'static str,
    pub display: fn(&Attribute) -> Option<String>,
}

// Check for a custom display by attribute name and call the display function
fn custom_attr_display(attribute: &Attribute) -> Option<String> {
    let name = attribute.name.as_str();
    // Strip off MacOS Metadata Persistence Flags
    // See https://eclecticlight.co/2020/11/02/controlling-metadata-tricks-with-persistence/
    #[cfg(target_os = "macos")]
    let name = name.rsplit_once('#').map_or(name, |n| n.0);

    ATTRIBUTE_DISPLAYS
        .iter()
        .find(|c| c.attribute == name)
        .and_then(|c| (c.display)(attribute))
}

#[cfg(target_os = "macos")]
const ATTRIBUTE_DISPLAYS: &[AttributeDisplay] = &[
    AttributeDisplay {
        attribute: "com.apple.lastuseddate",
        display: display_lastuseddate,
    },
    AttributeDisplay {
        attribute: "com.apple.macl",
        display: display_macl,
    },
];

#[cfg(not(target_os = "macos"))]
const ATTRIBUTE_DISPLAYS: &[AttributeDisplay] = &[];

// com.apple.lastuseddate is two 64-bit values representing the seconds and nano seconds
// from January 1, 1970
#[cfg(target_os = "macos")]
fn display_lastuseddate(attribute: &Attribute) -> Option<String> {
    use chrono::{Local, SecondsFormat, TimeZone};

    attribute
        .value
        .as_ref()
        .filter(|value| value.len() == 16)
        .and_then(|value| {
            let sec = i64::from_le_bytes(value[0..8].try_into().unwrap());
            let n_sec = i64::from_le_bytes(value[8..].try_into().unwrap());
            Local
                .timestamp_opt(sec, n_sec as u32)
                .map(|dt| dt.to_rfc3339_opts(SecondsFormat::Nanos, true))
                .single()
        })
}

// com.apple.macl is a two byte flag followed by a uuid for the application
#[cfg(target_os = "macos")]
fn format_macl(value: &[u8]) -> String {
    const HEX: [u8; 16] = [
        b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9', b'a', b'b', b'c', b'd', b'e',
        b'f',
    ];
    const GROUPS: [(usize, usize, u8); 6] = [
        (0, 4, b';'),
        (5, 13, b'-'),
        (14, 18, b'-'),
        (19, 23, b'-'),
        (24, 28, b'-'),
        (29, 41, 0),
    ];

    let mut dst = [0; 41];
    let mut i = 0;

    for (start, end, sep) in GROUPS {
        for j in (start..end).step_by(2) {
            let x = value[i];
            i += 1;
            dst[j] = HEX[(x >> 4) as usize];
            dst[j + 1] = HEX[(x & 0x0f) as usize];
        }
        if sep != 0 {
            dst[end] = sep;
        }
    }

    // SAFETY: Vector generated above with only ASCII characters.
    unsafe { String::from_utf8_unchecked(dst.to_vec()) }
}

// See https://book.hacktricks.xyz/macos-hardening/macos-security-and-privilege-escalation/macos-security-protections/macos-tcc
#[cfg(target_os = "macos")]
fn display_macl(attribute: &Attribute) -> Option<String> {
    attribute
        .value
        .as_ref()
        .filter(|v| v.len() % 18 == 0)
        .map(|v| {
            let macls = v
                .as_slice()
                .chunks(18)
                .filter(|c| c[0] != 0 || c[1] != 0)
                .map(format_macl)
                .collect::<Vec<String>>()
                .join(", ");
            format!("[{macls}]")
        })
}

// plist::XmlWriter takes the writer instead of borrowing it.  This is a
// wrapper around a borrowed vector that just forwards the Write trait
// calls to the borrowed vector.
struct BorrowedWriter<'a> {
    pub buffer: &'a mut Vec<u8>,
}

impl io::Write for BorrowedWriter<'_> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.buffer.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.buffer.flush()
    }

    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.buffer.write_all(buf)
    }
}

fn custom_value_display(value: &[u8]) -> Option<String> {
    if value.starts_with(b"bplist") {
        plist_value_display(value)
    } else {
        None
    }
}

// Convert a binary plist to a XML plist.
fn plist_value_display(value: &[u8]) -> Option<String> {
    let reader = io::Cursor::new(value);
    plist::Value::from_reader(reader).ok().and_then(|v| {
        let mut buffer = Vec::new();
        v.to_writer_xml_with_options(
            BorrowedWriter {
                buffer: &mut buffer,
            },
            &plist::XmlWriteOptions::default()
                .indent(b' ', 0)
                .root_element(false),
        )
        .ok()
        .and_then(|()| str::from_utf8(&buffer).ok())
        .map(|s| format!("<plist version=\"1.0\">{}</plist>", s.replace('\n', "")))
    })
}
