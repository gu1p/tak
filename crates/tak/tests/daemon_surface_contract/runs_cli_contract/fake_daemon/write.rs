use std::io::Write;
use std::os::unix::net::UnixStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

pub(super) fn write_slow_prefix(
    stream: &mut UnixStream,
    bytes: &[u8],
    prefix_bytes: usize,
    interval: Duration,
    stop: &AtomicBool,
) {
    let split = prefix_bytes.min(bytes.len());
    for byte in &bytes[..split] {
        if stop.load(Ordering::Acquire) {
            return;
        }
        write_response(stream, std::slice::from_ref(byte));
        std::thread::sleep(interval);
    }
    if !stop.load(Ordering::Acquire) {
        write_response(stream, &bytes[split..]);
    }
}

pub(super) fn write_response(stream: &mut UnixStream, mut bytes: &[u8]) {
    while !bytes.is_empty() {
        match stream.write(bytes) {
            Ok(0) => panic!("fake daemon response write made no progress"),
            Ok(written) => bytes = &bytes[written..],
            Err(error) if disconnected(&error) => return,
            Err(error) => panic!("write fake daemon response: {error}"),
        }
    }
    if let Err(error) = stream.flush() {
        assert!(disconnected(&error), "flush fake daemon response: {error}");
    }
}

fn disconnected(error: &std::io::Error) -> bool {
    matches!(
        error.kind(),
        std::io::ErrorKind::BrokenPipe
            | std::io::ErrorKind::ConnectionReset
            | std::io::ErrorKind::NotConnected
    )
}
