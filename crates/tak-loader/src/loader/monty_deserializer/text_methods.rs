use monty::MontyObject;
use serde::de::Visitor;

use super::{MontyDeserializer, error::MontyDeserializeError};

pub(super) fn deserialize_char<'de, V>(
    de: MontyDeserializer<'de>,
    visitor: V,
) -> Result<V::Value, MontyDeserializeError>
where
    V: Visitor<'de>,
{
    match de.value {
        MontyObject::String(value) => {
            let mut chars = value.chars();
            let Some(ch) = chars.next() else {
                return Err(MontyDeserializeError::message("Monty string is empty"));
            };
            if chars.next().is_some() {
                return Err(MontyDeserializeError::message(
                    "Monty string must contain exactly one character",
                ));
            }
            visitor.visit_char(ch)
        }
        other => Err(MontyDeserializeError::unsupported_value(other)),
    }
}

macro_rules! visit_stringish {
    ($name:ident, $visit:ident, $pattern:pat => $value:expr) => {
        pub(super) fn $name<'de, V>(
            de: MontyDeserializer<'de>,
            visitor: V,
        ) -> Result<V::Value, MontyDeserializeError>
        where
            V: Visitor<'de>,
        {
            match de.value {
                $pattern => visitor.$visit($value),
                other => Err(MontyDeserializeError::unsupported_value(other)),
            }
        }
    };
}

visit_stringish!(deserialize_str, visit_str, MontyObject::String(value) => value);
visit_stringish!(deserialize_string, visit_string, MontyObject::String(value) => value.clone());
visit_stringish!(deserialize_bytes, visit_bytes, MontyObject::Bytes(value) => value);
visit_stringish!(deserialize_byte_buf, visit_byte_buf, MontyObject::Bytes(value) => value.clone());

pub(super) fn deserialize_option<'de, V>(
    de: MontyDeserializer<'de>,
    visitor: V,
) -> Result<V::Value, MontyDeserializeError>
where
    V: Visitor<'de>,
{
    match de.value {
        MontyObject::None => visitor.visit_none(),
        _ => visitor.visit_some(de),
    }
}

pub(super) fn deserialize_unit<'de, V>(
    de: MontyDeserializer<'de>,
    visitor: V,
) -> Result<V::Value, MontyDeserializeError>
where
    V: Visitor<'de>,
{
    match de.value {
        MontyObject::None => visitor.visit_unit(),
        other => Err(MontyDeserializeError::unsupported_value(other)),
    }
}

pub(super) fn deserialize_unit_struct<'de, V>(
    de: MontyDeserializer<'de>,
    _name: &'static str,
    visitor: V,
) -> Result<V::Value, MontyDeserializeError>
where
    V: Visitor<'de>,
{
    deserialize_unit(de, visitor)
}

pub(super) fn deserialize_newtype_struct<'de, V>(
    de: MontyDeserializer<'de>,
    _name: &'static str,
    visitor: V,
) -> Result<V::Value, MontyDeserializeError>
where
    V: Visitor<'de>,
{
    visitor.visit_newtype_struct(de)
}
