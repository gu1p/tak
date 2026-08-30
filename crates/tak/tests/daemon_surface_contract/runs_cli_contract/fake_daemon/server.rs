use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::Value;

use super::Reply;
use super::read::read_request;
use super::response::response_bytes;
use super::write::{write_response, write_slow_prefix};

pub(super) fn serve(
    listener: UnixListener,
    reply: Reply,
    stop: &AtomicBool,
    requests: &Arc<Mutex<Vec<Value>>>,
) {
    std::thread::scope(|scope| {
        loop {
            let (stream, _) = listener.accept().expect("accept fake run request");
            if stop.load(Ordering::Acquire) {
                break;
            }
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .expect("bound fake request read");
            stream
                .set_write_timeout(Some(Duration::from_secs(2)))
                .expect("bound fake response write");
            if matches!(reply, Reply::RawThenStall(_)) {
                handle(stream, &reply, stop, requests);
                continue;
            }
            let reply = &reply;
            let requests = Arc::clone(requests);
            scope.spawn(move || handle(stream, reply, stop, &requests));
        }
    });
}

fn handle(
    mut stream: UnixStream,
    reply: &Reply,
    stop: &AtomicBool,
    requests: &Arc<Mutex<Vec<Value>>>,
) {
    let Some(line) = read_request(&stream) else {
        return;
    };
    let request: Value = serde_json::from_str(line.trim_end()).expect("decode run request");
    let request_id = request["request_id"]
        .as_str()
        .expect("request id")
        .to_string();
    let request_number = requests.lock().expect("request capture lock").len();
    let bytes = response_bytes(reply, &request_id, &request, request_number);
    let delay = match reply {
        Reply::DelayedSubmissionFlow(operation, delay)
            if request["operation"]["type"].as_str() == Some(operation) =>
        {
            Some(*delay)
        }
        Reply::DelayedCancellationFlow(operation, delay)
            if request["operation"]["type"].as_str() == Some(operation)
                || request["operation"]["type"].as_str() == Some("CancelRun") =>
        {
            Some(*delay)
        }
        _ => None,
    };
    requests.lock().expect("request capture lock").push(request);
    if let Some(delay) = delay {
        let deadline = Instant::now() + delay;
        while !stop.load(Ordering::Acquire) && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    if let Some(bytes) = bytes {
        if let Reply::SlowDripInactive(_, interval, prefix_bytes) = reply {
            write_slow_prefix(&mut stream, &bytes, *prefix_bytes, *interval, stop);
        } else {
            write_response(&mut stream, &bytes);
        }
    }
    if matches!(reply, Reply::RawThenStall(_)) {
        let deadline = Instant::now() + Duration::from_secs(30);
        while !stop.load(Ordering::Acquire) && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}
