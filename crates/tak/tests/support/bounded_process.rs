use std::io::{Read, Seek};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};

pub fn output_with_timeout(mut command: Command, timeout: Duration) -> Result<Output> {
    let mut stdout = tempfile::tempfile()?;
    let mut stderr = tempfile::tempfile()?;
    command
        .stdin(Stdio::null())
        .stdout(stdout.try_clone()?)
        .stderr(stderr.try_clone()?);
    let mut child = command.spawn().context("failed running bounded command")?;
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        let Some(remaining) = timeout.checked_sub(started.elapsed()) else {
            let _ = child.kill();
            child.wait().context("failed reaping timed-out command")?;
            bail!("command timed out after {timeout:?}");
        };
        std::thread::sleep(remaining.min(Duration::from_millis(1)));
    };
    stdout.rewind()?;
    stderr.rewind()?;
    let mut output = Output {
        status,
        stdout: Vec::new(),
        stderr: Vec::new(),
    };
    stdout.read_to_end(&mut output.stdout)?;
    stderr.read_to_end(&mut output.stderr)?;
    Ok(output)
}
