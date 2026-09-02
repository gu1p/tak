use anyhow::{Result, bail};

pub(super) fn validate_digest(value: &str) -> Result<()> {
    if value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Ok(());
    }
    bail!("invalid terminal digest")
}

pub(super) fn validate_failure_exit_code(value: Option<i32>) -> Result<()> {
    if value.is_none_or(|code| (1..=u8::MAX.into()).contains(&code)) {
        return Ok(());
    }
    bail!("invalid failed process exit code")
}
