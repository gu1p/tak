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
