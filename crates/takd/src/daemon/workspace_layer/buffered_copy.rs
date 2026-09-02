use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::Path;

pub(super) fn copy(source: &Path, destination: &Path) -> io::Result<()> {
    let mut source = File::open(source)?;
    let mut destination_file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)?;
    let result = copy_inner(&mut source, &mut destination_file);
    drop(destination_file);
    if result.is_err() {
        let _ = fs::remove_file(destination);
    }
    result
}

fn copy_inner(source: &mut File, destination: &mut File) -> io::Result<()> {
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = source.read(&mut buffer)?;
        if read == 0 {
            return destination.flush();
        }
        destination.write_all(&buffer[..read])?;
    }
}
