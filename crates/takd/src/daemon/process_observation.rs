use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};
use tak_proto::worker_v2::WorkerProcessObservation;

pub(super) fn current() -> Vec<WorkerProcessObservation> {
    let mut system = System::new();
    system.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::nothing()
            .with_cmd(UpdateKind::Always)
            .without_tasks(),
    );
    let mut processes = system
        .processes()
        .values()
        .map(|process| WorkerProcessObservation {
            name: process.name().to_string_lossy().into_owned(),
            arguments: process
                .cmd()
                .iter()
                .map(|argument| argument.to_string_lossy().into_owned())
                .collect(),
        })
        .collect::<Vec<_>>();
    processes.sort_unstable_by(|left, right| {
        (&left.name, &left.arguments).cmp(&(&right.name, &right.arguments))
    });
    processes
}
