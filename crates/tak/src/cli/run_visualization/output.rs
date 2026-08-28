use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{Result, anyhow};
use tak_exec::{
    TaskFinishedEvent, TaskOutputChunk, TaskOutputObserver, TaskStartedEvent, TaskStatusEvent,
    TaskStructuredStatusEvent,
};

use super::model::RunState;
use super::output_io::{OutputBuffers, write_status_line, write_stderr, write_stdout};
use super::terminal::TerminalDisplay;
use crate::cli::task_history::HistoryOutputObserver;

struct OutputState {
    run: RunState,
    display: TerminalDisplay,
    buffers: OutputBuffers,
}

pub(in crate::cli) struct RunVisualizationObserver {
    state: Mutex<OutputState>,
    history: HistoryOutputObserver,
    refresh_stopped: AtomicBool,
}

impl RunVisualizationObserver {
    pub(in crate::cli) fn new(jobs: usize) -> Result<Self> {
        let observer = Self {
            state: Mutex::new(OutputState {
                run: RunState::new(jobs),
                display: TerminalDisplay::detect(),
                buffers: OutputBuffers::new(),
            }),
            history: HistoryOutputObserver::new_recording_only(),
            refresh_stopped: AtomicBool::new(false),
        };
        let mut state = observer.lock_state()?;
        if state.display.is_inline() {
            redraw(&mut state)?;
        } else {
            write_stderr(b"tak run: planning execution graph\n")?;
        }
        drop(state);
        Ok(observer)
    }

    pub(in crate::cli) fn finish_run(&self, error: Option<&anyhow::Error>) -> Result<()> {
        self.refresh_stopped.store(true, Ordering::SeqCst);
        let mut state = self.lock_state()?;
        state.display.begin_log()?;
        let OutputState { run, buffers, .. } = &mut *state;
        buffers.flush_all(run)?;
        if let Some(error) = error {
            write_stderr(format!("tak run: failed — {error}\n").as_bytes())?;
        }
        state.run.finish(error.map(|error| format!("{error:#}")));
        let OutputState { run, display, .. } = &mut *state;
        display.final_frame(run)
    }

    pub(in crate::cli) fn write_notice(&self, message: &str) -> Result<()> {
        let mut state = self.lock_state()?;
        state.display.begin_log()?;
        write_stderr(format!("{message}\n").as_bytes())?;
        redraw(&mut state)
    }

    pub(in crate::cli) fn write_result_line(&self, message: &str) -> Result<()> {
        let mut state = self.lock_state()?;
        state.display.begin_log()?;
        write_stdout(format!("{message}\n").as_bytes())?;
        redraw(&mut state)
    }

    pub(in crate::cli) fn start_refresh(self: &Arc<Self>) -> Result<()> {
        if !self.lock_state()?.display.is_inline() {
            return Ok(());
        }
        let observer = Arc::downgrade(self);
        std::thread::Builder::new()
            .name("tak-run-visualization".into())
            .spawn(move || refresh_loop(observer))?;
        Ok(())
    }

    fn lock_state(&self) -> Result<std::sync::MutexGuard<'_, OutputState>> {
        self.state
            .lock()
            .map_err(|_| anyhow!("run visualization lock poisoned"))
    }
}

fn refresh_loop(observer: std::sync::Weak<RunVisualizationObserver>) {
    loop {
        std::thread::sleep(Duration::from_secs(1));
        let Some(observer) = observer.upgrade() else {
            return;
        };
        if observer.refresh_stopped.load(Ordering::SeqCst) {
            return;
        }
        let result = observer.lock_state().and_then(|mut state| {
            state.display.begin_log()?;
            redraw(&mut state)
        });
        if result.is_err() {
            observer.refresh_stopped.store(true, Ordering::SeqCst);
            return;
        }
    }
}

impl TaskOutputObserver for RunVisualizationObserver {
    fn observe_output(&self, chunk: TaskOutputChunk) -> Result<()> {
        let mut state = self.lock_state()?;
        state.display.begin_log()?;
        self.history.observe_output(chunk.clone())?;
        let root = state.run.display_root(&chunk.task_label);
        let placement = state.run.placement_for(&root);
        state.buffers.emit_chunk(&chunk, &root, &placement)?;
        redraw(&mut state)
    }

    fn observe_status(&self, event: TaskStatusEvent) -> Result<()> {
        let mut state = self.lock_state()?;
        state.display.begin_log()?;
        self.history.observe_status(event.clone())?;
        state.run.apply_status(&event);
        write_status_line(&state.run, &event.task_label, None, None, &event.message)?;
        redraw(&mut state)
    }

    fn observe_structured_status(&self, event: TaskStructuredStatusEvent) -> Result<()> {
        let mut state = self.lock_state()?;
        state.display.begin_log()?;
        self.history.observe_structured_status(event.clone())?;
        state.run.apply_structured(event.clone());
        let is_queue_event = matches!(
            event.kind,
            tak_exec::TaskStatusEventKind::QueueAdmission
                | tak_exec::TaskStatusEventKind::QueuePositionChanged
        );
        write_status_line(
            &state.run,
            &event.task_label,
            is_queue_event
                .then_some(event.queue_id.as_deref())
                .flatten(),
            is_queue_event.then_some(event.queue_position).flatten(),
            &event.message,
        )?;
        redraw(&mut state)
    }

    fn observe_task_started(&self, event: TaskStartedEvent) -> Result<()> {
        let mut state = self.lock_state()?;
        state.display.begin_log()?;
        self.history.observe_task_started(event.clone())?;
        state.run.apply_started(event.clone());
        write_status_line(&state.run, &event.task_label, None, None, "task started")?;
        redraw(&mut state)
    }

    fn observe_task_finished(&self, event: TaskFinishedEvent) -> Result<()> {
        let mut state = self.lock_state()?;
        state.display.begin_log()?;
        self.history.observe_task_finished(event.clone())?;
        let root = state.run.display_root(&event.task_label);
        let placement = state.run.placement_for(&root);
        state.buffers.flush_task(&root, &placement)?;
        state.run.apply_finished(event);
        write_status_line(&state.run, &root, None, None, "task finished")?;
        redraw(&mut state)
    }
}

fn redraw(state: &mut OutputState) -> Result<()> {
    let OutputState { run, display, .. } = state;
    display.redraw(run)
}
