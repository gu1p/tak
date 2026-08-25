//! Makefile goal discovery and execution orchestration for Tak.
//!
//! The application module coordinates two injected ports: a Makefile reader and a
//! goal executor. Parsing and annotation validation remain pure domain behavior,
//! including file-wide defaults and goal-specific overrides, while filesystem access
//! stays in an outer adapter.

mod adapters;
mod application;
mod domain;

pub use adapters::FilesystemMakefileReader;
pub use application::{
    GoalExecutionFuture, GoalExecutionRequest, GoalExecutor, MakeExecutionPlan, MakeGoalExecution,
    MakeRunOutcome, MakefileReadError, MakefileReader, MakefileSource, RunMake, RunMakeError,
    RunMakeRequest,
};
pub use domain::{
    ContainerSource, ExecutionPlacement, GoalAnnotations, MakefileParseError, ParallelOutputMode,
};

extern crate self as tak_make;
