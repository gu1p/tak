use super::annotations::{resolve_annotations, resolve_goal_annotations};
use super::makefile_model::MakefileModel;
use super::parallel_plan::ParallelPlanBuilder;
use super::{GoalAnnotations, MakefileParseError, ParallelOutputMode};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MakeGoalPlan {
    pub(crate) goal: String,
    pub(crate) argv: Vec<String>,
    pub(crate) annotations: GoalAnnotations,
    pub(crate) dependencies: Vec<String>,
    pub(crate) parallel_output: ParallelOutputMode,
}

pub(crate) fn annotations_for_goal(
    source: &str,
    requested_goal: &str,
) -> Result<GoalAnnotations, MakefileParseError> {
    let model = validated_model(source)?;
    let goal = model.required_goal(requested_goal)?;
    resolve_annotations(&model.defaults, goal.annotations.clone())
}

pub(crate) fn parallel_plan_for_goal(
    source: &str,
    requested_goal: &str,
) -> Result<Option<Vec<MakeGoalPlan>>, MakefileParseError> {
    let model = validated_model(source)?;
    let root = model.required_goal(requested_goal)?;
    if root.annotations.parallel.is_none() {
        return Ok(None);
    }
    let root_settings =
        super::annotations::inherit_annotations(&model.defaults, &root.annotations)?;
    resolve_goal_annotations(&root_settings)?;
    ParallelPlanBuilder::new(&model)
        .build(requested_goal, root_settings)
        .map(Some)
}

fn validated_model(source: &str) -> Result<MakefileModel, MakefileParseError> {
    let model = MakefileModel::parse(source)?;
    for (goal, rule) in &model.rules {
        if rule.annotations.parallel.is_none() {
            continue;
        }
        let settings = super::annotations::inherit_annotations(&model.defaults, &rule.annotations)?;
        ParallelPlanBuilder::new(&model).build(goal, settings)?;
    }
    Ok(model)
}
