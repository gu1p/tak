use serde::{Deserialize, Deserializer, Serialize};

use super::ValidationError;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct PassEnv(Vec<String>);

impl PassEnv {
    pub fn new<I, S>(names: I) -> Result<Self, ValidationError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut names = names
            .into_iter()
            .map(|name| name.as_ref().to_owned())
            .collect::<Vec<_>>();
        if let Some(name) = names.iter().find(|name| !is_portable_name(name)) {
            return Err(ValidationError::InvalidEnvironmentName(name.clone()));
        }
        names.sort();
        names.dedup();
        Ok(Self(names))
    }

    #[must_use]
    pub fn as_strs(&self) -> Vec<&str> {
        self.0.iter().map(String::as_str).collect()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl<'de> Deserialize<'de> for PassEnv {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let names = Vec::<String>::deserialize(deserializer)?;
        Self::new(names).map_err(serde::de::Error::custom)
    }
}

fn is_portable_name(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first == '_' || first.is_ascii_alphabetic())
        && chars.all(|character| character == '_' || character.is_ascii_alphanumeric())
}
