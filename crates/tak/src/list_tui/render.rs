use super::*;

pub(crate) fn render_list(spec: &WorkspaceSpec) -> String {
    let mut lines = Vec::with_capacity(spec.tasks.len().max(1));

    for (label, task) in &spec.tasks {
        lines.push(render_task_header(label, task));
        lines.extend(render_task_doc_lines(&task.doc));
    }

    if lines.is_empty() {
        lines.push(format!("{LIST_PUNC}(no tasks){RESET}"));
    }

    format!("{}\n", lines.join("\n"))
}

fn render_task_header(label: &TaskLabel, task: &ResolvedTask) -> String {
    let task_name = display_label(label);
    if task.deps.is_empty() {
        return format!("{LIST_TASK}{task_name}{RESET}");
    }

    let deps = task
        .deps
        .iter()
        .map(|dep| format!("{LIST_DEP}{}{RESET}", display_label(dep)))
        .collect::<Vec<_>>()
        .join(&format!("{LIST_PUNC}, {RESET}"));

    format!("{LIST_TASK}{task_name}{RESET} {LIST_PUNC}[{RESET}{deps}{LIST_PUNC}]{RESET}")
}

fn render_task_doc_lines(doc: &str) -> Vec<String> {
    let Some(doc) = doc_block(doc) else {
        return Vec::new();
    };

    doc.lines()
        .map(|line| format!("{LIST_PUNC}  {line}{RESET}"))
        .collect()
}

fn doc_block(doc: &str) -> Option<&str> {
    let trimmed = doc.trim();
    (!trimmed.is_empty()).then_some(trimmed)
}

pub(crate) fn render_tree(spec: &WorkspaceSpec) -> Result<String> {
    let tree_lines = build_tree_lines(spec);
    let longest = tree_lines.iter().map(String::len).max().unwrap_or(12);
    let width = usize::max(TREE_MIN_WIDTH as usize, longest + 4).min(u16::MAX as usize) as u16;
    let height =
        usize::max(TREE_MIN_HEIGHT as usize, tree_lines.len() + 4).min(u16::MAX as usize) as u16;

    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend)?;

    terminal.draw(|frame| {
        let area = frame.area();
        let block = Block::default().borders(Borders::ALL).title(Span::styled(
            " Tak Tree ",
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        ));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let text = tree_lines
            .iter()
            .cloned()
            .map(Line::from)
            .collect::<Vec<_>>();
        let paragraph = Paragraph::new(text).wrap(Wrap { trim: false });
        frame.render_widget(paragraph, inner);
    })?;

    let raw = buffer_to_plain_text(terminal.backend().buffer());
    Ok(colorize_tree_output(&raw))
}

pub(super) fn display_label(label: &TaskLabel) -> String {
    let package = label.package.trim();
    if package.is_empty() || package == "//" || package == "/" {
        return format!("//:{}", label.name);
    }
    let normalized = package
        .strip_prefix("//")
        .map(|rest| format!("//{}", rest.trim_start_matches('/')))
        .unwrap_or_else(|| format!("//{}", package.trim_start_matches('/')));
    format!("{normalized}:{}", label.name)
}

fn build_tree_lines(spec: &WorkspaceSpec) -> Vec<String> {
    let mut children: BTreeMap<TaskLabel, Vec<TaskLabel>> = BTreeMap::new();
    let mut in_degree: BTreeMap<TaskLabel, usize> = BTreeMap::new();

    for label in spec.tasks.keys() {
        in_degree.entry(label.clone()).or_insert(0);
    }

    for (label, task) in &spec.tasks {
        for dep in &task.deps {
            children.entry(dep.clone()).or_default().push(label.clone());
            *in_degree.entry(label.clone()).or_insert(0) += 1;
        }
    }

    for nodes in children.values_mut() {
        nodes.sort();
        nodes.dedup();
    }

    let mut roots: Vec<TaskLabel> = in_degree
        .iter()
        .filter_map(|(label, degree)| (*degree == 0).then_some(label.clone()))
        .collect();
    if roots.is_empty() {
        roots = spec.tasks.keys().cloned().collect();
    }
    roots.sort();

    let mut walker = TreeWalker::new(&children);
    let root_count = roots.len();
    for (index, root) in roots.iter().enumerate() {
        walker.walk(root, "", index + 1 == root_count, true);
    }

    if walker.lines.is_empty() {
        walker.lines.push("(no tasks)".to_string());
    }

    walker.lines
}
