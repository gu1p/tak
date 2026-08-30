use std::fmt::{Debug, Formatter};

use serde::{Deserialize, Serialize};

use super::ResolvedRunError;
use crate::v2::PassEnv;

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnvironmentValue {
    pub name: String,
    pub value: String,
}

impl EnvironmentValue {
    pub fn new(
        name: impl Into<String>,
        value: impl Into<String>,
    ) -> Result<Self, ResolvedRunError> {
        let name = name.into();
        PassEnv::new([name.as_str()]).map_err(|error| ResolvedRunError::new(error.to_string()))?;
        Ok(Self {
            name,
            value: value.into(),
        })
    }
}

impl Debug for EnvironmentValue {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("EnvironmentValue")
            .field("name", &self.name)
            .field("value", &"<redacted>")
            .finish()
    }
}
