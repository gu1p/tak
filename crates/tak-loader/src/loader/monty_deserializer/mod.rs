use anyhow::{Result, anyhow};
use monty::MontyObject;
use serde::de::{self, DeserializeOwned, Visitor};

mod access;
mod compound_methods;
mod error;
mod helpers;
mod numeric_methods;
mod text_methods;

use error::MontyDeserializeError;

pub(crate) fn deserialize_from_monty<T>(value: MontyObject) -> Result<T>
where
    T: DeserializeOwned,
{
    <T as serde::Deserialize>::deserialize(MontyDeserializer { value: &value })
        .map_err(|err| anyhow!(err.to_string()))
}

struct MontyDeserializer<'de> {
    value: &'de MontyObject,
}

macro_rules! delegate_deserialize {
    ($name:ident => $target:path) => {
        fn $name<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de>,
        {
            $target(self, visitor)
        }
    };
    ($name:ident($($arg:ident: $ty:ty),+ $(,)?) => $target:path) => {
        fn $name<V>(self, $($arg: $ty,)+ visitor: V) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de>,
        {
            $target(self, $($arg,)+ visitor)
        }
    };
}

impl<'de> de::Deserializer<'de> for MontyDeserializer<'de> {
    type Error = MontyDeserializeError;

    delegate_deserialize!(deserialize_any => numeric_methods::deserialize_any);
    delegate_deserialize!(deserialize_bool => numeric_methods::deserialize_bool);
    delegate_deserialize!(deserialize_i8 => numeric_methods::deserialize_i8);
    delegate_deserialize!(deserialize_i16 => numeric_methods::deserialize_i16);
    delegate_deserialize!(deserialize_i32 => numeric_methods::deserialize_i32);
    delegate_deserialize!(deserialize_i64 => numeric_methods::deserialize_i64);
    delegate_deserialize!(deserialize_u8 => numeric_methods::deserialize_u8);
    delegate_deserialize!(deserialize_u16 => numeric_methods::deserialize_u16);
    delegate_deserialize!(deserialize_u32 => numeric_methods::deserialize_u32);
    delegate_deserialize!(deserialize_u64 => numeric_methods::deserialize_u64);
    delegate_deserialize!(deserialize_f32 => numeric_methods::deserialize_f32);
    delegate_deserialize!(deserialize_f64 => numeric_methods::deserialize_f64);
    delegate_deserialize!(deserialize_char => text_methods::deserialize_char);
    delegate_deserialize!(deserialize_str => text_methods::deserialize_str);
    delegate_deserialize!(deserialize_string => text_methods::deserialize_string);
    delegate_deserialize!(deserialize_bytes => text_methods::deserialize_bytes);
    delegate_deserialize!(deserialize_byte_buf => text_methods::deserialize_byte_buf);
    delegate_deserialize!(deserialize_option => text_methods::deserialize_option);
    delegate_deserialize!(deserialize_unit => text_methods::deserialize_unit);
    delegate_deserialize!(deserialize_unit_struct(name: &'static str) => text_methods::deserialize_unit_struct);
    delegate_deserialize!(deserialize_newtype_struct(name: &'static str) => text_methods::deserialize_newtype_struct);
    delegate_deserialize!(deserialize_seq => compound_methods::deserialize_seq);
    delegate_deserialize!(deserialize_tuple(len: usize) => compound_methods::deserialize_tuple);
    delegate_deserialize!(deserialize_tuple_struct(name: &'static str, len: usize) => compound_methods::deserialize_tuple_struct);
    delegate_deserialize!(deserialize_map => compound_methods::deserialize_map);
    delegate_deserialize!(deserialize_struct(name: &'static str, fields: &'static [&'static str]) => compound_methods::deserialize_struct);
    delegate_deserialize!(deserialize_enum(name: &'static str, variants: &'static [&'static str]) => compound_methods::deserialize_enum);
    delegate_deserialize!(deserialize_identifier => compound_methods::deserialize_identifier);
    delegate_deserialize!(deserialize_ignored_any => compound_methods::deserialize_ignored_any);
}
