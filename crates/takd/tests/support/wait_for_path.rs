use std::{fs, path::Path, time::SystemTime};

pub fn backdate(path: &Path) -> SystemTime {
    fs::File::open(path)
        .expect("open path to backdate")
        .set_modified(SystemTime::UNIX_EPOCH)
        .expect("backdate path");
    fs::metadata(path)
        .expect("read backdated path metadata")
        .modified()
        .expect("read backdated path mtime")
}

pub async fn wait_for_path(path: &Path, expected_present: bool, action: &str) {
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(45);
    while path.exists() != expected_present {
        assert!(
            tokio::time::Instant::now() < deadline,
            "timed out waiting for {action} of {}",
            path.display()
        );
        tokio::time::sleep(std::time::Duration::from_millis(2)).await;
    }
}

pub async fn wait_for_path_refreshed(path: &Path, baseline: SystemTime) {
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(45);
    loop {
        let modified = fs::metadata(path)
            .expect("read path metadata while waiting for refresh")
            .modified()
            .expect("read path mtime while waiting for refresh");
        if modified > baseline {
            return;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "timed out waiting for refresh of {}",
            path.display()
        );
        tokio::time::sleep(std::time::Duration::from_millis(2)).await;
    }
}
