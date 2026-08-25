use std::path::PathBuf;
use std::sync::Mutex;

use tak_make::{
    GoalExecutionFuture, GoalExecutionRequest, GoalExecutor, MakeExecutionPlan, MakeRunOutcome,
    RunMake, RunMakeRequest,
};

use crate::fixtures::RecordingMakefileReader;

struct RecordingPlanExecutor {
    plan: Mutex<Option<MakeExecutionPlan>>,
}

impl RecordingPlanExecutor {
    fn new() -> Self {
        Self {
            plan: Mutex::new(None),
        }
    }

    fn take_plan(&self) -> MakeExecutionPlan {
        self.plan
            .lock()
            .expect("executor plan lock")
            .take()
            .expect("executor plan")
    }
}

impl GoalExecutor for RecordingPlanExecutor {
    fn execute(&self, _request: GoalExecutionRequest) -> GoalExecutionFuture<'_> {
        panic!("parallel Make execution must use the plan port")
    }

    fn execute_plan(&self, plan: MakeExecutionPlan) -> GoalExecutionFuture<'_> {
        self.plan.lock().expect("executor plan lock").replace(plan);
        Box::pin(async { Ok(MakeRunOutcome { exit_code: 0 }) })
    }
}

pub(crate) async fn plan_source(source: &str, goal: &str) -> MakeExecutionPlan {
    let reader = RecordingMakefileReader::new("Makefile", source);
    let executor = RecordingPlanExecutor::new();
    RunMake::new(&reader, &executor)
        .execute(RunMakeRequest {
            workspace_root: PathBuf::from("/workspace"),
            goal: goal.to_string(),
        })
        .await
        .expect("parallel Make plan");
    executor.take_plan()
}
