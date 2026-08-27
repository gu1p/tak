use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;

const MAX_LOG_TAIL_BYTES: u64 = 1024 * 1024;

pub fn read_log_tail(log_path: &Path, lines: usize) -> io::Result<String> {
    let mut file = File::open(log_path)?;
    if lines == 0 {
        return Ok(String::new());
    }
    let file_len = file.metadata()?.len();
    let read_len = file_len.min(MAX_LOG_TAIL_BYTES);
    file.seek(SeekFrom::End(-i64::try_from(read_len).unwrap_or(i64::MAX)))?;
    let mut suffix = vec![0; usize::try_from(read_len).unwrap_or(MAX_LOG_TAIL_BYTES as usize)];
    file.read_exact(&mut suffix)?;
    let contents = String::from_utf8_lossy(&suffix);
    let bounded = discard_partial_first_line(&contents, file_len > read_len);
    Ok(tail_lines(bounded, lines))
}

fn discard_partial_first_line(contents: &str, truncated: bool) -> &str {
    if !truncated {
        return contents;
    }
    contents
        .find('\n')
        .map(|newline| &contents[newline + 1..])
        .unwrap_or(contents)
}

fn tail_lines(contents: &str, lines: usize) -> String {
    if lines == 0 || contents.is_empty() {
        return String::new();
    }
    let all_lines = contents.lines().collect::<Vec<_>>();
    let start = all_lines.len().saturating_sub(lines);
    let mut tail = all_lines[start..].join("\n");
    if !tail.is_empty() && contents.ends_with('\n') {
        tail.push('\n');
    }
    tail
}
