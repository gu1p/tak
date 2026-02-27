//! CLI render helpers for `tak list` and `tak tree`.

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
            .map(|dep| format!("{LIST_DEP}{}{RESET}", display_dep_label(label, dep)))
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
    let package = label.package.trim_start_matches("//");
    if package.is_empty() {
        return label.name.clone();
    }
    format!("{}:{}", package.replace('/', "."), label.name)
}

fn display_dep_label(task_label: &TaskLabel, dep_label: &TaskLabel) -> String {
    if dep_label.package == task_label.package {
        return dep_label.name.clone();
    }
    display_label(dep_label)
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

struct TreeWalker<'a> {
    children: &'a BTreeMap<TaskLabel, Vec<TaskLabel>>,
    seen: BTreeSet<TaskLabel>,
    lines: Vec<String>,
}

impl<'a> TreeWalker<'a> {
    fn new(children: &'a BTreeMap<TaskLabel, Vec<TaskLabel>>) -> Self {
        Self {
            children,
            seen: BTreeSet::new(),
            lines: Vec::new(),
        }
    }

    fn walk(&mut self, node: &TaskLabel, prefix: &str, is_last: bool, is_root: bool) {
        let branch = if is_root {
            ""
        } else if is_last {
            "└─ "
        } else {
            "├─ "
        };
        self.lines
            .push(format!("{prefix}{branch}{}", display_label(node)));

        if !self.seen.insert(node.clone()) {
            self.lines.push(format!("{prefix}↳ (already shown)"));
            return;
        }

        let Some(node_children) = self.children.get(node) else {
            return;
        };

        let child_prefix = if is_root {
            prefix.to_string()
        } else if is_last {
            format!("{prefix}   ")
        } else {
            format!("{prefix}│  ")
        };

        for (index, child) in node_children.iter().enumerate() {
            let child_is_last = index + 1 == node_children.len();
            if self.seen.contains(child) {
                let branch = if child_is_last { "└─ " } else { "├─ " };
                let continuation = if child_is_last { "   " } else { "│  " };
                self.lines
                    .push(format!("{child_prefix}{branch}{}", display_label(child)));
                self.lines
                    .push(format!("{child_prefix}{continuation}↳ (already shown)"));
                continue;
            }
            self.walk(child, &child_prefix, child_is_last, false);
        }
    }
}

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
