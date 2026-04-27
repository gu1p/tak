use std::fs;
use std::path::Path;

use anyhow::{Error, anyhow};
use tak_proto::{decode_remote_token, decode_tor_invite};

use super::paths::token_path;

#[derive(Debug)]
pub(super) enum ReadTokenError {
    NotReady,
    TransportNotReady(String),
    Invalid(Error),
}

pub(super) fn read_token_state(state_root: &Path) -> std::result::Result<String, ReadTokenError> {
    let path = token_path(state_root);
    let token = match fs::read_to_string(&path) {
        Ok(token) => token,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Err(ReadTokenError::NotReady);
        }
        Err(err) => return Err(ReadTokenError::Invalid(err.into())),
    };
    let token = token.trim().to_string();
    if token.is_empty() {
        return Err(ReadTokenError::NotReady);
    }
    if decode_remote_token(&token).is_err() {
        let _ = decode_tor_invite(&token).map_err(ReadTokenError::Invalid)?;
    }
    Ok(token)
}

pub(super) fn should_retry_token_error(err: &ReadTokenError) -> bool {
    matches!(
        err,
        ReadTokenError::NotReady | ReadTokenError::TransportNotReady(_)
    )
}

pub(super) fn read_token_error_into_anyhow(err: ReadTokenError) -> Error {
    match err {
        ReadTokenError::NotReady => anyhow!("agent token not ready"),
        ReadTokenError::TransportNotReady(detail) => anyhow!(detail),
        ReadTokenError::Invalid(err) => err,
    }
}
