mod authored_root;
mod authored_source;
mod load_options;
mod monty_deserializer;
mod v2_includes;
mod v2_module_eval;
mod v2_wire;
mod v2_wire_conversion;
mod v2_wire_primitives;
mod v2_workspace_view;
mod workspace_discovery;
mod workspace_load_and_policy_eval;

pub use authored_root::{V2AuthoredRoot, inspect_authored_root_module};
pub use load_options::LoadOptions;
pub use workspace_discovery::detect_workspace_root;
pub use workspace_load_and_policy_eval::load_workspace;

const TASKS_FILE: &str = "TASKS.py";
const PRELUDE_V2: &str = include_str!("prelude_v2.py");
const DSL_STUBS_V2: &str = include_str!("dsl_stubs_v2.pyi");
