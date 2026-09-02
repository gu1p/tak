use std::sync::Mutex;

use anyhow::Result;
use futures::future::{BoxFuture, FutureExt};
use tak_core::v2::{Affinity, JobEdge, Session, SessionReuse};
use takd::{
    AttemptCompletion, AttemptDispatch, AttemptObservation, AttemptTransport, DispatchCommand,
    RunStore,
};

use super::v2_run::scheduler::independent_jobs;

pub fn sequential_shared(key: &str) -> tak_core::v2::RunSubmission {
    let mut request = independent_jobs(key, 2);
    let affinity = Affinity::require_same_node("shared").unwrap();
    let session = Session::new(
        "session-a",
        SessionReuse::shared_workspace(1).unwrap(),
        Some(affinity.clone()),
    )
    .unwrap();
    for task in &mut request.run.tasks {
        task.affinity = Some(affinity.clone());
    }
    for job in &mut request.run.jobs {
        job.affinity = Some(affinity.clone());
        job.session = Some(session.clone());
    }
    request.run.tasks[1].dependencies = vec![request.run.tasks[0].task_id.clone()];
    request.run.job_edges = vec![JobEdge {
        dependency_job_id: request.run.jobs[0].job_id.clone(),
        dependent_job_id: request.run.jobs[1].job_id.clone(),
    }];
    tak_core::v2::RunSubmission::new(
        request.idempotency_key,
        request.run,
        request.environment_values,
    )
    .unwrap()
}

#[derive(Default)]
pub struct AckRecorder {
    pub acks: Mutex<Vec<(String, bool)>>,
    completion: Mutex<Option<(RunStore, DispatchCommand)>>,
}

impl AckRecorder {
    pub fn completing(store: RunStore, command: DispatchCommand) -> Self {
        Self {
            completion: Mutex::new(Some((store, command))),
            ..Self::default()
        }
    }
}

impl AttemptTransport for AckRecorder {
    fn dispatch<'a>(&'a self, _: &'a DispatchCommand) -> BoxFuture<'a, Result<AttemptDispatch>> {
        async { Ok(AttemptDispatch::Accepted) }.boxed()
    }

    fn cancel_and_wait<'a>(&'a self, _: &'a DispatchCommand) -> BoxFuture<'a, Result<()>> {
        async { Ok(()) }.boxed()
    }

    fn reconcile<'a>(
        &'a self,
        _: &'a DispatchCommand,
    ) -> BoxFuture<'a, Result<AttemptObservation>> {
        async { Ok(AttemptObservation::Running) }.boxed()
    }

    fn acknowledge_terminal<'a>(
        &'a self,
        command: &'a DispatchCommand,
        _: &'a str,
        run_terminal: bool,
    ) -> BoxFuture<'a, Result<()>> {
        async move {
            let completion = (!run_terminal)
                .then(|| self.completion.lock().unwrap().take())
                .flatten();
            if let Some((store, command)) = completion {
                store.complete_attempt(
                    &command,
                    AttemptCompletion::Succeeded {
                        terminal_digest: "b".repeat(64),
                    },
                )?;
            }
            self.acks
                .lock()
                .unwrap()
                .push((command.job_id.clone(), run_terminal));
            Ok(())
        }
        .boxed()
    }
}
