use std::any::TypeId;

use tak_core::model;

macro_rules! assert_public_types {
    ($($ty:ty),+ $(,)?) => {
        $(
            let _ = TypeId::of::<$ty>();
        )+
    };
}

#[test]
fn model_facade_reexports_public_types() {
    assert_public_types!(
        model::TaskLabel,
        model::Scope,
        model::LimiterRef,
        model::ModuleSpec,
        model::Defaults,
        model::TaskDef,
        model::PathInputDef,
        model::IgnoreSourceDef,
        model::CurrentStateDef,
        model::OutputSelectorDef,
        model::LocalDef,
        model::RemoteDef,
        model::RemoteSelectionDef,
        model::RemoteTransportDef,
        model::RemoteTransportKind,
        model::RemoteRuntimeDef,
        model::ContainerMountDef,
        model::ContainerResourceLimitsDef,
        model::ContainerImageReference,
        model::ContainerImageReferenceError,
        model::ContainerMountSpec,
        model::ContainerResourceLimitsSpec,
        model::ContainerRuntimeSourceInputSpec,
        model::ContainerRuntimeExecutionSpec,
        model::ContainerRuntimeExecutionSpecError,
        model::PolicyDecisionModeDef,
        model::PolicyDecisionDef,
        model::ExecutionPolicyDef,
        model::TaskExecutionDef,
        model::StepDef,
        model::Hold,
        model::NeedDef,
        model::QueueUseDef,
        model::LimiterDef,
        model::QueueDef,
        model::QueueDiscipline,
        model::RetryDef,
        model::BackoffDef,
        model::LimiterKey,
        model::ResolvedTask,
        model::LocalSpec,
        model::RemoteSpec,
        model::ContainerRuntimeSourceSpec,
        model::RemoteRuntimeSpec,
        model::PolicyDecisionSpec,
        model::ExecutionPlacementSpec,
        model::ExecutionPolicySpec,
        model::TaskExecutionSpec,
        model::RemoteSelectionSpec,
        model::IgnoreSourceSpec,
        model::CurrentStateOrigin,
        model::CurrentStateSpec,
        model::OutputSelectorSpec,
        model::WorkspaceSpec,
        model::PathAnchor,
        model::PathRef,
        model::ContextManifest,
        model::PathNormalizationError
    );
}
