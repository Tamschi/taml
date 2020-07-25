use {
    crate::{
        parser::{parse, Diagnostics, Key, List, ListIter, Map, MapIter, Taml},
        token::Token,
    },
    serde::de,
};

struct Deserializer<'a, 'de>(&'a Taml<'de>);

pub type Error = de::value::Error;
pub type Result<T> = std::result::Result<T, Error>;

#[allow(clippy::missing_errors_doc)]
pub fn from_str<T: de::DeserializeOwned>(str: &str) -> (Result<T>, Diagnostics<()>) {
    use logos::Logos as _;
    let lexer = Token::lexer(str);
    from_tokens(lexer)
}

#[allow(clippy::missing_errors_doc)]
pub fn from_tokens<'de, T: de::Deserialize<'de>>(
    tokens: impl IntoIterator<Item = Token<'de>>,
) -> (Result<T>, Diagnostics<()>) {
    //TODO: This seems overly explicit.
    let (root, diagnostics) = parse(tokens);

    (
        root.map_err(|()| de::Error::custom("Pasing error"))
            .and_then(|root| from_taml(&Taml::Map(root))),
        diagnostics,
    )
}

#[allow(clippy::missing_errors_doc)]
pub fn from_taml<'de, T: de::Deserialize<'de>>(taml: &Taml<'de>) -> Result<T> {
    T::deserialize(Deserializer(&taml))
}

fn invalid_type<'de>(unexp: &'de Taml<'de>, exp: &dyn de::Expected) -> Error {
    de::Error::invalid_type(
        match unexp {
            Taml::String(str) => de::Unexpected::Str(str.as_ref()),
            Taml::Integer(str) => de::Unexpected::Other("integer"), //TODO
            Taml::Float(str) => str
                .parse()
                .map_or_else(|_| de::Unexpected::Other(str), de::Unexpected::Float),
            Taml::List(_) => de::Unexpected::Seq,
            Taml::Map(_) => de::Unexpected::Map,
            Taml::StructuredVariant { .. } => de::Unexpected::StructVariant,
            Taml::TupleVariant { values, .. } => match values.len() {
                1 => de::Unexpected::NewtypeVariant,
                _ => de::Unexpected::TupleVariant,
            },
            Taml::UnitVariant { .. } => de::Unexpected::UnitVariant,
        },
        exp,
    )
}

fn invalid_value<'de>(unexp: &'de Taml<'de>, exp: &dyn de::Expected) -> Error {
    de::Error::invalid_value(
        match unexp {
            Taml::String(str) => de::Unexpected::Str(str.as_ref()),
            Taml::Integer(str) => de::Unexpected::Other(str),
            Taml::Float(str) => str
                .parse()
                .map_or_else(|_| de::Unexpected::Other(str), de::Unexpected::Float),
            Taml::List(_) => de::Unexpected::Seq,
            Taml::Map(_) => de::Unexpected::Map,
            Taml::StructuredVariant { .. } => de::Unexpected::StructVariant,
            Taml::TupleVariant { values, .. } => match values.len() {
                1 => de::Unexpected::NewtypeVariant,
                _ => de::Unexpected::TupleVariant,
            },
            Taml::UnitVariant { .. } => de::Unexpected::UnitVariant,
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
            Taml::UnitVariant { variant } if variant.as_ref() == "true" => visitor.visit_bool(true),
            Taml::UnitVariant { variant } if variant.as_ref() == "false" => {
                visitor.visit_bool(false)
            }
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
        match self.0 {
            Taml::Integer(str) => {
                let value = str.parse().map_err(|_| invalid_value(self.0, &visitor))?;
                visitor.visit_i16(value)
            }
            other => Err(invalid_type(other, &visitor)),
        }
    }
    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        match self.0 {
            Taml::Integer(str) => {
                let value = str.parse().map_err(|_| invalid_value(self.0, &visitor))?;
                visitor.visit_i32(value)
            }
            other => Err(invalid_type(other, &visitor)),
        }
    }
    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        match self.0 {
            Taml::Integer(str) => {
                let value = str.parse().map_err(|_| invalid_value(self.0, &visitor))?;
                visitor.visit_i64(value)
            }
            other => Err(invalid_type(other, &visitor)),
        }
    }
    fn deserialize_i128<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        match self.0 {
            Taml::Integer(str) => {
                let value = str.parse().map_err(|_| invalid_value(self.0, &visitor))?;
                visitor.visit_i128(value)
            }
            other => Err(invalid_type(other, &visitor)),
        }
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
        match self.0 {
            Taml::Integer(str) => {
                let value = str.parse().map_err(|_| invalid_value(self.0, &visitor))?;
                visitor.visit_u16(value)
            }
            other => Err(invalid_type(self.0, &visitor)),
        }
    }
    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        match self.0 {
            Taml::Integer(str) => {
                let value = str.parse().map_err(|_| invalid_value(self.0, &visitor))?;
                visitor.visit_u32(value)
            }
            other => Err(invalid_type(self.0, &visitor)),
        }
    }
    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        match self.0 {
            Taml::Integer(str) => {
                let value = str.parse().map_err(|_| invalid_value(self.0, &visitor))?;
                visitor.visit_u64(value)
            }
            other => Err(invalid_type(self.0, &visitor)),
        }
    }
    fn deserialize_u128<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        match self.0 {
            Taml::Integer(str) => {
                let value = str.parse().map_err(|_| invalid_value(self.0, &visitor))?;
                visitor.visit_u128(value)
            }
            other => Err(invalid_type(self.0, &visitor)),
        }
    }
    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        match self.0 {
            Taml::Float(str) => {
                let value = str.parse().map_err(|_| invalid_value(self.0, &visitor))?;
                visitor.visit_f32(value)
            }
            other => Err(invalid_type(self.0, &visitor)),
        }
    }
    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        match self.0 {
            Taml::Float(str) => {
                let value = str.parse().map_err(|_| invalid_value(self.0, &visitor))?;
                visitor.visit_f64(value)
            }
            other => Err(invalid_type(self.0, &visitor)),
        }
    }
    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        match self.0 {
            Taml::String(str) => {
                let value = str.parse().map_err(|_| invalid_value(self.0, &visitor))?;
                visitor.visit_char(value)
            }
            other => Err(invalid_type(self.0, &visitor)),
        }
    }
    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        match self.0 {
            Taml::String(str) => visitor.visit_str(str),
            other => Err(invalid_type(self.0, &visitor)),
        }
    }
    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }
    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        unimplemented!("Byte slices are not supported")
    }
    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        unimplemented!("Byte buffers are not supported")
    }

    /// [`Option`]s are decoded as their contents and always [`Some(...)`] if present at all.
    /// Use [`#[serde(default)]`] to parse a missing field as [`None`].
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
        self.deserialize_unit(visitor)
    }
    fn deserialize_newtype_struct<V>(self, name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }
    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        match self.0 {
            Taml::List(list) => de::Deserializer::deserialize_seq(ListDeserializer(list), visitor),
            other => Err(invalid_type(other, &visitor)),
        }
    }
    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        match self.0 {
            Taml::List(list) if list.len() == len => self.deserialize_seq(visitor),
            Taml::List(list) => Err(de::Error::invalid_length(list.len(), &visitor)),
            _ => Err(invalid_type(self.0, &visitor)),
        }
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
        self.deserialize_tuple(len, visitor)
    }
    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        match self.0 {
            Taml::Map(map) => de::Deserializer::deserialize_map(MapDeserializer(map), visitor),
            other => Err(invalid_type(other, &visitor)),
        }
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
        struct EnumVariantAccess<'a, 'de>(&'a Taml<'de>);

        impl<'a, 'de> de::EnumAccess<'de> for EnumVariantAccess<'a, 'de> {
            type Error = Error;
            type Variant = Self;

            fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
            where
                V: de::DeserializeSeed<'de>,
            {
                Ok((
                    seed.deserialize(KeyDeserializer(match self.0 {
                        Taml::StructuredVariant { variant, .. }
                        | Taml::TupleVariant { variant, .. }
                        | Taml::UnitVariant { variant } => variant,
                        _ => unreachable!(),
                    }))?,
                    self,
                ))
            }
        }

        impl<'a, 'de> de::VariantAccess<'de> for EnumVariantAccess<'a, 'de> {
            type Error = Error;

            fn unit_variant(self) -> Result<()> {
                match self.0 {
                    Taml::UnitVariant { .. } => Ok(()),
                    _ => Err(invalid_type(self.0, &"unit variant")),
                }
            }

            fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
            where
                T: de::DeserializeSeed<'de>,
            {
                match self.0 {
                    Taml::TupleVariant { values, .. } if values.len() == 1 => {
                        seed.deserialize(Deserializer(&values[0]))
                    }
                    Taml::TupleVariant { values, .. } => Err(de::Error::invalid_length(
                        values.len(),
                        &"tuple variant of length 1",
                    )),
                    _ => Err(invalid_type(self.0, &"tuple variant of length 1")),
                }
            }

            fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value>
            where
                V: de::Visitor<'de>,
            {
                match self.0 {
                    Taml::TupleVariant { values, .. } if values.len() == len => {
                        de::Deserializer::deserialize_seq(ListDeserializer(values), visitor)
                    }
                    Taml::TupleVariant { values, .. } => {
                        Err(de::Error::invalid_length(values.len(), &visitor))
                    }
                    _ => Err(invalid_type(self.0, &visitor)),
                }
            }

            fn struct_variant<V>(
                self,
                _fields: &'static [&'static str],
                visitor: V,
            ) -> Result<V::Value>
            where
                V: de::Visitor<'de>,
            {
                match self.0 {
                    Taml::StructuredVariant { fields, .. } => {
                        de::Deserializer::deserialize_map(MapDeserializer(fields), visitor)
                    }
                    _ => Err(invalid_type(self.0, &visitor)),
                }
            }
        }

        match self.0 {
            Taml::StructuredVariant { variant, .. }
            | Taml::TupleVariant { variant, .. }
            | Taml::UnitVariant { variant } => visitor.visit_enum(EnumVariantAccess(self.0)),
            _ => Err(invalid_type(self.0, &visitor)),
        }
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
        visitor.visit_unit()
    }

    fn is_human_readable(&self) -> bool {
        true
    }
}

struct KeyDeserializer<'a, 'de>(&'a Key<'de>);

impl<'a, 'de> de::Deserializer<'de> for KeyDeserializer<'a, 'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_str(self.0)
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum identifier ignored_any
    }
}

struct MapDeserializer<'a, 'de>(&'a Map<'de>);

impl<'a, 'de> de::Deserializer<'de> for MapDeserializer<'a, 'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
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

        visitor.visit_map(MapAccess(self.0.iter(), None))
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum identifier ignored_any
    }
}

struct ListDeserializer<'a, 'de>(&'a List<'de>);

impl<'a, 'de> de::Deserializer<'de> for ListDeserializer<'a, 'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
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

            fn size_hint(&self) -> Option<usize> {
                match self.0.size_hint() {
                    (min, Some(max)) if min == max => Some(min),
                    _ => None,
                }
            }
        }

        visitor.visit_seq(ListAccess(self.0.iter()))
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum identifier ignored_any
    }
}
