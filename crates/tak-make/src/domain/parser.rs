use super::annotations::{
    Annotation, AnnotationSettings, ParsedAnnotation, parse_annotation, resolve_annotation_block,
    resolve_annotations,
};
use super::{GoalAnnotations, MakefileParseError};

struct SelectedGoal {
    annotations: AnnotationSettings,
    authored: bool,
}

pub(crate) fn annotations_for_goal(
    source: &str,
    requested_goal: &str,
) -> Result<GoalAnnotations, MakefileParseError> {
    let mut defaults = Vec::<Annotation>::new();
    let mut pending = Vec::<Annotation>::new();
    let mut authored_blocks = Vec::<AnnotationSettings>::new();
    let mut selected = None;

    for (index, line) in source.lines().enumerate() {
        let line_number = index + 1;
        let trimmed = line.trim();
        if line.starts_with('\t') {
            pending.clear();
            continue;
        }
        if let Some(annotation) = parse_annotation(trimmed, line_number)? {
            match annotation {
                ParsedAnnotation::Default(annotation) => {
                    defaults.push(annotation);
                }
                ParsedAnnotation::Goal(annotation) => pending.push(annotation),
            }
            continue;
        }
        if trimmed.is_empty() || trimmed.starts_with('#') {
            pending.clear();
            continue;
        }

        let Some(rule) = parse_rule(trimmed) else {
            reject_pending_annotations(&pending, line_number, trimmed)?;
            continue;
        };
        if !rule.literal_single_target {
            reject_pending_annotations(&pending, line_number, trimmed)?;
            pending.clear();
            continue;
        }

        let authored = !pending.is_empty();
        let annotations = resolve_annotation_block(&pending)?;
        if authored {
            authored_blocks.push(annotations.clone());
        }
        if rule.target == requested_goal {
            merge_selected_goal(&mut selected, annotations, authored, requested_goal)?;
        }
        pending.clear();
    }

    let defaults = resolve_annotation_block(&defaults)?;
    resolve_annotations(&defaults, AnnotationSettings::default())?;
    for annotations in authored_blocks {
        resolve_annotations(&defaults, annotations)?;
    }
    let goal = selected.ok_or_else(|| MakefileParseError::GoalNotFound {
        goal: requested_goal.to_string(),
    })?;
    resolve_annotations(&defaults, goal.annotations)
}

fn merge_selected_goal(
    selected: &mut Option<SelectedGoal>,
    annotations: AnnotationSettings,
    authored: bool,
    goal: &str,
) -> Result<(), MakefileParseError> {
    let Some(current) = selected else {
        *selected = Some(SelectedGoal {
            annotations,
            authored,
        });
        return Ok(());
    };
    if !authored {
        return Ok(());
    }
    if current.authored && current.annotations != annotations {
        return Err(MakefileParseError::ConflictingGoalAnnotations {
            goal: goal.to_string(),
        });
    }
    current.annotations = annotations;
    current.authored = true;
    Ok(())
}

fn reject_pending_annotations(
    pending: &[Annotation],
    line: usize,
    declaration: &str,
) -> Result<(), MakefileParseError> {
    if pending.is_empty() {
        return Ok(());
    }
    Err(MakefileParseError::UnsupportedAnnotatedRule {
        line,
        declaration: declaration.to_string(),
    })
}

struct ParsedRule<'a> {
    target: &'a str,
    literal_single_target: bool,
}

fn parse_rule(line: &str) -> Option<ParsedRule<'_>> {
    let (left, right) = line.split_once(':')?;
    let target = left.trim();
    let unsupported = target.is_empty()
        || target.split_whitespace().count() != 1
        || target.contains(['%', '$', '\\'])
        || right.starts_with(':')
        || is_variable_assignment(right)
        || is_static_pattern_rule(right);
    Some(ParsedRule {
        target,
        literal_single_target: !unsupported,
    })
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
