use monty::MontyObject;
use serde::de::{IntoDeserializer, Visitor};

use super::{
    MontyDeserializer,
    access::{MontyEnumDeserializer, MontyMapAccess, MontySeqAccess},
    error::MontyDeserializeError,
};

pub(super) fn deserialize_seq<'de, V>(
    de: MontyDeserializer<'de>,
    visitor: V,
) -> Result<V::Value, MontyDeserializeError>
where
    V: Visitor<'de>,
{
    visitor.visit_seq(MontySeqAccess {
        iter: de.list_values()?.iter(),
    })
}

pub(super) fn deserialize_tuple<'de, V>(
    de: MontyDeserializer<'de>,
    _len: usize,
    visitor: V,
) -> Result<V::Value, MontyDeserializeError>
where
    V: Visitor<'de>,
{
    deserialize_seq(de, visitor)
}

pub(super) fn deserialize_tuple_struct<'de, V>(
    de: MontyDeserializer<'de>,
    _name: &'static str,
    _len: usize,
    visitor: V,
) -> Result<V::Value, MontyDeserializeError>
where
    V: Visitor<'de>,
{
    deserialize_seq(de, visitor)
}

pub(super) fn deserialize_map<'de, V>(
    de: MontyDeserializer<'de>,
    visitor: V,
) -> Result<V::Value, MontyDeserializeError>
where
    V: Visitor<'de>,
{
    visitor.visit_map(MontyMapAccess {
        entries: de.map_entries()?,
        index: 0,
    })
}

pub(super) fn deserialize_struct<'de, V>(
    de: MontyDeserializer<'de>,
    _name: &'static str,
    _fields: &'static [&'static str],
    visitor: V,
) -> Result<V::Value, MontyDeserializeError>
where
    V: Visitor<'de>,
{
    deserialize_map(de, visitor)
}

pub(super) fn deserialize_enum<'de, V>(
    de: MontyDeserializer<'de>,
    _name: &'static str,
    _variants: &'static [&'static str],
    visitor: V,
) -> Result<V::Value, MontyDeserializeError>
where
    V: Visitor<'de>,
{
    match de.value {
        MontyObject::String(value) => visitor.visit_enum(value.as_str().into_deserializer()),
        MontyObject::Dict(_) => {
            let value = de.value;
            let entries = de.map_entries()?;
            if entries.len() != 1 {
                return Err(MontyDeserializeError::unsupported_value(value));
            }
            let (variant, value) = entries[0];
            visitor.visit_enum(MontyEnumDeserializer {
                variant,
                value: Some(value),
            })
        }
        other => Err(MontyDeserializeError::unsupported_value(other)),
    }
}

pub(super) fn deserialize_identifier<'de, V>(
    de: MontyDeserializer<'de>,
    visitor: V,
) -> Result<V::Value, MontyDeserializeError>
where
    V: Visitor<'de>,
{
    super::text_methods::deserialize_str(de, visitor)
}

pub(super) fn deserialize_ignored_any<'de, V>(
    _de: MontyDeserializer<'de>,
    visitor: V,
) -> Result<V::Value, MontyDeserializeError>
where
    V: Visitor<'de>,
{
    visitor.visit_unit()
}
