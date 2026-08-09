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

/// Tak execution metadata attached to a literal Make goal.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GoalAnnotations {
    /// Optional local or remote placement.
    pub placement: Option<ExecutionPlacement>,
    /// Optional container source.
    pub container: Option<ContainerSource>,
}
