use std::collections::{BTreeMap, BTreeSet};

use super::MakefileParseError;
use super::annotations::{
    Annotation, AnnotationSettings, ParsedAnnotation, parse_annotation, resolve_annotation_block,
    resolve_annotations,
};
use super::make_rule::ParsedRule;

#[derive(Clone)]
pub(super) struct MakeRule {
    pub(super) prerequisites: Vec<String>,
    pub(super) annotations: AnnotationSettings,
    pub(super) parallel_prerequisites_supported: bool,
    authored: bool,
}

pub(super) struct MakefileModel {
    pub(super) defaults: AnnotationSettings,
    pub(super) rules: BTreeMap<String, MakeRule>,
    pub(super) phony: BTreeSet<String>,
}

impl MakefileModel {
    pub(super) fn parse(source: &str) -> Result<Self, MakefileParseError> {
        ModelParser::default().parse(source)
    }

    pub(super) fn required_goal(&self, goal: &str) -> Result<&MakeRule, MakefileParseError> {
        self.rules
            .get(goal)
            .ok_or_else(|| MakefileParseError::GoalNotFound {
                goal: goal.to_string(),
            })
    }
}

#[derive(Default)]
struct ModelParser {
    defaults: Vec<Annotation>,
    pending: Vec<Annotation>,
    authored_blocks: Vec<AnnotationSettings>,
    rules: BTreeMap<String, MakeRule>,
    phony: BTreeSet<String>,
}

impl ModelParser {
    fn parse(mut self, source: &str) -> Result<MakefileModel, MakefileParseError> {
        for (index, line) in source.lines().enumerate() {
            self.parse_line(line, index + 1)?;
        }
        let defaults = resolve_annotation_block(&self.defaults)?;
        resolve_annotations(&defaults, AnnotationSettings::default())?;
        for annotations in &self.authored_blocks {
            resolve_annotations(&defaults, annotations.clone())?;
        }
        Ok(MakefileModel {
            defaults,
            rules: self.rules,
            phony: self.phony,
        })
    }

    fn parse_line(&mut self, line: &str, line_number: usize) -> Result<(), MakefileParseError> {
        let trimmed = line.trim();
        if line.starts_with('\t') {
            self.pending.clear();
            return Ok(());
        }
        if let Some(annotation) = parse_annotation(trimmed, line_number)? {
            match annotation {
                ParsedAnnotation::Default(value) => self.defaults.push(value),
                ParsedAnnotation::Goal(value) => self.pending.push(value),
            }
            return Ok(());
        }
        if trimmed.is_empty() || trimmed.starts_with('#') {
            self.pending.clear();
            return Ok(());
        }
        self.parse_declaration(trimmed, line_number)
    }

    fn parse_declaration(
        &mut self,
        declaration: &str,
        line: usize,
    ) -> Result<(), MakefileParseError> {
        let Some(rule) = ParsedRule::parse(declaration) else {
            return self.reject_pending(line, declaration);
        };
        if !rule.literal_single_target {
            self.reject_pending(line, declaration)?;
            self.pending.clear();
            return Ok(());
        }
        if rule.target == ".PHONY" {
            self.phony.extend(rule.prerequisites.iter().cloned());
        }
        let authored = !self.pending.is_empty();
        let annotations = resolve_annotation_block(&self.pending)?;
        if authored {
            self.authored_blocks.push(annotations.clone());
        }
        self.merge_rule(rule, annotations, authored)?;
        self.pending.clear();
        Ok(())
    }

    fn merge_rule(
        &mut self,
        parsed: ParsedRule<'_>,
        annotations: AnnotationSettings,
        authored: bool,
    ) -> Result<(), MakefileParseError> {
        let Some(current) = self.rules.get_mut(parsed.target) else {
            self.rules.insert(
                parsed.target.to_string(),
                MakeRule {
                    prerequisites: parsed.prerequisites,
                    annotations,
                    parallel_prerequisites_supported: parsed.prerequisites_supported,
                    authored,
                },
            );
            return Ok(());
        };
        if authored && current.authored && current.annotations != annotations {
            return Err(MakefileParseError::ConflictingGoalAnnotations {
                goal: parsed.target.to_string(),
            });
        }
        if authored {
            current.annotations = annotations;
            current.authored = true;
        }
        current.prerequisites.extend(parsed.prerequisites);
        current.parallel_prerequisites_supported &= parsed.prerequisites_supported;
        Ok(())
    }

    fn reject_pending(&self, line: usize, declaration: &str) -> Result<(), MakefileParseError> {
        if self.pending.is_empty() {
            return Ok(());
        }
        Err(MakefileParseError::UnsupportedAnnotatedRule {
            line,
            declaration: declaration.to_string(),
        })
    }
}
