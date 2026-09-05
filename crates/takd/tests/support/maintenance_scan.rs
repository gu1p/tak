use std::ffi::CString;
use std::fs::{self, File, OpenOptions};
use std::os::unix::{ffi::OsStrExt, fs::OpenOptionsExt};
use std::path::Path;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use takd::{RunStore, RunStoreMaintenanceReport};

// A FIFO makes the cache metadata read pause until the test closes its writer.
pub fn pause_scan(
    store: RunStore,
    db: &Path,
) -> (File, JoinHandle<anyhow::Result<RunStoreMaintenanceReport>>) {
    let cache = db.with_extension("v2-blobs").join("path-caches/slow");
    fs::create_dir_all(&cache).unwrap();
    fs::write(cache.join("value"), b"cache").unwrap();
    let marker = cache.join(".last-accessed-ms");
    let name = CString::new(marker.as_os_str().as_bytes()).unwrap();
    // SAFETY: name is a live, NUL-terminated pathname in this test's private directory.
    assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
    let maintenance = thread::spawn(move || store.run_maintenance_at(100));
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        match OpenOptions::new()
            .write(true)
            .custom_flags(libc::O_NONBLOCK)
            .open(&marker)
        {
            Ok(writer) => return (writer, maintenance),
            Err(error) if error.raw_os_error() == Some(libc::ENXIO) => {
                assert!(
                    Instant::now() < deadline,
                    "maintenance never reached the scan"
                );
                thread::sleep(Duration::from_millis(5));
            }
            Err(error) => panic!("open scan barrier: {error}"),
        }
    }
}
