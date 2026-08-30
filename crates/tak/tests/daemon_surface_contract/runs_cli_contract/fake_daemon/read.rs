use std::io::{BufRead, BufReader};
use std::os::unix::net::UnixStream;

pub(super) fn read_request(stream: &UnixStream) -> Option<String> {
    let mut reader = BufReader::new(stream.try_clone().expect("clone fake stream"));
    let mut line = String::new();
    match reader.read_line(&mut line) {
        Ok(0) => None,
        Ok(_) => Some(line),
        Err(error) => panic!("read run request: {error}"),
    }
}
