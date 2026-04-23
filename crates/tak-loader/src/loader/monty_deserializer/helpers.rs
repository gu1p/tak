use monty::{DictPairs, MontyObject};

use super::{MontyDeserializer, error::MontyDeserializeError};

impl<'de> MontyDeserializer<'de> {
    pub(super) fn deserialize_i64_value(self) -> Result<i64, MontyDeserializeError> {
        match self.value {
            MontyObject::Int(value) => Ok(*value),
            MontyObject::BigInt(value) => value
                .to_string()
                .parse::<i64>()
                .map_err(|_| MontyDeserializeError::message("Monty bigint is out of i64 range")),
            other => Err(MontyDeserializeError::unsupported_value(other)),
        }
    }

    pub(super) fn deserialize_u64_value(self) -> Result<u64, MontyDeserializeError> {
        match self.value {
            MontyObject::Int(value) => (*value)
                .try_into()
                .map_err(|_| MontyDeserializeError::message("Monty int is out of u64 range")),
            MontyObject::BigInt(value) => value
                .to_string()
                .parse::<u64>()
                .map_err(|_| MontyDeserializeError::message("Monty bigint is out of u64 range")),
            other => Err(MontyDeserializeError::unsupported_value(other)),
        }
    }

    pub(super) fn deserialize_f64_value(self) -> Result<f64, MontyDeserializeError> {
        match self.value {
            MontyObject::Int(value) => Ok(*value as f64),
            MontyObject::BigInt(value) => value
                .to_string()
                .parse::<f64>()
                .map_err(|_| MontyDeserializeError::message("Monty bigint is out of f64 range")),
            MontyObject::Float(value) if value.is_finite() => Ok(*value),
            MontyObject::Float(_) => {
                Err(MontyDeserializeError::message("Monty float must be finite"))
            }
            other => Err(MontyDeserializeError::unsupported_value(other)),
        }
    }

    pub(super) fn list_values(self) -> Result<&'de [MontyObject], MontyDeserializeError> {
        match self.value {
            MontyObject::List(values) => Ok(values),
            other => Err(MontyDeserializeError::unsupported_value(other)),
        }
    }

    pub(super) fn map_entries(
        self,
    ) -> Result<Vec<(&'de str, &'de MontyObject)>, MontyDeserializeError> {
        match self.value {
            MontyObject::Dict(pairs) => dict_entries(pairs),
            MontyObject::NamedTuple {
                field_names,
                values,
                ..
            } => Ok(field_names
                .iter()
                .map(String::as_str)
                .zip(values.iter())
                .collect()),
            other => Err(MontyDeserializeError::unsupported_value(other)),
        }
    }
}

fn dict_entries(pairs: &DictPairs) -> Result<Vec<(&str, &MontyObject)>, MontyDeserializeError> {
    let mut entries = Vec::new();
    for (key, value) in pairs {
        let MontyObject::String(key) = key else {
            return Err(MontyDeserializeError::invalid_dict_key(key));
        };
        entries.push((key.as_str(), value));
    }
    Ok(entries)
}
