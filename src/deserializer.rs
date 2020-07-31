//TODO: Add secondary labels without caption while unrolling due to error. Disarm/return `Ok(())` with  `.void()` on that guard.

use {
    crate::{
        diagnostics::Reporter as diagReporter,
        parser::{
            parse, IntoToken, Key, List, ListIter, Map, MapIter, Taml, TamlValue, VariantPayload,
        },
        token::Token,
    },
    serde::de,
};

struct Deserializer<'a, 'de, Position, Reporter: diagReporter<Position>>(
    &'a Taml<'de, Position>,
    &'a mut Reporter,
);

pub type Error = de::value::Error;
pub type Result<T> = std::result::Result<T, Error>;

#[allow(clippy::missing_errors_doc)]
pub fn from_str<T: de::DeserializeOwned>(
    str: &str,
    reporter: &mut impl diagReporter<usize>,
) -> Result<T> {
    use logos::Logos as _;
    let lexer = Token::lexer(str).spanned();
    from_tokens(lexer, reporter)
}

#[allow(clippy::missing_errors_doc)]
pub fn from_tokens<'de, T: de::Deserialize<'de>, Position: Clone + Default>(
    tokens: impl IntoIterator<Item = impl IntoToken<'de, Position>>,
    reporter: &mut impl diagReporter<Position>,
) -> Result<T> {
    //TODO: This seems overly explicit.
    let root = parse(tokens, reporter).map_err(|()| de::Error::custom("Pasing error"))?;

    from_taml(
        &Taml {
            value: TamlValue::Map(root),
            span: Position::default()..Position::default(),
        },
        reporter,
    )
}

#[allow(clippy::missing_errors_doc)]
pub fn from_taml<'de, T: de::Deserialize<'de>, Position>(
    taml: &Taml<'de, Position>,
    reporter: &mut impl diagReporter<Position>,
) -> Result<T> {
    T::deserialize(Deserializer(&taml, reporter))
}

fn invalid_type<'de, Position>(unexp: &'de Taml<'de, Position>, exp: &dyn de::Expected) -> Error {
    de::Error::invalid_type(
        match &unexp.value {
            TamlValue::String(str) => de::Unexpected::Str(str),
            TamlValue::Integer(_) => de::Unexpected::Other("integer"), //TODO
            TamlValue::Float(str) => str
                .parse()
                .map_or_else(|_| de::Unexpected::Other(str), de::Unexpected::Float),
            TamlValue::List(_) => de::Unexpected::Seq,
            TamlValue::Map(_) => de::Unexpected::Map,
            TamlValue::EnumVariant { payload, .. } => match payload {
                VariantPayload::Structured(_) => de::Unexpected::StructVariant,
                VariantPayload::Tuple(values) => match values.len() {
                    1 => de::Unexpected::NewtypeVariant,
                    _ => de::Unexpected::TupleVariant,
                },
                VariantPayload::Unit => de::Unexpected::UnitVariant,
            },
        },
        exp,
    )
}

fn invalid_variant_type<'de, Position>(
    unexp: &'de VariantPayload<'de, Position>,
    exp: &dyn de::Expected,
) -> Error {
    de::Error::invalid_type(
        match &unexp {
            VariantPayload::Structured(_) => de::Unexpected::StructVariant,
            VariantPayload::Tuple(values) => match values.len() {
                1 => de::Unexpected::NewtypeVariant,
                _ => de::Unexpected::TupleVariant,
            },
            VariantPayload::Unit => de::Unexpected::UnitVariant,
        },
        exp,
    )
}

fn invalid_value<'de, Position>(unexp: &'de Taml<'de, Position>, exp: &dyn de::Expected) -> Error {
    de::Error::invalid_value(
        match &unexp.value {
            TamlValue::String(str) => de::Unexpected::Str(str),
            TamlValue::Integer(str) => de::Unexpected::Other(str),
            TamlValue::Float(str) => str
                .parse()
                .map_or_else(|_| de::Unexpected::Other(str), de::Unexpected::Float),
            TamlValue::List(_) => de::Unexpected::Seq,
            TamlValue::Map(_) => de::Unexpected::Map,
            TamlValue::EnumVariant { payload, .. } => match payload {
                VariantPayload::Structured(_) => de::Unexpected::StructVariant,
                VariantPayload::Tuple(values) => match values.len() {
                    1 => de::Unexpected::NewtypeVariant,
                    _ => de::Unexpected::TupleVariant,
                },
                VariantPayload::Unit => de::Unexpected::UnitVariant,
            },
        },
        exp,
    )
}

macro_rules! number {
    ($deserialize:ident, $TamlVariant:ident => $visit:ident) => {
        fn $deserialize<V>(self, visitor: V) -> Result<V::Value>
        where
            V: de::Visitor<'de>,
        {
            match &self.0.value {
                TamlValue::$TamlVariant(str) => {
                    let value = str.parse().map_err(|_| invalid_value(self.0, &visitor))?;
                    visitor.$visit(value)
                }
                _ => Err(invalid_type(self.0, &visitor)),
            }
        }
    };
}

macro_rules! integer {
    ($($deserialize:ident => $visit:ident,)*) => {
        $(number!($deserialize, Integer => $visit);)*
    };
}

macro_rules! float {
    ($($deserialize:ident => $visit:ident,)*) => {
        $(number!($deserialize, Float => $visit);)*
    };
}

impl<'a, 'de, Position, Reporter: diagReporter<Position>> de::Deserializer<'de>
    for Deserializer<'a, 'de, Position, Reporter>
{
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        match &self.0.value {
            TamlValue::String(_) => self.deserialize_str(visitor),
            TamlValue::Integer(str) => {
                if let Ok(u8) = str.parse::<u8>() {
                    visitor.visit_u8(u8)
                } else if let Ok(u16) = str.parse::<u16>() {
                    visitor.visit_u16(u16)
                } else if let Ok(u32) = str.parse::<u32>() {
                    visitor.visit_u32(u32)
                } else if let Ok(u64) = str.parse::<u64>() {
                    visitor.visit_u64(u64)
                } else if let Ok(u128) = str.parse::<u128>() {
                    visitor.visit_u128(u128)
                } else if let Ok(i8) = str.parse::<i8>() {
                    visitor.visit_i8(i8)
                } else if let Ok(i16) = str.parse::<i16>() {
                    visitor.visit_i16(i16)
                } else if let Ok(i32) = str.parse::<i32>() {
                    visitor.visit_i32(i32)
                } else if let Ok(i64) = str.parse::<i64>() {
                    visitor.visit_i64(i64)
                } else if let Ok(i128) = str.parse::<i128>() {
                    visitor.visit_i128(i128)
                } else {
                    Err(invalid_value(self.0, &visitor))
                }
            }
            TamlValue::Float(_) => self.deserialize_f64(visitor),
            TamlValue::List(_) => self.deserialize_seq(visitor),
            TamlValue::Map(_) => self.deserialize_map(visitor),
            TamlValue::EnumVariant { .. } => {
                self.deserialize_enum(
                    "",  // Ignored.
                    &[], // Ignored.
                    visitor,
                )
            }
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        match &self.0.value {
            TamlValue::EnumVariant {
                key,
                payload: VariantPayload::Unit,
            } if key == "true" => visitor.visit_bool(true),
            TamlValue::EnumVariant {
                key,
                payload: VariantPayload::Unit,
            } if key == "false" => visitor.visit_bool(false),
            _ => Err(invalid_type(self.0, &visitor)),
        }
    }

    integer! {
        deserialize_i8 => visit_i8,
        deserialize_i16 => visit_i16,
        deserialize_i32 => visit_i32,
        deserialize_i64 => visit_i64,
        deserialize_i128 => visit_i128,

        deserialize_u8 => visit_u8,
        deserialize_u16 => visit_u16,
        deserialize_u32 => visit_u32,
        deserialize_u64 => visit_u64,
        deserialize_u128 => visit_u128,
    }

    float! {
        deserialize_f32 => visit_f32,
        deserialize_f64 => visit_f64,
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        match &self.0.value {
            TamlValue::String(str) => {
                let value = str.parse().map_err(|_| invalid_value(self.0, &visitor))?;
                visitor.visit_char(value)
            }
            _ => Err(invalid_type(self.0, &visitor)),
        }
    }
    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        match &self.0.value {
            TamlValue::String(str) => visitor.visit_str(str),
            _ => Err(invalid_type(self.0, &visitor)),
        }
    }
    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }
    fn deserialize_bytes<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        unimplemented!("Byte slices are not supported")
    }
    fn deserialize_byte_buf<V>(self, _visitor: V) -> Result<V::Value>
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
        if matches!(&self.0.value, TamlValue::List(list) if list.is_empty()) {
            visitor.visit_unit()
        } else {
            Err(invalid_type(self.0, &visitor))
        }
    }
    fn deserialize_unit_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }
    fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }
    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        match &self.0.value {
            TamlValue::List(list) => {
                de::Deserializer::deserialize_seq(ListDeserializer(&list, self.1), visitor)
            }
            _ => Err(invalid_type(self.0, &visitor)),
        }
    }
    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        match &self.0.value {
            TamlValue::List(list) if list.len() == len => {
                de::Deserializer::deserialize_seq(ListDeserializer(&list, self.1), visitor)
            }
            TamlValue::List(list) => Err(de::Error::invalid_length(list.len(), &visitor)),
            _ => Err(invalid_type(self.0, &visitor)),
        }
    }
    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
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
        match &self.0.value {
            TamlValue::Map(map) => {
                de::Deserializer::deserialize_map(MapDeserializer(map, self.1), visitor)
            }
            _ => Err(invalid_type(self.0, &visitor)),
        }
    }
    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }
    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        struct EnumAccess<'a, 'de, Position, Reporter: diagReporter<Position>>(
            &'a Key<'de, Position>,
            &'a VariantPayload<'de, Position>,
            &'a mut Reporter,
        );

        impl<'a, 'de, Position, Reporter: diagReporter<Position>> de::EnumAccess<'de>
            for EnumAccess<'a, 'de, Position, Reporter>
        {
            type Error = Error;
            type Variant = VariantAccess<'a, 'de, Position, Reporter>;

            fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
            where
                V: de::DeserializeSeed<'de>,
            {
                Ok((
                    seed.deserialize(KeyDeserializer(self.0, self.2))?,
                    VariantAccess(self.1, self.2),
                ))
            }
        }

        struct VariantAccess<'a, 'de, Position, Reporter: diagReporter<Position>>(
            &'a VariantPayload<'de, Position>,
            &'a mut Reporter,
        );

        impl<'a, 'de, Position, Reporter: diagReporter<Position>> de::VariantAccess<'de>
            for VariantAccess<'a, 'de, Position, Reporter>
        {
            type Error = Error;

            fn unit_variant(self) -> Result<()> {
                match self.0 {
                    VariantPayload::Unit => Ok(()),
                    _ => Err(invalid_variant_type(self.0, &"unit variant")),
                }
            }

            fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
            where
                T: de::DeserializeSeed<'de>,
            {
                match self.0 {
                    VariantPayload::Tuple(values) if values.len() == 1 => {
                        seed.deserialize(Deserializer(&values[0], self.1))
                    }
                    VariantPayload::Tuple(values) => Err(de::Error::invalid_length(
                        values.len(),
                        &"tuple variant of length 1",
                    )),
                    _ => Err(invalid_variant_type(self.0, &"tuple variant of length 1")),
                }
            }

            fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value>
            where
                V: de::Visitor<'de>,
            {
                match self.0 {
                    VariantPayload::Tuple(values) if values.len() == len => {
                        de::Deserializer::deserialize_seq(ListDeserializer(values, self.1), visitor)
                    }
                    VariantPayload::Tuple(values) => {
                        Err(de::Error::invalid_length(values.len(), &visitor))
                    }
                    _ => Err(invalid_variant_type(self.0, &visitor)),
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
                    VariantPayload::Structured(fields) => {
                        de::Deserializer::deserialize_map(MapDeserializer(fields, self.1), visitor)
                    }
                    _ => Err(invalid_variant_type(self.0, &visitor)),
                }
            }
        }

        match &self.0.value {
            TamlValue::EnumVariant { key, payload } => {
                visitor.visit_enum(EnumAccess(key, payload, self.1))
            }
            _ => Err(invalid_type(self.0, &visitor)),
        }
    }
    fn deserialize_identifier<V>(self, _visitor: V) -> Result<V::Value>
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

struct KeyDeserializer<'a, 'de, Position, Reporter: diagReporter<Position>>(
    &'a Key<'de, Position>,
    &'a mut Reporter,
);

impl<'a, 'de, Position, Reporter: diagReporter<Position>> de::Deserializer<'de>
    for KeyDeserializer<'a, 'de, Position, Reporter>
{
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

struct MapDeserializer<'a, 'de, Position, Reporter: diagReporter<Position>>(
    &'a Map<'de, Position>,
    &'a mut Reporter,
);

impl<'a, 'de, Position, Reporter: diagReporter<Position>> de::Deserializer<'de>
    for MapDeserializer<'a, 'de, Position, Reporter>
{
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        struct MapAccess<'a, 'de, Position, Reporter: diagReporter<Position>>(
            MapIter<'a, 'de, Position>,
            Option<&'a Taml<'de, Position>>,
            &'a mut Reporter,
        );

        impl<'a, 'de, Position, Reporter: diagReporter<Position>> de::MapAccess<'de>
            for MapAccess<'a, 'de, Position, Reporter>
        {
            type Error = Error;

            fn next_key_seed<K: de::DeserializeSeed<'de>>(
                &mut self,
                seed: K,
            ) -> Result<Option<K::Value>> {
                self.0
                    .next()
                    .map(|(k, v)| {
                        self.1 = Some(v);
                        seed.deserialize(KeyDeserializer(k, self.2))
                    })
                    .transpose()
            }

            fn next_value_seed<V: de::DeserializeSeed<'de>>(
                &mut self,
                seed: V,
            ) -> Result<V::Value> {
                seed.deserialize(Deserializer(
                    self.1.expect("next_value_seed called before next_key_seed"),
                    self.2,
                ))
            }
        }

        visitor.visit_map(MapAccess(self.0.iter(), None, self.1))
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum identifier ignored_any
    }
}

struct ListDeserializer<'a, 'de, Position, Reporter: diagReporter<Position>>(
    &'a List<'de, Position>,
    &'a mut Reporter,
);

impl<'a, 'de, Position, Reporter: diagReporter<Position>> de::Deserializer<'de>
    for ListDeserializer<'a, 'de, Position, Reporter>
{
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        struct ListAccess<'a, 'de, Position, Reporter: diagReporter<Position>>(
            ListIter<'a, 'de, Position>,
            &'a mut Reporter,
        );

        impl<'a, 'de, Position, Reporter: diagReporter<Position>> de::SeqAccess<'de>
            for ListAccess<'a, 'de, Position, Reporter>
        {
            type Error = Error;

            fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
            where
                T: de::DeserializeSeed<'de>,
            {
                self.0
                    .next()
                    .map(|t| seed.deserialize(Deserializer(t, self.1)))
                    .transpose()
            }

            fn size_hint(&self) -> Option<usize> {
                match self.0.size_hint() {
                    (min, Some(max)) if min == max => Some(min),
                    _ => None,
                }
            }
        }

        visitor.visit_seq(ListAccess(self.0.iter(), self.1))
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum identifier ignored_any
    }
}
