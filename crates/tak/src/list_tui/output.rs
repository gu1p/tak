fn buffer_to_plain_text(buffer: &Buffer) -> String {
    let area = buffer.area;
    let mut lines = Vec::with_capacity(area.height as usize);

    for y in area.y..(area.y + area.height) {
        let mut line = String::with_capacity(area.width as usize);
        for x in area.x..(area.x + area.width) {
            let cell = &buffer[(x, y)];
            let symbol = cell.symbol();
            if symbol.is_empty() {
                line.push(' ');
            } else {
                line.push_str(symbol);
            }
        }
        lines.push(line.trim_end().to_string());
    }

    while lines.last().is_some_and(|line| line.is_empty()) {
        lines.pop();
    }

    lines.join("\n")
}

fn colorize_tree_output(raw: &str) -> String {
    let mut output = String::new();
    for line in raw.lines() {
        if line.contains("Tak Tree") {
            output.push_str(TREE_TITLE);
            output.push_str(line);
            output.push_str(RESET);
            output.push('\n');
            continue;
        }

        if line.contains("(already shown)") {
            output.push_str(&line.replace(
                "(already shown)",
                &format!("{TREE_DIM}(already shown){RESET}"),
            ));
            output.push('\n');
            continue;
        }

        output.push_str(line);
        output.push('\n');
    }

    output
}
