use serde::Deserialize;

use super::Output;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Context {
    pub(crate) roots: Vec<Output>,
    pub(crate) ignored: Vec<Ignore>,
    pub(crate) include: Vec<Output>,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum Ignore {
    Path { value: String },
    Gitignore,
}
