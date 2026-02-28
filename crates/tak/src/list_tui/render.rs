
use std::collections::{BTreeMap, BTreeSet};

use anyhow::Result;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use tak_core::model::{TaskLabel, WorkspaceSpec};

const LIST_TASK: &str = "\x1b[1;36m";
const LIST_DEP: &str = "\x1b[1;33m";
const LIST_PUNC: &str = "\x1b[2;37m";
const TREE_TITLE: &str = "\x1b[1;35m";
const TREE_DIM: &str = "\x1b[2;37m";
const RESET: &str = "\x1b[0m";

const TREE_MIN_WIDTH: u16 = 70;
const TREE_MIN_HEIGHT: u16 = 10;

pub(crate) fn render_list(spec: &WorkspaceSpec) -> String {
    let mut lines = Vec::with_capacity(spec.tasks.len().max(1));

    for (label, task) in &spec.tasks {
        let task_name = display_label(label);
        if task.deps.is_empty() {
            lines.push(format!("{LIST_TASK}{task_name}{RESET}"));
            continue;
        }

        let deps = task
            .deps
            .iter()
            .map(|dep| format!("{LIST_DEP}{}{RESET}", display_label(dep)))
            .collect::<Vec<_>>()
            .join(&format!("{LIST_PUNC}, {RESET}"));

        lines.push(format!(
            "{LIST_TASK}{task_name}{RESET} {LIST_PUNC}[{RESET}{deps}{LIST_PUNC}]{RESET}"
        ));
    }

    if lines.is_empty() {
        lines.push(format!("{LIST_PUNC}(no tasks){RESET}"));
    }

    format!("{}\n", lines.join("\n"))
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

fn display_label(label: &TaskLabel) -> String {
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
