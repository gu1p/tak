use std::io::{self, Write};

use anyhow::{Error, Result};

pub(in crate::cli) fn start_or_disable<T>(result: Result<Option<T>>, stage: &str) -> Option<T> {
    match result {
        Ok(dashboard) => dashboard,
        Err(error) => {
            report(stage, &error);
            None
        }
    }
}

pub(in crate::cli) fn attempt_or_disable<T, U>(
    dashboard: &mut Option<T>,
    operation: impl FnOnce(&mut T) -> Result<U>,
    stage: &str,
) -> Option<U> {
    let result = operation(dashboard.as_mut()?);
    match result {
        Ok(value) => Some(value),
        Err(error) => {
            disable_after_error(dashboard, error, stage);
            None
        }
    }
}

pub(in crate::cli) fn disable_after_error<T>(dashboard: &mut Option<T>, error: Error, stage: &str) {
    dashboard.take();
    report(stage, &error);
}

pub(in crate::cli) fn input_or_disable<T>(
    dashboard: &mut Option<T>,
    input: Result<()>,
    stage: &str,
) -> bool {
    match input {
        Ok(()) => true,
        Err(error) => {
            disable_after_error(dashboard, error, stage);
            false
        }
    }
}

pub(in crate::cli) fn safe_warning(stage: &str, error: &Error) -> String {
    format!("warning: run dashboard disabled {stage}; daemon-owned run continues: {error:#}")
        .chars()
        .filter(|character| !character.is_control())
        .collect()
}

fn report(stage: &str, error: &Error) {
    let mut stderr = io::stderr().lock();
    let _ = writeln!(stderr, "{}", safe_warning(stage, error));
}
