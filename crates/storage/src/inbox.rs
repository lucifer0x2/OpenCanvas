//! The per-project filesystem inbox shared by staged batches and asset imports.
//! Descriptor-relative, no-follow traversal prevents symlink substitution between
//! validation and open. Nonblocking open prevents a FIFO from hanging a storage task.
use crate::*;
use std::io::Read;
#[cfg(unix)]
use std::{
    ffi::CString,
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::ffi::OsStrExt,
    },
    path::Component,
};

/// What an inbox read is for. Only error wording and the byte bound differ; every
/// purpose gets the same containment checks.
#[derive(Clone, Copy)]
pub(crate) enum Purpose {
    Batch,
    Asset,
}

impl Purpose {
    fn rejected(self) -> StorageError {
        StorageError::new("PATH_REJECTED", match self {
            Self::Batch => "Batch path must name a regular file inside this project's inbox; traversal and symlinks are forbidden",
            Self::Asset => "Asset path must name a regular file inside this project's inbox; traversal and symlinks are forbidden",
        })
    }
    pub(crate) fn missing(self) -> StorageError {
        StorageError::new(
            "FILE_NOT_FOUND",
            match self {
                Self::Batch => "Batch inbox file does not exist",
                Self::Asset => "Asset inbox file does not exist",
            },
        )
    }
    fn not_regular(self) -> StorageError {
        StorageError::new(
            "PATH_REJECTED",
            match self {
                Self::Batch => "Batch input must be a regular file",
                Self::Asset => "Asset input must be a regular file",
            },
        )
    }
    fn limit(self) -> (usize, StorageError) {
        match self {
            Self::Batch => (
                MAX_BATCH_BYTES,
                StorageError::validation("path", "Batch file exceeds 2 MiB"),
            ),
            Self::Asset => (
                MAX_ASSET_BYTES,
                StorageError::validation("path", "Asset file exceeds 10 MiB"),
            ),
        }
    }
}

/// Reads one regular file from `<root>/<project slug>/inbox`, bounded by the purpose's
/// limit. `root` must be canonical; `project` is the project directory below it.
pub(crate) fn read(root: &Path, project: &Path, input: &str, purpose: Purpose) -> Result<Vec<u8>> {
    let file = open(root, project, input, purpose)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() {
        return Err(purpose.not_regular());
    }
    let (max, too_large) = purpose.limit();
    if metadata.len() > max as u64 {
        return Err(too_large);
    }
    // Check the stream too, so concurrent file growth cannot bypass the bound.
    let mut bytes = Vec::new();
    file.take(max as u64 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > max {
        return Err(too_large);
    }
    Ok(bytes)
}

#[cfg(unix)]
fn open(root: &Path, project: &Path, input: &str, purpose: Purpose) -> Result<File> {
    let rejected = || purpose.rejected();
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
    let mut directory = checked_fd(fd, purpose)?;
    let slug = project.file_name().ok_or_else(rejected)?;
    directory = child(&directory, slug, true, purpose)?;
    directory = child(&directory, std::ffi::OsStr::new("inbox"), true, purpose)?;
    for (index, component) in parts.iter().enumerate() {
        let is_dir = index + 1 < parts.len();
        directory = child(&directory, component.as_os_str(), is_dir, purpose)?;
    }
    Ok(directory)
}

#[cfg(unix)]
fn child(directory: &File, name: &std::ffi::OsStr, is_dir: bool, purpose: Purpose) -> Result<File> {
    let name = CString::new(name.as_bytes()).map_err(|_| purpose.rejected())?;
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
                return Err(purpose.rejected());
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
    checked_fd(fd, purpose)
}

#[cfg(unix)]
fn checked_fd(fd: libc::c_int, purpose: Purpose) -> Result<File> {
    if fd < 0 {
        return Err(match std::io::Error::last_os_error().raw_os_error() {
            Some(libc::ENOENT) => purpose.missing(),
            _ => purpose.rejected(),
        });
    }
    // SAFETY: fd is a newly opened descriptor, uniquely owned by the returned File.
    Ok(unsafe { File::from_raw_fd(fd) })
}

#[cfg(not(unix))]
fn open(_: &Path, _: &Path, _: &str, _: Purpose) -> Result<File> {
    Err(StorageError::new(
        "UNSUPPORTED_PLATFORM",
        "Secure batch inbox reads require Unix",
    ))
}
