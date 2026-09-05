use std::collections::BTreeMap;
use std::io::{self, Write};
use std::sync::Mutex;

use anyhow::{Result, anyhow};
use tak_core::model::TaskLabel;
use tak_exec::{
    OutputStream, TaskFinishedEvent, TaskOutputChunk, TaskOutputObserver, TaskStatusEvent,
};
use tak_make::ParallelOutputMode;

use super::task::ParallelMakeGoal;
use lines::{complete_lines, flush_partials, record_make_exit_code, stream_key, write_prefixed};
use visibility::OutputVisibility;

mod lines;
mod persisted;
#[cfg(test)]
mod tests;
mod visibility;
#[cfg(test)]
mod visibility_bdd_tests;

struct GoalOutput {
    name: String,
    mode: ParallelOutputMode,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum StreamKey {
    Stdout,
    Stderr,
}

struct BufferedLine {
    stream: OutputStream,
    bytes: Vec<u8>,
}

#[derive(Default)]
struct OutputState {
    pending: BTreeMap<(TaskLabel, StreamKey), Vec<u8>>,
    grouped: BTreeMap<TaskLabel, Vec<BufferedLine>>,
    make_exit_codes: BTreeMap<TaskLabel, i32>,
    failures: BTreeMap<TaskLabel, i32>,
}

pub(super) struct ParallelMakeOutputObserver {
    goals: BTreeMap<TaskLabel, GoalOutput>,
    state: Mutex<OutputState>,
    visibility: OutputVisibility,
}

impl ParallelMakeOutputObserver {
    pub(super) fn new(
        goals: &[ParallelMakeGoal],
        override_mode: Option<ParallelOutputMode>,
    ) -> Self {
        Self::with_visibility(goals, override_mode, OutputVisibility::current())
    }

    fn with_visibility(
        goals: &[ParallelMakeGoal],
        override_mode: Option<ParallelOutputMode>,
        visibility: OutputVisibility,
    ) -> Self {
        let goals = goals
            .iter()
            .map(|goal| {
                (
                    goal.label.clone(),
                    GoalOutput {
                        name: goal.goal.clone(),
                        mode: override_mode.unwrap_or(goal.output),
                    },
                )
            })
            .collect();
        Self {
            goals,
            state: Mutex::new(OutputState::default()),
            visibility,
        }
    }

    pub(super) fn first_failure(&self, goals: &[ParallelMakeGoal]) -> Result<Option<i32>> {
        let state = self
            .state
            .lock()
            .map_err(|_| anyhow!("parallel Make output lock poisoned"))?;
        Ok(goals
            .iter()
            .find_map(|goal| state.failures.get(&goal.label).copied()))
    }

    fn observe_lines(&self, chunk: TaskOutputChunk) -> Result<()> {
        let Some(goal) = self.goals.get(&chunk.task_label) else {
            return Ok(());
        };
        let mut state = self
            .state
            .lock()
            .map_err(|_| anyhow!("parallel Make output lock poisoned"))?;
        let key = (chunk.task_label.clone(), stream_key(chunk.stream));
        let lines = complete_lines(state.pending.entry(key).or_default(), &chunk.bytes);
        for line in lines {
            record_make_exit_code(&mut state, &chunk.task_label, chunk.stream, &line);
            if !self.visibility.writes(chunk.stream) {
                continue;
            }
            match goal.mode {
                ParallelOutputMode::Live => write_prefixed(chunk.stream, &goal.name, &line)?,
                ParallelOutputMode::Grouped => state
                    .grouped
                    .entry(chunk.task_label.clone())
                    .or_default()
                    .push(BufferedLine {
                        stream: chunk.stream,
                        bytes: line,
                    }),
            }
        }
        Ok(())
    }

    fn finish(&self, event: TaskFinishedEvent) -> Result<()> {
        let Some(goal) = self.goals.get(&event.task_label) else {
            return Ok(());
        };
        let mut state = self
            .state
            .lock()
            .map_err(|_| anyhow!("parallel Make output lock poisoned"))?;
        flush_partials(&mut state, &event.task_label, goal, &self.visibility)?;
        if goal.mode == ParallelOutputMode::Grouped {
            for line in state.grouped.remove(&event.task_label).unwrap_or_default() {
                write_prefixed(line.stream, &goal.name, &line.bytes)?;
            }
        }
        if !event.success {
            let code = state
                .make_exit_codes
                .get(&event.task_label)
                .copied()
                .or(event.exit_code)
                .unwrap_or(1);
            state.failures.insert(event.task_label, code);
        }
        Ok(())
    }
}

impl TaskOutputObserver for ParallelMakeOutputObserver {
    fn observe_output(&self, chunk: TaskOutputChunk) -> Result<()> {
        self.observe_lines(chunk)
    }

    fn observe_status(&self, event: TaskStatusEvent) -> Result<()> {
        let Some(goal) = self.goals.get(&event.task_label) else {
            return Ok(());
        };
        writeln!(
            io::stderr().lock(),
            "[{}] [attempt {}] {}",
            goal.name,
            event.attempt,
            event.message
        )?;
        Ok(())
    }

    fn observe_task_finished(&self, event: TaskFinishedEvent) -> Result<()> {
        self.finish(event)
    }
}
