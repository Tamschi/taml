use crate::parser::{ListIter, MapIter, MapKey};
use {
    crate::{parser::Taml, token::Token},
    serde::de,
};

struct Deserializer<'a, 'de>(&'a Taml<'de>);

pub type Error = de::value::Error;
pub type Result<T> = std::result::Result<T, Error>;

#[allow(clippy::missing_errors_doc)]
pub fn from_str<T: de::DeserializeOwned>(str: &str) -> Result<T> {
    use logos::Logos as _;
    let lexer = Token::lexer(str);
    from_tokens(lexer)
}

#[allow(clippy::missing_errors_doc)]
pub fn from_tokens<'de, T: de::Deserialize<'de>>(
    tokens: impl IntoIterator<Item = Token<'de>>,
) -> Result<T> {
    //TODO: This seems overly explicit.
    use std::iter::FromIterator as _;
    let taml =
        std::result::Result::<crate::parser::Map<'de>, crate::parser::Expected>::from_iter(tokens)
            .map_err(|expected| de::Error::custom(format_args!("Expected {:?}", expected)))?;
    from_taml(&Taml::Map(taml))
}

#[allow(clippy::missing_errors_doc)]
pub fn from_taml<'de, T: de::Deserialize<'de>>(taml: &Taml<'de>) -> Result<T> {
    T::deserialize(Deserializer(&taml))
}

fn invalid_type<'de>(unexp: &'de Taml<'de>, exp: &dyn de::Expected) -> Error {
    de::Error::invalid_type(
        match unexp {
            Taml::String(str) => de::Unexpected::Str(str.as_ref()),
            Taml::Boolean(bool) => de::Unexpected::Bool(*bool),
            Taml::Integer(str) => de::Unexpected::Other("integer"), //TODO
            Taml::Float(str) => str
                .parse()
                .map_or_else(|_| de::Unexpected::Other(str), de::Unexpected::Float),
            Taml::List(_) => de::Unexpected::Seq,
            Taml::Map(_) => de::Unexpected::Map,
        },
        exp,
    )
}

fn invalid_value<'de>(unexp: &'de Taml<'de>, exp: &dyn de::Expected) -> Error {
    de::Error::invalid_type(
        match unexp {
            Taml::String(str) => de::Unexpected::Str(str.as_ref()),
            Taml::Boolean(bool) => de::Unexpected::Bool(*bool),
            Taml::Integer(str) => de::Unexpected::Other(str),
            Taml::Float(str) => str
                .parse()
                .map_or_else(|_| de::Unexpected::Other(str), de::Unexpected::Float),
            Taml::List(_) => de::Unexpected::Seq,
            Taml::Map(_) => de::Unexpected::Map,
        },
        exp,
    )
}

impl<'a, 'de> de::Deserializer<'de> for Deserializer<'a, 'de> {
    type Error = Error;
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        todo!("any {}", std::any::type_name::<V::Value>())
    }
    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        match self.0 {
            Taml::Boolean(bool) => visitor.visit_bool(*bool),
            other => Err(invalid_type(other, &visitor)),
        }
    }
    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        match self.0 {
            Taml::Integer(str) => {
                let value = str.parse().map_err(|_| invalid_value(self.0, &visitor))?;
                visitor.visit_i8(value)
            }
            other => Err(invalid_type(other, &visitor)),
        }
    }
    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        todo!("{}", std::any::type_name::<V::Value>())
    }
    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        todo!("{}", std::any::type_name::<V::Value>())
    }
    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        todo!("{}", std::any::type_name::<V::Value>())
    }
    fn deserialize_i128<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        todo!("{}", std::any::type_name::<V::Value>())
    }
    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        match self.0 {
            Taml::Integer(str) => {
                let value = str.parse().map_err(|_| invalid_value(self.0, &visitor))?;
                visitor.visit_u8(value)
            }
            other => Err(invalid_type(self.0, &visitor)),
        }
    }
    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        todo!("{}", std::any::type_name::<V::Value>())
    }
    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        todo!("{}", std::any::type_name::<V::Value>())
    }
    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        todo!("{}", std::any::type_name::<V::Value>())
    }
    fn deserialize_u128<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        todo!("{}", std::any::type_name::<V::Value>())
    }
    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        todo!("{}", std::any::type_name::<V::Value>())
    }
    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        todo!("{}", std::any::type_name::<V::Value>())
    }
    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        todo!("{}", std::any::type_name::<V::Value>())
    }
    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        todo!("{}", std::any::type_name::<V::Value>())
    }
    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        todo!("{}", std::any::type_name::<V::Value>())
    }
    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        todo!("{}", std::any::type_name::<V::Value>())
    }
    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        todo!("{}", std::any::type_name::<V::Value>())
    }
    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_some(self)
    }
    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        if matches!(self.0, Taml::List(list) if list.is_empty()) {
            visitor.visit_unit()
        } else {
            Err(invalid_type(self.0, &visitor))
        }
    }
    fn deserialize_unit_struct<V>(self, name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        todo!("{}", std::any::type_name::<V::Value>())
    }
    fn deserialize_newtype_struct<V>(self, name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        todo!("{}", std::any::type_name::<V::Value>())
    }
    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        struct ListAccess<'a, 'de>(ListIter<'a, 'de>);

        impl<'a, 'de> de::SeqAccess<'de> for ListAccess<'a, 'de> {
            type Error = Error;

            fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
            where
                T: de::DeserializeSeed<'de>,
            {
                self.0
                    .next()
                    .map(|t| seed.deserialize(Deserializer(t)))
                    .transpose()
            }
        }

        let list = match self.0 {
            Taml::List(list) => list,
            other => return Err(invalid_type(other, &visitor)),
        };

        visitor.visit_seq(ListAccess(list.iter()))
    }
    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        todo!("{}", std::any::type_name::<V::Value>())
    }
    fn deserialize_tuple_struct<V>(
        self,
        name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        todo!("{}", std::any::type_name::<V::Value>())
    }
    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        struct MapAccess<'a, 'de>(MapIter<'a, 'de>, Option<&'a Taml<'de>>);

        impl<'a, 'de> de::MapAccess<'de> for MapAccess<'a, 'de> {
            type Error = Error;

            fn next_key_seed<K: de::DeserializeSeed<'de>>(
                &mut self,
                seed: K,
            ) -> Result<Option<K::Value>> {
                self.0
                    .next()
                    .map(|(k, v)| {
                        self.1 = Some(v);
                        seed.deserialize(KeyDeserializer(k))
                    })
                    .transpose()
            }

            fn next_value_seed<V: de::DeserializeSeed<'de>>(
                &mut self,
                seed: V,
            ) -> Result<V::Value> {
                seed.deserialize(Deserializer(
                    self.1.expect("next_value_seed called before next_key_seed"),
                ))
            }
        }

        struct KeyDeserializer<'a, 'de>(&'a MapKey<'de>);

        impl<'a, 'de> de::Deserializer<'de> for KeyDeserializer<'a, 'de> {
            type Error = Error;

            fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
            where
                V: de::Visitor<'de>,
            {
                dbg!(std::any::type_name::<V::Value>());
                visitor.visit_str(self.0)
            }

            serde::forward_to_deserialize_any! {
                bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
                bytes byte_buf option unit unit_struct newtype_struct seq tuple
                tuple_struct map struct enum identifier ignored_any
            }
        }

        let map = match self.0 {
            Taml::Map(map) => map,
            other => return Err(invalid_type(other, &visitor)),
        };

        visitor.visit_map(MapAccess(map.iter(), None))
    }
    fn deserialize_struct<V>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }
    fn deserialize_enum<V>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        todo!("{}", std::any::type_name::<V::Value>())
    }
    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        todo!("{}", std::any::type_name::<V::Value>())
    }
    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        todo!("{}", std::any::type_name::<V::Value>())
    }
    fn is_human_readable(&self) -> bool {
        true
    }
}
