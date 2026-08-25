use std::collections::{BTreeMap, BTreeSet};

use super::annotations::{AnnotationSettings, inherit_annotations, resolve_goal_annotations};
use super::makefile_model::MakefileModel;
use super::parser::MakeGoalPlan;
use super::{GoalAnnotations, MakefileParseError, ParallelOutputMode};

#[derive(PartialEq, Eq)]
struct EffectiveGoal {
    annotations: GoalAnnotations,
    output: ParallelOutputMode,
}

pub(super) struct ParallelPlanBuilder<'a> {
    model: &'a MakefileModel,
    visiting: BTreeSet<String>,
    planned: BTreeMap<String, EffectiveGoal>,
    goals: Vec<MakeGoalPlan>,
}

impl<'a> ParallelPlanBuilder<'a> {
    pub(super) fn new(model: &'a MakefileModel) -> Self {
        Self {
            model,
            visiting: BTreeSet::new(),
            planned: BTreeMap::new(),
            goals: Vec::new(),
        }
    }

    pub(super) fn build(
        mut self,
        root: &str,
        settings: AnnotationSettings,
    ) -> Result<Vec<MakeGoalPlan>, MakefileParseError> {
        self.visit(root, settings)?;
        Ok(self.goals)
    }

    fn visit(
        &mut self,
        goal: &str,
        settings: AnnotationSettings,
    ) -> Result<(), MakefileParseError> {
        if !self.visiting.insert(goal.to_string()) {
            return Err(MakefileParseError::ParallelCycle {
                goal: goal.to_string(),
            });
        }
        let result = self.visit_unchecked(goal, settings);
        self.visiting.remove(goal);
        result
    }

    fn visit_unchecked(
        &mut self,
        goal: &str,
        settings: AnnotationSettings,
    ) -> Result<(), MakefileParseError> {
        self.require_phony(goal)?;
        let effective = EffectiveGoal {
            annotations: resolve_goal_annotations(&settings)?,
            output: settings.parallel_output.unwrap_or_default(),
        };
        if let Some(previous) = self.planned.get(goal) {
            if previous == &effective {
                return Ok(());
            }
            return Err(MakefileParseError::ConflictingParallelAnnotations {
                goal: goal.to_string(),
            });
        }
        let rule = self.model.required_goal(goal)?.clone();
        let dependencies = rule.annotations.parallel.clone().unwrap_or_default();
        self.validate_group(
            goal,
            &rule.prerequisites,
            rule.parallel_prerequisites_supported,
            &dependencies,
        )?;
        self.planned.insert(goal.to_string(), effective);

        for child in &dependencies {
            let child_rule = self.model.required_goal(child)?;
            let child_settings = inherit_annotations(&settings, &child_rule.annotations)?;
            self.visit(child, child_settings)?;
        }
        self.goals.push(MakeGoalPlan {
            goal: goal.to_string(),
            argv: make_argv(goal, &dependencies),
            annotations: resolve_goal_annotations(&settings)?,
            dependencies,
            parallel_output: settings.parallel_output.unwrap_or_default(),
        });
        Ok(())
    }

    fn validate_group(
        &self,
        goal: &str,
        prerequisites: &[String],
        syntax_supported: bool,
        dependencies: &[String],
    ) -> Result<(), MakefileParseError> {
        if dependencies.is_empty() {
            return Ok(());
        }
        if !syntax_supported {
            return Err(MakefileParseError::UnsupportedParallelPrerequisites {
                goal: goal.to_string(),
            });
        }
        for child in dependencies {
            if !prerequisites.contains(child) {
                return Err(MakefileParseError::ParallelMemberNotDirect {
                    goal: goal.to_string(),
                    member: child.clone(),
                });
            }
            self.require_phony(child)?;
        }
        Ok(())
    }

    fn require_phony(&self, goal: &str) -> Result<(), MakefileParseError> {
        if self.model.phony.contains(goal) {
            return Ok(());
        }
        Err(MakefileParseError::ParallelTargetNotPhony {
            goal: goal.to_string(),
        })
    }
}

fn make_argv(goal: &str, dependencies: &[String]) -> Vec<String> {
    let mut argv = vec!["make".to_string()];
    argv.extend(
        dependencies
            .iter()
            .map(|child| format!("--assume-old={child}")),
    );
    argv.push(goal.to_string());
    argv
}
