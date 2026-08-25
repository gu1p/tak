pub(super) struct ParsedRule<'a> {
    pub(super) target: &'a str,
    pub(super) literal_single_target: bool,
    pub(super) prerequisites: Vec<String>,
    pub(super) prerequisites_supported: bool,
}

impl<'a> ParsedRule<'a> {
    pub(super) fn parse(line: &'a str) -> Option<Self> {
        let (left, right) = line.split_once(':')?;
        let target = left.trim();
        let unsupported = target.is_empty()
            || target.split_whitespace().count() != 1
            || target.contains(['%', '$', '\\'])
            || right.starts_with(':')
            || is_variable_assignment(right)
            || is_static_pattern_rule(right);
        let prerequisite_source = right.split_once(';').map_or(right, |(head, _)| head);
        let prerequisites_supported = !prerequisite_source.contains(['%', '$', '\\']);
        let prerequisites = prerequisite_source
            .split_whitespace()
            .filter(|value| *value != "|")
            .map(str::to_string)
            .collect();
        Some(Self {
            target,
            literal_single_target: !unsupported,
            prerequisites,
            prerequisites_supported,
        })
    }
}

fn is_variable_assignment(rule_tail: &str) -> bool {
    [":::=", "::=", ":=", "?=", "+=", "!=", "="]
        .into_iter()
        .any(|operator| {
            rule_tail
                .trim_start()
                .split_once(operator)
                .is_some_and(|(left, _)| is_variable_declaration(left))
        })
}

fn is_variable_declaration(left: &str) -> bool {
    let mut words = left.split_whitespace().peekable();
    while words
        .peek()
        .is_some_and(|word| matches!(*word, "export" | "private" | "override" | "unexport"))
    {
        words.next();
    }
    words.next().is_some() && words.next().is_none()
}

fn is_static_pattern_rule(rule_tail: &str) -> bool {
    rule_tail
        .split_once(':')
        .is_some_and(|(target_pattern, _)| target_pattern.contains('%'))
}
