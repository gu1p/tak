use std::collections::BTreeMap;

use anyhow::{Context, Result, bail};
use prost::Message;
use tak_core::model::{
    ContainerRuntimeSourceSpec, NeedDef, OutputSelectorSpec, RemoteRuntimeSpec, ResolvedTask,
    Scope, StepDef, TaskLabel,
};
use tak_proto::{
    CmdStep, ContainerResourceLimits, ContainerRuntime, ExecutionSession, GetTaskResultResponse,
    PollTaskEventsResponse, RuntimeSpec, ScriptStep, Step, SubmitTaskRequest, SubmittedNeed,
    runtime_spec, step,
};

use crate::{
    OutputStream, ParsedRemoteEvents, RemoteLogChunk, RemoteStatusUpdate, RemoteWorkspaceStage,
    StrictRemoteTarget, SyncedOutput, TaskStatusEventKind,
    task_run_metadata::task_run_metadata_for_runtime,
};

mod events_parser;
mod fused_members;
mod outputs_parser;
mod submit_payload;

pub(crate) use events_parser::parse_remote_events_response;
pub(crate) use outputs_parser::parse_remote_result_outputs;
pub(crate) use submit_payload::{RemoteSubmitPayloadInput, build_remote_submit_payload};

#[cfg(test)]
mod events_parser_queue_tests;
#[cfg(test)]
mod events_parser_tests;
#[cfg(test)]
mod submit_payload_behavior_tests;
#[cfg(test)]
mod submit_payload_error_tests;
#[cfg(test)]
mod submit_payload_fused_tests;
#[cfg(test)]
mod submit_payload_test_support;
