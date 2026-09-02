use std::path::Path;

const LEGACY_EXECUTION_MESSAGES: &[&str] = &[
    "CmdStep",
    "ScriptStep",
    "Step",
    "ContainerRuntime",
    "RuntimeSpec",
    "FixedRetryBackoff",
    "ExpJitterRetryBackoff",
    "RetryBackoff",
    "RetryPolicy",
    "FusedTaskMember",
    "OutputSelector",
    "SubmitTaskRequest",
    "WorkspaceUploadRef",
    "BeginWorkspaceUploadRequest",
    "BeginWorkspaceUploadResponse",
    "AppendWorkspaceUploadResponse",
    "FinishWorkspaceUploadResponse",
    "StartWorkspaceWormholeUploadRequest",
    "StartWorkspaceWormholeUploadResponse",
    "ExecutionSession",
    "SubmitTaskResponse",
    "OutputFile",
    "GetTaskResultResponse",
    "RemoteFailureKind",
    "CancelTaskResponse",
];

#[test]
fn generated_protocol_is_v2_only_and_has_no_legacy_execution_messages() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let schema = read(root, "proto/takd.proto");
    let generated = read(root, "src/generated.rs");
    let library = read(root, "src/lib.rs");

    assert!(schema.contains("package tak.proto.v2;"));
    assert!(!schema.contains("tak.proto.v1"));
    assert!(generated.contains("tak.proto.v2.rs"));
    assert!(!generated.contains("tak.proto.v1"));
    assert!(library.contains("generated::tak::proto::v2::*"));
    assert!(!library.contains("generated::tak::proto::v1"));
    assert!(!root.join("tests/failure_kind_binary_contract.rs").exists());
    for message in LEGACY_EXECUTION_MESSAGES {
        assert!(
            !schema.contains(&format!("message {message} "))
                && !schema.contains(&format!("enum {message} ")),
            "legacy execution message remains: {message}"
        );
    }
}

fn read(root: &Path, relative: &str) -> String {
    std::fs::read_to_string(root.join(relative)).expect(relative)
}
