pub fn extract_block<'a>(stdout: &'a str, title: &str) -> Vec<&'a str> {
    let lines = stdout.lines().collect::<Vec<_>>();
    let start = lines
        .iter()
        .position(|line| line.starts_with('┌') && line.contains(&format!("┌{title}")))
        .expect("block title should exist");
    let end = lines[start + 1..]
        .iter()
        .position(|line| line.starts_with('└'))
        .map(|offset| start + 1 + offset)
        .expect("block should have a closing border");
    lines[start..=end].to_vec()
}

pub fn qr_block_body_height(block: &[&str]) -> usize {
    block.len().saturating_sub(2)
}

pub fn visible_text(block: &[&str]) -> String {
    block
        .iter()
        .skip(1)
        .take(block.len().saturating_sub(2))
        .flat_map(|line| {
            line.strip_prefix('│')
                .unwrap_or(line)
                .strip_suffix('│')
                .unwrap_or(line)
                .chars()
        })
        .filter(|ch| !ch.is_whitespace())
        .collect()
}
