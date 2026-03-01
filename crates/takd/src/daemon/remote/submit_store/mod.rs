use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result, anyhow, bail};
use rusqlite::{Connection, ErrorCode, params};

use super::query_helpers::unix_epoch_ms;
use super::types::SubmitEventRecord;

mod commands;
mod key;
mod persistence;
mod queries;
mod types;

pub use key::build_submit_idempotency_key;
use key::validate_submit_attempt;
pub use types::{SubmitAttemptStore, SubmitRegistration};

fn is_submit_unique_violation(error: &rusqlite::Error) -> bool {
    matches!(
        error,
        rusqlite::Error::SqliteFailure(inner, _)
            if inner.code == ErrorCode::ConstraintViolation
    )
}
