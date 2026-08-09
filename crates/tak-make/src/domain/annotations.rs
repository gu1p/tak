use std::collections::BTreeMap;

use super::{ContainerSource, ExecutionPlacement, GoalAnnotations, MakefileParseError};

pub(super) struct Annotation {
    line: usize,
    key: String,
    value: String,
}

pub(super) enum ParsedAnnotation {
    Default(Annotation),
    Goal(Annotation),
}

#[derive(Clone, Default, PartialEq, Eq)]
pub(super) struct AnnotationSettings {
    placement: Option<ExecutionPlacement>,
    image: Option<String>,
    dockerfile: Option<String>,
    build_context: Option<String>,
}

pub(super) fn parse_annotation(
    line: &str,
    line_number: usize,
) -> Result<Option<ParsedAnnotation>, MakefileParseError> {
    let Some(body) = line.strip_prefix("# tak:") else {
        return Ok(None);
    };
    let Some((key, value)) = body.trim().split_once('=') else {
        return Err(MakefileParseError::MalformedAnnotation { line: line_number });
    };
    let key = key.trim();
    let value = value.trim();
    if key.is_empty() || value.is_empty() {
        return Err(MakefileParseError::MalformedAnnotation { line: line_number });
    }
    let (is_default, key) = match key.strip_prefix("default.") {
        Some("") => {
            return Err(MakefileParseError::MalformedAnnotation { line: line_number });
        }
        Some(key) => (true, key),
        None => (false, key),
    };
    let annotation = Annotation {
        line: line_number,
        key: key.to_string(),
        value: value.to_string(),
    };
    Ok(Some(if is_default {
        ParsedAnnotation::Default(annotation)
    } else {
        ParsedAnnotation::Goal(annotation)
    }))
}

pub(super) fn resolve_annotation_block(
    entries: &[Annotation],
) -> Result<AnnotationSettings, MakefileParseError> {
    let mut values = BTreeMap::new();
    for entry in entries {
        if values.insert(entry.key.as_str(), entry).is_some() {
            return Err(MakefileParseError::DuplicateAnnotation {
                line: entry.line,
                key: entry.key.clone(),
            });
        }
    }
    validate_keys(&values)?;
    let placement = parse_placement(values.get("execution").copied())?;
    let image = value(&values, "container-image");
    let dockerfile = value(&values, "container-dockerfile");
    let build_context = value(&values, "container-build-context");
    if image.is_some() && dockerfile.is_some() {
        return Err(MakefileParseError::ConflictingContainerSources);
    }
    Ok(AnnotationSettings {
        placement,
        image,
        dockerfile,
        build_context,
    })
}

pub(super) fn resolve_annotations(
    defaults: &AnnotationSettings,
    goal: AnnotationSettings,
) -> Result<GoalAnnotations, MakefileParseError> {
    let placement = goal.placement.or(defaults.placement);
    let (image, dockerfile, build_context) = merge_container_settings(defaults, goal);
    let container = parse_container(image, dockerfile, build_context)?;
    Ok(GoalAnnotations {
        placement,
        container,
    })
}

fn merge_container_settings(
    defaults: &AnnotationSettings,
    goal: AnnotationSettings,
) -> (Option<String>, Option<String>, Option<String>) {
    if goal.image.is_some() {
        return (goal.image, None, goal.build_context);
    }
    let build_context = goal
        .build_context
        .or_else(|| defaults.build_context.clone());
    if goal.dockerfile.is_some() {
        return (None, goal.dockerfile, build_context);
    }
    (
        defaults.image.clone(),
        defaults.dockerfile.clone(),
        build_context,
    )
}

fn value(values: &BTreeMap<&str, &Annotation>, key: &str) -> Option<String> {
    values.get(key).map(|entry| entry.value.clone())
}

fn validate_keys(values: &BTreeMap<&str, &Annotation>) -> Result<(), MakefileParseError> {
    for (key, annotation) in values {
        if !matches!(
            *key,
            "execution" | "container-image" | "container-dockerfile" | "container-build-context"
        ) {
            return Err(MakefileParseError::UnknownAnnotation {
                line: annotation.line,
                key: (*key).to_string(),
            });
        }
    }
    Ok(())
}

fn parse_placement(
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

fn parse_container(
    image: Option<String>,
    dockerfile: Option<String>,
    build_context: Option<String>,
) -> Result<Option<ContainerSource>, MakefileParseError> {
    match (image, dockerfile, build_context) {
        (Some(_), Some(_), _) => Err(MakefileParseError::ConflictingContainerSources),
        (Some(image), None, None) => Ok(Some(ContainerSource::Image { image })),
        (None, Some(dockerfile), build_context) => Ok(Some(ContainerSource::Dockerfile {
            dockerfile,
            build_context,
        })),
        (None, None, Some(_)) | (Some(_), None, Some(_)) => {
            Err(MakefileParseError::BuildContextWithoutDockerfile)
        }
        (None, None, None) => Ok(None),
    }
}
