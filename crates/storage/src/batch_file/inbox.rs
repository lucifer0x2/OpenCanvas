//! Descriptor-relative, no-follow traversal prevents symlink substitution between
//! validation and open. Nonblocking open prevents a FIFO from hanging a storage task.
use crate::*;
#[cfg(unix)]
use std::{
    ffi::CString,
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::ffi::OsStrExt,
    },
    path::Component,
};

fn rejected() -> StorageError {
    StorageError::new("PATH_REJECTED", "Batch path must name a regular file inside this project's inbox; traversal and symlinks are forbidden")
}

#[cfg(unix)]
pub(super) fn open(root: &Path, project: &Path, input: &str) -> Result<File> {
    if input.is_empty() || input.len() > 4096 || input.as_bytes().contains(&0) {
        return Err(rejected());
    }
    let path = Path::new(input);
    // Reject traversal rather than normalize it, even if it would end up inside inbox.
    if path
        .components()
        .any(|p| matches!(p, Component::ParentDir | Component::CurDir))
    {
        return Err(rejected());
    }
    let inbox = project.join("inbox");
    let relative = if path.is_absolute() {
        path.strip_prefix(&inbox).map_err(|_| rejected())?
    } else {
        path
    };
    let parts: Vec<_> = relative.components().collect();
    if parts.is_empty() || parts.iter().any(|p| !matches!(p, Component::Normal(_))) {
        return Err(rejected());
    }
    // Start from the canonical data root, then anchor every lookup to an already
    // opened directory descriptor. Neither inbox nor intermediate dirs are created.
    let root_name = CString::new(root.as_os_str().as_bytes()).map_err(|_| rejected())?;
    let flags = libc::O_RDONLY | libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK;
    // SAFETY: CString is NUL terminated; successful fd ownership is transferred once.
    let fd = unsafe { libc::open(root_name.as_ptr(), flags | libc::O_DIRECTORY) };
    let mut directory = checked_fd(fd)?;
    let slug = project.file_name().ok_or_else(rejected)?;
    directory = child(&directory, slug, true)?;
    directory = child(&directory, std::ffi::OsStr::new("inbox"), true)?;
    for (index, component) in parts.iter().enumerate() {
        directory = child(&directory, component.as_os_str(), index + 1 < parts.len())?;
    }
    Ok(directory)
}

#[cfg(unix)]
fn child(directory: &File, name: &std::ffi::OsStr, is_dir: bool) -> Result<File> {
    let name = CString::new(name.as_bytes()).map_err(|_| rejected())?;
    if !is_dir {
        let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
        // SAFETY: stat points to valid writable space; only read after success.
        let status = unsafe {
            libc::fstatat(
                directory.as_raw_fd(),
                name.as_ptr(),
                stat.as_mut_ptr(),
                libc::AT_SYMLINK_NOFOLLOW,
            )
        };
        if status == 0 {
            let stat = unsafe { stat.assume_init() };
            if stat.st_mode & libc::S_IFMT != libc::S_IFREG {
                return Err(rejected());
            }
        }
        // openat below maps lookup errors; the opened descriptor is checked again
        // before reading to cover replacement after this regular-file preflight.
    }
    let flags = libc::O_RDONLY
        | libc::O_CLOEXEC
        | libc::O_NOFOLLOW
        | libc::O_NONBLOCK
        | libc::O_NOCTTY
        | if is_dir { libc::O_DIRECTORY } else { 0 };
    // SAFETY: the directory fd is live, and CString is NUL terminated.
    let fd = unsafe { libc::openat(directory.as_raw_fd(), name.as_ptr(), flags) };
    checked_fd(fd)
}

#[cfg(unix)]
fn checked_fd(fd: libc::c_int) -> Result<File> {
    if fd < 0 {
        return Err(match std::io::Error::last_os_error().raw_os_error() {
            Some(libc::ENOENT) => {
                StorageError::new("FILE_NOT_FOUND", "Batch inbox file does not exist")
            }
            _ => rejected(),
        });
    }
    // SAFETY: fd is a newly opened descriptor, uniquely owned by the returned File.
    Ok(unsafe { File::from_raw_fd(fd) })
}

#[cfg(not(unix))]
pub(super) fn open(_: &Path, _: &Path, _: &str) -> Result<File> {
    Err(StorageError::new(
        "UNSUPPORTED_PLATFORM",
        "Secure batch inbox reads require Unix",
    ))
}
