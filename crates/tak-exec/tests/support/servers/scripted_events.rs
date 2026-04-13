#![allow(dead_code)]

use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::Duration;

use tak_proto::{GetTaskResultResponse, RemoteEvent};

use self::{scripted_events_state::ScriptedEventsState, scripted_events_support::serve_request};

#[path = "scripted_events_state.rs"]
mod scripted_events_state;
#[path = "scripted_events_support.rs"]
mod scripted_events_support;
#[path = "scripted_node_status.rs"]
mod scripted_node_status;

#[derive(Clone)]
pub struct EventPollPlan {
    pub delay: Duration,
    pub events: Vec<RemoteEvent>,
    pub done: bool,
}

pub struct ScriptedEventsServer {
    pub base_url: String,
    handle: Option<thread::JoinHandle<()>>,
    port: u16,
}

impl ScriptedEventsServer {
    pub fn spawn(
        node_id: &str,
        plans: Vec<EventPollPlan>,
        result_ready_after_event_calls: usize,
        result: GetTaskResultResponse,
    ) -> Self {
        Self::spawn_with_status(node_id, plans, result_ready_after_event_calls, result, true)
    }

    pub fn spawn_with_status(
        node_id: &str,
        plans: Vec<EventPollPlan>,
        result_ready_after_event_calls: usize,
        result: GetTaskResultResponse,
        status_available: bool,
    ) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind scripted event server");
        let port = listener
            .local_addr()
            .expect("scripted listener addr")
            .port();
        let node_id = node_id.to_string();
        let handle = thread::spawn(move || {
            let mut state = ScriptedEventsState {
                node_id,
                port,
                status_available,
                fallback_plan: plans
                    .last()
                    .cloned()
                    .expect("scripted server needs poll plans"),
                plans,
                event_calls: 0,
                result_ready_after_event_calls,
                result,
            };
            loop {
                let (mut stream, _) = listener.accept().expect("accept scripted request");
                if !serve_request(&mut stream, &mut state) {
                    break;
                }
            }
        });
        Self {
            base_url: format!("http://127.0.0.1:{port}"),
            handle: Some(handle),
            port,
        }
    }
}

impl Drop for ScriptedEventsServer {
    fn drop(&mut self) {
        if let Ok(mut stream) = TcpStream::connect(("127.0.0.1", self.port)) {
            let _ = stream.write_all(
                b"GET /__shutdown HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
            );
        }
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}
