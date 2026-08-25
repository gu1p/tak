use super::annotations::Annotation;
use super::{ExecutionPlacement, MakefileParseError, ParallelOutputMode};

pub(super) fn parse_parallel(
    annotation: Option<&Annotation>,
) -> Result<Option<Vec<String>>, MakefileParseError> {
    let Some(annotation) = annotation else {
        return Ok(None);
    };
    let members = annotation
        .value
        .split(',')
        .map(str::trim)
        .map(str::to_string)
        .collect::<Vec<_>>();
    if members.len() < 2 || members.iter().any(String::is_empty) {
        return Err(MakefileParseError::InvalidParallel {
            line: annotation.line,
            reason: "expected at least two comma-separated targets".to_string(),
        });
    }
    let unique = members.iter().collect::<std::collections::BTreeSet<_>>();
    if unique.len() != members.len() {
        return Err(MakefileParseError::InvalidParallel {
            line: annotation.line,
            reason: "duplicate targets are not allowed".to_string(),
        });
    }
    Ok(Some(members))
}

pub(super) fn parse_parallel_output(
    annotation: Option<&Annotation>,
) -> Result<Option<ParallelOutputMode>, MakefileParseError> {
    let Some(annotation) = annotation else {
        return Ok(None);
    };
    match annotation.value.as_str() {
        "live" => Ok(Some(ParallelOutputMode::Live)),
        "grouped" => Ok(Some(ParallelOutputMode::Grouped)),
        value => Err(MakefileParseError::InvalidParallelOutput {
            line: annotation.line,
            value: value.to_string(),
        }),
    }
}

pub(super) fn parse_placement(
    annotation: Option<&Annotation>,
) -> Result<Option<ExecutionPlacement>, MakefileParseError> {
    let Some(annotation) = annotation else {
        return Ok(None);
    };
    match annotation.value.as_str() {
        "local" => Ok(Some(ExecutionPlacement::Local)),
        "remote" => Ok(Some(ExecutionPlacement::Remote)),
        value => Err(MakefileParseError::InvalidExecution {
            line: annotation.line,
            value: value.to_string(),
        }),
    }
}
