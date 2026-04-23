use std::fmt;

use monty::MontyObject;
use serde::de;

#[derive(Debug, Clone)]
pub(super) struct MontyDeserializeError(pub(super) String);

impl MontyDeserializeError {
    pub(super) fn message(message: impl Into<String>) -> Self {
        Self(message.into())
    }

    pub(super) fn unsupported_value(value: &MontyObject) -> Self {
        Self(format!(
            "unsupported Monty runtime value: {}",
            runtime_value_kind(value)
        ))
    }

    pub(super) fn invalid_dict_key(key: &MontyObject) -> Self {
        Self(format!(
            "Monty dict keys must be strings, got {}",
            runtime_value_kind(key)
        ))
    }
}

impl fmt::Display for MontyDeserializeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for MontyDeserializeError {}

impl de::Error for MontyDeserializeError {
    fn custom<T>(msg: T) -> Self
    where
        T: fmt::Display,
    {
        Self(msg.to_string())
    }
}

fn runtime_value_kind(value: &MontyObject) -> &'static str {
    match value {
        MontyObject::Ellipsis => "ellipsis",
        MontyObject::None => "none",
        MontyObject::Bool(_) => "bool",
        MontyObject::Int(_) => "int",
        MontyObject::BigInt(_) => "bigint",
        MontyObject::Float(_) => "float",
        MontyObject::String(_) => "string",
        MontyObject::Bytes(_) => "bytes",
        MontyObject::List(_) => "list",
        MontyObject::Tuple(_) => "tuple",
        MontyObject::NamedTuple { .. } => "namedtuple",
        MontyObject::Dict(_) => "dict",
        MontyObject::Set(_) => "set",
        MontyObject::FrozenSet(_) => "frozenset",
        MontyObject::Date(_) => "date",
        MontyObject::DateTime(_) => "datetime",
        MontyObject::TimeDelta(_) => "timedelta",
        MontyObject::TimeZone(_) => "timezone",
        MontyObject::Exception { .. } => "exception",
        MontyObject::Type(_) => "type",
        MontyObject::BuiltinFunction(_) => "builtin_function",
        MontyObject::Path(_) => "path",
        MontyObject::Dataclass { .. } => "dataclass",
        MontyObject::Function { .. } => "function",
        MontyObject::Repr(_) => "repr",
        MontyObject::Cycle(_, _) => "cycle",
    }
}
