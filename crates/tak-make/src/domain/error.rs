/// Invalid or unsupported Tak metadata in a Makefile.
#[derive(Debug, thiserror::Error)]
pub enum MakefileParseError {
    /// The requested literal goal was not declared.
    #[error("Make goal `{goal}` was not found as a literal single-target rule")]
    GoalNotFound {
        /// Requested goal.
        goal: String,
    },
    /// An annotation block was attached to unsupported Make syntax.
    #[error(
        "Tak annotations require a literal single-target rule at line {line}; got `{declaration}`"
    )]
    UnsupportedAnnotatedRule {
        /// One-based source line.
        line: usize,
        /// Unsupported declaration.
        declaration: String,
    },
    /// An annotation did not use `key=value` syntax.
    #[error("malformed Tak annotation at line {line}: expected `# tak: key=value`")]
    MalformedAnnotation {
        /// One-based source line.
        line: usize,
    },
    /// An annotation key is not part of the supported surface.
    #[error("unknown Tak annotation `{key}` at line {line}")]
    UnknownAnnotation {
        /// One-based source line.
        line: usize,
        /// Unsupported key.
        key: String,
    },
    /// A key appeared twice in one annotation block.
    #[error("duplicate Tak annotation `{key}` at line {line}")]
    DuplicateAnnotation {
        /// One-based source line.
        line: usize,
        /// Repeated key.
        key: String,
    },
    /// The execution value was neither local nor remote.
    #[error("invalid `execution` value `{value}` at line {line}; expected `local` or `remote`")]
    InvalidExecution {
        /// One-based source line.
        line: usize,
        /// Invalid value.
        value: String,
    },
    /// Image and Dockerfile sources cannot both be selected.
    #[error("Tak annotations `container-image` and `container-dockerfile` are mutually exclusive")]
    ConflictingContainerSources,
    /// A build context only has meaning with a Dockerfile.
    #[error("Tak annotation `container-build-context` requires `container-dockerfile`")]
    BuildContextWithoutDockerfile,
    /// Repeated declarations selected different Tak metadata for one goal.
    #[error("Make goal `{goal}` has conflicting Tak annotations across repeated rules")]
    ConflictingGoalAnnotations {
        /// Repeated goal with incompatible metadata.
        goal: String,
    },
    /// File defaults cannot define a target graph.
    #[error("Tak annotation `default.parallel` is not supported at line {line}")]
    DefaultParallel {
        /// One-based source line.
        line: usize,
    },
    /// A parallel member list is malformed or too small.
    #[error("invalid `parallel` value at line {line}: {reason}")]
    InvalidParallel {
        /// One-based source line.
        line: usize,
        /// Human-readable violated invariant.
        reason: String,
    },
    /// A parallel output mode is unknown.
    #[error(
        "invalid `parallel-output` value `{value}` at line {line}; expected `live` or `grouped`"
    )]
    InvalidParallelOutput {
        /// One-based source line.
        line: usize,
        /// Invalid value.
        value: String,
    },
    /// Every Tak-managed parallel goal must be explicitly phony.
    #[error("parallel Make target `{goal}` must be declared in `.PHONY`")]
    ParallelTargetNotPhony {
        /// Goal missing from `.PHONY`.
        goal: String,
    },
    /// An annotated member is not a direct prerequisite of its group.
    #[error("parallel member `{member}` must be a direct prerequisite of `{goal}`")]
    ParallelMemberNotDirect {
        /// Annotated aggregate goal.
        goal: String,
        /// Invalid member.
        member: String,
    },
    /// A recursively expanded group refers back to an ancestor.
    #[error("parallel Make graph contains a cycle at `{goal}`")]
    ParallelCycle {
        /// Revisited goal.
        goal: String,
    },
    /// A shared goal inherited incompatible settings through two parents.
    #[error("parallel Make target `{goal}` inherits conflicting Tak annotations")]
    ConflictingParallelAnnotations {
        /// Shared goal.
        goal: String,
    },
    /// Dynamic prerequisite syntax cannot be safely split into independent Make runs.
    #[error("parallel Make target `{goal}` requires literal prerequisites on one line")]
    UnsupportedParallelPrerequisites {
        /// Annotated goal.
        goal: String,
    },
}
