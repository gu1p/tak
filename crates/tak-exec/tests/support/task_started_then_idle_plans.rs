use std::time::Duration;

use tak_proto::RemoteEvent;

use crate::support::EventPollPlan;

pub fn task_started_then_idle_plans() -> Vec<EventPollPlan> {
    let mut plans = vec![EventPollPlan {
        delay: Duration::from_millis(1000),
        events: vec![RemoteEvent {
            seq: 1,
            kind: "TASK_STARTED".into(),
            timestamp_ms: 1,
            success: None,
            exit_code: None,
            message: None,
            chunk: None,
            chunk_bytes: Vec::new(),
        }],
        done: false,
    }];
    for _ in 0..7 {
        plans.push(EventPollPlan {
            delay: Duration::ZERO,
            events: Vec::new(),
            done: false,
        });
    }
    plans.push(EventPollPlan {
        delay: Duration::ZERO,
        events: Vec::new(),
        done: true,
    });
    plans
}
