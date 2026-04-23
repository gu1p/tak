use monty::MontyObject;
use serde::de::Visitor;

use super::{
    MontyDeserializer,
    access::{MontyMapAccess, MontySeqAccess},
    error::MontyDeserializeError,
};

pub(super) fn deserialize_any<'de, V>(
    de: MontyDeserializer<'de>,
    visitor: V,
) -> Result<V::Value, MontyDeserializeError>
where
    V: Visitor<'de>,
{
    match de.value {
        MontyObject::None => visitor.visit_unit(),
        MontyObject::Bool(value) => visitor.visit_bool(*value),
        MontyObject::Int(value) => visitor.visit_i64(*value),
        MontyObject::BigInt(_) => visitor.visit_i64(de.deserialize_i64_value()?),
        MontyObject::Float(_) => visitor.visit_f64(de.deserialize_f64_value()?),
        MontyObject::String(value) => visitor.visit_str(value),
        MontyObject::List(values) => visitor.visit_seq(MontySeqAccess {
            iter: values.iter(),
        }),
        MontyObject::Dict(_) | MontyObject::NamedTuple { .. } => {
            visitor.visit_map(MontyMapAccess {
                entries: de.map_entries()?,
                index: 0,
            })
        }
        other => Err(MontyDeserializeError::unsupported_value(other)),
    }
}

pub(super) fn deserialize_bool<'de, V>(
    de: MontyDeserializer<'de>,
    visitor: V,
) -> Result<V::Value, MontyDeserializeError>
where
    V: Visitor<'de>,
{
    match de.value {
        MontyObject::Bool(value) => visitor.visit_bool(*value),
        other => Err(MontyDeserializeError::unsupported_value(other)),
    }
}

macro_rules! visit_int {
    ($name:ident, $visit:ident, $value:ident, $message:literal) => {
        pub(super) fn $name<'de, V>(
            de: MontyDeserializer<'de>,
            visitor: V,
        ) -> Result<V::Value, MontyDeserializeError>
        where
            V: Visitor<'de>,
        {
            visitor.$visit(
                de.$value()?
                    .try_into()
                    .map_err(|_| MontyDeserializeError::message($message))?,
            )
        }
    };
}

visit_int!(
    deserialize_i8,
    visit_i8,
    deserialize_i64_value,
    "Monty int is out of i8 range"
);
visit_int!(
    deserialize_i16,
    visit_i16,
    deserialize_i64_value,
    "Monty int is out of i16 range"
);
visit_int!(
    deserialize_i32,
    visit_i32,
    deserialize_i64_value,
    "Monty int is out of i32 range"
);
visit_int!(
    deserialize_u8,
    visit_u8,
    deserialize_u64_value,
    "Monty int is out of u8 range"
);
visit_int!(
    deserialize_u16,
    visit_u16,
    deserialize_u64_value,
    "Monty int is out of u16 range"
);
visit_int!(
    deserialize_u32,
    visit_u32,
    deserialize_u64_value,
    "Monty int is out of u32 range"
);

pub(super) fn deserialize_i64<'de, V>(
    de: MontyDeserializer<'de>,
    visitor: V,
) -> Result<V::Value, MontyDeserializeError>
where
    V: Visitor<'de>,
{
    visitor.visit_i64(de.deserialize_i64_value()?)
}

pub(super) fn deserialize_u64<'de, V>(
    de: MontyDeserializer<'de>,
    visitor: V,
) -> Result<V::Value, MontyDeserializeError>
where
    V: Visitor<'de>,
{
    visitor.visit_u64(de.deserialize_u64_value()?)
}

pub(super) fn deserialize_f32<'de, V>(
    de: MontyDeserializer<'de>,
    visitor: V,
) -> Result<V::Value, MontyDeserializeError>
where
    V: Visitor<'de>,
{
    visitor.visit_f32(de.deserialize_f64_value()? as f32)
}

pub(super) fn deserialize_f64<'de, V>(
    de: MontyDeserializer<'de>,
    visitor: V,
) -> Result<V::Value, MontyDeserializeError>
where
    V: Visitor<'de>,
{
    visitor.visit_f64(de.deserialize_f64_value()?)
}
