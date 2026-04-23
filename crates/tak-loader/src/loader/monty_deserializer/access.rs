use serde::de::{
    self, DeserializeSeed, EnumAccess, IntoDeserializer, MapAccess, SeqAccess, VariantAccess,
    Visitor,
};

use super::{MontyDeserializer, error::MontyDeserializeError};

pub(super) struct MontySeqAccess<'de> {
    pub(super) iter: std::slice::Iter<'de, monty::MontyObject>,
}

impl<'de> SeqAccess<'de> for MontySeqAccess<'de> {
    type Error = MontyDeserializeError;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        let Some(value) = self.iter.next() else {
            return Ok(None);
        };
        seed.deserialize(MontyDeserializer { value }).map(Some)
    }
}

pub(super) struct MontyMapAccess<'de> {
    pub(super) entries: Vec<(&'de str, &'de monty::MontyObject)>,
    pub(super) index: usize,
}

impl<'de> MapAccess<'de> for MontyMapAccess<'de> {
    type Error = MontyDeserializeError;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: DeserializeSeed<'de>,
    {
        let Some((key, _)) = self.entries.get(self.index).copied() else {
            return Ok(None);
        };
        seed.deserialize(key.into_deserializer()).map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: DeserializeSeed<'de>,
    {
        let Some((_, value)) = self.entries.get(self.index).copied() else {
            return Err(MontyDeserializeError::message(
                "missing Monty map value during deserialization",
            ));
        };
        self.index += 1;
        seed.deserialize(MontyDeserializer { value })
    }
}

pub(super) struct MontyEnumDeserializer<'de> {
    pub(super) variant: &'de str,
    pub(super) value: Option<&'de monty::MontyObject>,
}

impl<'de> EnumAccess<'de> for MontyEnumDeserializer<'de> {
    type Error = MontyDeserializeError;
    type Variant = MontyVariantDeserializer<'de>;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
    where
        V: DeserializeSeed<'de>,
    {
        let variant = seed.deserialize(self.variant.into_deserializer())?;
        Ok((variant, MontyVariantDeserializer { value: self.value }))
    }
}

pub(super) struct MontyVariantDeserializer<'de> {
    pub(super) value: Option<&'de monty::MontyObject>,
}

impl<'de> VariantAccess<'de> for MontyVariantDeserializer<'de> {
    type Error = MontyDeserializeError;

    fn unit_variant(self) -> Result<(), Self::Error> {
        match self.value {
            None | Some(monty::MontyObject::None) => Ok(()),
            Some(other) => Err(MontyDeserializeError::unsupported_value(other)),
        }
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        let value = self
            .value
            .ok_or_else(|| MontyDeserializeError::message("missing Monty enum payload"))?;
        seed.deserialize(MontyDeserializer { value })
    }

    fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let value = self
            .value
            .ok_or_else(|| MontyDeserializeError::message("missing Monty enum payload"))?;
        de::Deserializer::deserialize_tuple(MontyDeserializer { value }, len, visitor)
    }

    fn struct_variant<V>(
        self,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let value = self
            .value
            .ok_or_else(|| MontyDeserializeError::message("missing Monty enum payload"))?;
        de::Deserializer::deserialize_struct(MontyDeserializer { value }, "", fields, visitor)
    }
}
