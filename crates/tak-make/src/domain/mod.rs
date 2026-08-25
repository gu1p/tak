mod annotation_values;
mod annotations;
mod error;
mod make_rule;
mod makefile_model;
mod parallel_plan;
mod parser;
mod types;

pub use error::MakefileParseError;
pub use types::{ContainerSource, ExecutionPlacement, GoalAnnotations, ParallelOutputMode};

pub(crate) use parser::{annotations_for_goal, parallel_plan_for_goal};
