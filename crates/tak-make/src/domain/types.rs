/// Where Tak should place one Make invocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionPlacement {
    /// Execute on the local Tak client.
    Local,
    /// Execute on a configured remote Tak agent.
    Remote,
}

/// Container source selected by Makefile annotations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContainerSource {
    /// Use an existing container image.
    Image {
        /// Image reference passed to Tak.
        image: String,
    },
    /// Build a container image from a Dockerfile.
    Dockerfile {
        /// Workspace-relative Dockerfile path.
        dockerfile: String,
        /// Optional workspace-relative build context.
        build_context: Option<String>,
    },
}

/// How output from concurrently executing Make goals is presented.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ParallelOutputMode {
    /// Write each logical line as soon as it arrives.
    #[default]
    Live,
    /// Hold each goal's output until that goal completes.
    Grouped,
}

/// Resolved Tak execution metadata after file defaults and goal overrides are merged.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GoalAnnotations {
    /// Optional local or remote placement.
    pub placement: Option<ExecutionPlacement>,
    /// Optional container source.
    pub container: Option<ContainerSource>,
}
