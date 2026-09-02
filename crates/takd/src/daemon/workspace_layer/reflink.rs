use std::io;
use std::path::Path;

#[cfg(target_os = "macos")]
pub(super) fn try_reflink(source: &Path, destination: &Path) -> io::Result<bool> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    let source = CString::new(source.as_os_str().as_bytes()).map_err(invalid_path)?;
    let destination = CString::new(destination.as_os_str().as_bytes()).map_err(invalid_path)?;
    // SAFETY: both pointers reference live, NUL-terminated path buffers for the full call.
    if unsafe { libc::clonefile(source.as_ptr(), destination.as_ptr(), 0) } == 0 {
        return Ok(true);
    }
    let _ = std::fs::remove_file(destination_path(&destination));
    Ok(false)
}

#[cfg(target_os = "macos")]
fn invalid_path(error: std::ffi::NulError) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, error)
}

#[cfg(target_os = "macos")]
fn destination_path(path: &std::ffi::CStr) -> &Path {
    use std::os::unix::ffi::OsStrExt;
    Path::new(std::ffi::OsStr::from_bytes(path.to_bytes()))
}

#[cfg(target_os = "linux")]
pub(super) fn try_reflink(source: &Path, destination: &Path) -> io::Result<bool> {
    use std::fs::{File, OpenOptions};
    use std::os::fd::AsRawFd;

    let source = File::open(source)?;
    let destination_file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)?;
    const FICLONE: libc::c_ulong = 0x4004_9409;
    // SAFETY: both descriptors remain open and valid for the duration of this ioctl call.
    let cloned =
        unsafe { libc::ioctl(destination_file.as_raw_fd(), FICLONE, source.as_raw_fd()) } == 0;
    drop(destination_file);
    if !cloned {
        std::fs::remove_file(destination)?;
    }
    Ok(cloned)
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
pub(super) fn try_reflink(_source: &Path, _destination: &Path) -> io::Result<bool> {
    Ok(false)
}
