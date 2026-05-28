use super::*;

use sha2::{Digest, Sha256};

pub(super) fn output_file_size_and_sha256(path: &Path) -> Result<(u64, String)> {
    use std::io::Read;

    let mut file = fs::File::open(path)
        .with_context(|| format!("failed to read partial output {}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut size = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .with_context(|| format!("failed to read partial output {}", path.display()))?;
        if read == 0 {
            break;
        }
        size = size.saturating_add(read as u64);
        hasher.update(&buffer[..read]);
    }
    Ok((size, format!("{:x}", hasher.finalize())))
}
