#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContainerImageReference {
    pub canonical: String,
    pub digest_pinned: bool,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ContainerImageReferenceError {
    #[error("container image reference cannot be empty")]
    EmptyReference,
    #[error("container image reference cannot contain whitespace")]
    ContainsWhitespace,
    #[error("container image reference must not include a URL scheme")]
    ContainsScheme,
    #[error("container image digest must be `<algorithm>:<hex>`")]
    MalformedDigest,
    #[error("container image digest algorithm cannot be empty")]
    EmptyDigestAlgorithm,
    #[error("container image digest cannot be empty")]
    EmptyDigest,
    #[error("container image digest must contain only hexadecimal characters")]
    NonHexDigest,
    #[error("container image sha256 digest must be exactly 64 hexadecimal characters")]
    InvalidSha256DigestLength,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ContainerMountSpec {
    pub source: String,
    pub target: String,
    pub read_only: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ContainerResourceLimitsSpec {
    pub cpu_cores: Option<f64>,
    pub memory_mb: Option<u64>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ContainerRuntimeSourceInputSpec {
    Image { image: String },
    Dockerfile {
        dockerfile: PathInputDef,
        build_context: Option<PathInputDef>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct ContainerRuntimeExecutionSpec {
    pub source: ContainerRuntimeSourceInputSpec,
    pub command: Vec<String>,
    pub mounts: Vec<ContainerMountSpec>,
    pub env: BTreeMap<String, String>,
    pub resource_limits: Option<ContainerResourceLimitsSpec>,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ContainerRuntimeExecutionSpecError {
    #[error("runtime.kind `{kind}` is unsupported; expected `containerized`")]
    UnsupportedKind { kind: String },
    #[error("runtime must specify exactly one source: `image` or `dockerfile`")]
    MissingSource,
    #[error("runtime must not specify both `image` and `dockerfile`")]
    MultipleSources,
    #[error("runtime.image {0}")]
    InvalidImage(ContainerImageReferenceError),
    #[error("runtime.dockerfile is required for dockerfile runtime")]
    MissingDockerfile,
    #[error("runtime.dockerfile path must be declared with path(...)")]
    InvalidDockerfilePathType,
    #[error("runtime.build_context path must be declared with path(...)")]
    InvalidBuildContextPathType,
    #[error("runtime.command cannot be empty when provided")]
    EmptyCommand,
    #[error("runtime.command[{index}] cannot be empty")]
    EmptyCommandArg { index: usize },
    #[error("runtime.mounts[{index}].source cannot be empty")]
    EmptyMountSource { index: usize },
    #[error("runtime.mounts[{index}].target must be an absolute path without `..`, got `{target}`")]
    InvalidMountTarget { index: usize, target: String },
    #[error("runtime.env key `{key}` is invalid; expected `[A-Z_][A-Z0-9_]*`")]
    InvalidEnvKey { key: String },
    #[error("runtime.env key `{key}` is reserved for Tak runtime internals")]
    ReservedEnvKey { key: String },
    #[error("runtime.env value for `{key}` cannot contain null bytes: {value_preview}")]
    InvalidEnvValue { key: String, value_preview: String },
    #[error("runtime.resource_limits.cpu_cores must be > 0 and <= 256")]
    InvalidCpuCores,
    #[error("runtime.resource_limits.memory_mb must be > 0")]
    InvalidMemoryMb,
}
