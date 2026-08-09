mod annotations;
mod error;
mod parser;
mod types;

pub use error::MakefileParseError;
pub use types::{ContainerSource, ExecutionPlacement, GoalAnnotations};

pub(crate) use parser::annotations_for_goal;
