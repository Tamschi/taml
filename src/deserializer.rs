//TODO: Add secondary labels without caption while unrolling due to error. Disarm/return `Ok(())` with  `.void()` on that guard.

use {
    crate::{
        diagnostics::{
            Diagnostic, DiagnosticLabel, DiagnosticLabelPriority, DiagnosticType,
            Reporter as diagReporter,
        },
        parser::{
            parse, IntoToken, Key, List, ListIter, Map, MapIter, Taml, TamlValue, VariantPayload,
        },
        token::Token,
    },
    indexmap::IndexMap,
    serde::de,
    std::{borrow::Cow, ops::Range},
    woc::Woc,
    wyz::{Pipe as _, Tap as _},
};

pub struct Deserializer<'a, 'de, Position: Clone, Reporter: diagReporter<Position>>(
    pub &'a Taml<'de, Position>,
    pub &'a mut Reporter,
);

#[derive(Debug, PartialEq, Eq)]
pub struct Error;
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Error")
    }
}
impl std::error::Error for Error {}
impl de::Error for Error {
    // This error type is never constructed directly.
    fn custom<T>(_msg: T) -> Self
    where
        T: std::fmt::Display,
    {
        unimplemented!()
    }
    fn invalid_type(_unexp: de::Unexpected, _exp: &dyn de::Expected) -> Self {
        unimplemented!()
    }
    fn invalid_value(_unexp: de::Unexpected, _exp: &dyn de::Expected) -> Self {
        unimplemented!()
    }
    fn invalid_length(_len: usize, _exp: &dyn de::Expected) -> Self {
        unimplemented!()
    }
    fn unknown_variant(_variant: &str, _expected: &'static [&'static str]) -> Self {
        unimplemented!()
    }
    fn unknown_field(_field: &str, _expected: &'static [&'static str]) -> Self {
        unimplemented!()
    }
    fn missing_field(_field: &'static str) -> Self {
        unimplemented!()
    }
    fn duplicate_field(_field: &'static str) -> Self {
        unimplemented!()
    }
}

pub type Result<T> = std::result::Result<T, Error>;

fn format_exp(exp: &dyn de::Expected) -> String {
    format!("{}", exp)
}

enum SerdeError {
    Custom(String),
    InvalidType(String, String),
    InvalidValue(String, String),
    InvalidLength(usize, String),
    UnknownVariant(String, &'static [&'static str]),
    UnknownField(String, &'static [&'static str]),
    MissingField(&'static str),

    /// Used to specify that no further error handling should be done.
    Silent,
}
impl std::fmt::Debug for SerdeError {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unimplemented!()
    }
}
impl std::fmt::Display for SerdeError {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unimplemented!()
    }
}
impl std::error::Error for SerdeError {}
impl de::Error for SerdeError {
    fn custom<T>(msg: T) -> Self
    where
        T: std::fmt::Display,
    {
        SerdeError::Custom(format!("{}", msg))
    }
    fn invalid_type(unexp: de::Unexpected, exp: &dyn de::Expected) -> Self {
        SerdeError::InvalidType(unexp.to_string(), format_exp(exp))
    }
    fn invalid_value(unexp: de::Unexpected, exp: &dyn de::Expected) -> Self {
        SerdeError::InvalidValue(unexp.to_string(), format_exp(exp))
    }
    fn invalid_length(len: usize, exp: &dyn de::Expected) -> Self {
        SerdeError::InvalidLength(len, format_exp(exp))
    }
    fn unknown_variant(variant: &str, expected: &'static [&'static str]) -> Self {
        SerdeError::UnknownVariant(variant.to_string(), expected)
    }
    fn unknown_field(field: &str, expected: &'static [&'static str]) -> Self {
        SerdeError::UnknownField(field.to_string(), expected)
    }
    fn missing_field(field: &'static str) -> Self {
        SerdeError::MissingField(field)
    }
    fn duplicate_field(_field: &'static str) -> Self {
        // This is caught/resolved in the parsing stage.
        unreachable!()
    }
}

impl SerdeError {
    #[inline(always)]
    fn reporter<'a, Position: 'a, Reporter: diagReporter<Position>>(
        reporter: &'a mut Reporter,
        span: Range<Position>,
    ) -> impl 'a + FnOnce(SerdeError) -> Error {
        struct OneOfOrNothing<'a>(&'a [&'a str]);
        impl<'a> std::fmt::Display for OneOfOrNothing<'a> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self.0 {
                    [] => write!(f, "nothing"),
                    items if items.len() == 1 => write!(f, "`{}`", items[0]),
                    items => {
                        let len = items.len();
                        let mut items = items.iter();
                        for item in items.by_ref().take(len - 2) {
                            write!(f, "`{}`, ", item)?
                        }
                        write!(
                            f,
                            "`{}` or `{}`",
                            items.next().unwrap(),
                            items.next().unwrap()
                        )
                    }
                }
            }
        }

        move |serde_error| {
            if matches!(serde_error, SerdeError::Silent) {
                return Error;
            }

            reporter.report_with(|| Diagnostic {
                r#type: match serde_error {
                    SerdeError::Custom(_) => DiagnosticType::CustomErrorFromVisitor,
                    SerdeError::InvalidType(_, _) => DiagnosticType::InvalidType,
                    SerdeError::InvalidValue(_, _) => DiagnosticType::InvalidValue,
                    SerdeError::InvalidLength(_, _) => DiagnosticType::InvalidLength,
                    SerdeError::UnknownVariant(_, _) => DiagnosticType::UnknownVariant,
                    SerdeError::UnknownField(_, _) => DiagnosticType::UnknownField,
                    SerdeError::MissingField(_) => DiagnosticType::MissingField,
                    SerdeError::Silent => unreachable!(),
                },
                labels: vec![DiagnosticLabel::new(
                    match serde_error {
                        SerdeError::Custom(string) => Cow::Owned(string),
                        SerdeError::InvalidType(_, exp)
                        | SerdeError::InvalidValue(_, exp)
                        | SerdeError::InvalidLength(_, exp) => {
                            format!("Expected {} here.", exp).into()
                        }
                        SerdeError::UnknownVariant(_, exp) => match exp {
                            [] => "There are no possible variants.".into(),
                            exp => format!("Expected {}.", OneOfOrNothing(exp)).into(),
                        },
                        SerdeError::UnknownField(_, exp) => match exp {
                            [] => "Expected no fields.".into(),
                            exp => format!("Expected {}.", OneOfOrNothing(exp)).into(),
                        },
                        SerdeError::MissingField(field) => {
                            format!("Missing field {}.", field).into()
                        }
                        SerdeError::Silent => unreachable!(),
                    },
                    span,
                    DiagnosticLabelPriority::Primary,
                )],
            });
            Error
        }
    }

    fn silence(_error: Error) -> SerdeError {
        SerdeError::Silent
    }
}

type SerdeResult<T> = std::result::Result<T, SerdeError>;

#[allow(clippy::missing_errors_doc)]
pub fn from_str<'de, T: de::Deserialize<'de>, Reporter: diagReporter<usize>>(
    str: &'de str,
    reporter: &mut Reporter,
) -> Result<T> {
    use logos::Logos as _;
    let lexer = Token::lexer(str).spanned();
    from_tokens(lexer, reporter)
}

#[allow(clippy::missing_errors_doc)]
pub fn from_tokens<'de, T: de::Deserialize<'de>, Position: Clone + Default + Ord>(
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
pub fn from_taml<'de, T: de::Deserialize<'de>, Position: Clone + Ord>(
    taml: &Taml<'de, Position>,
    reporter: &mut impl diagReporter<Position>,
) -> Result<T> {
    T::deserialize(&mut Deserializer(&taml, reporter))
}

macro_rules! number {
    ($deserialize:ident, $TamlVariant:ident => $visit:ident) => {
        fn $deserialize<V>(self, visitor: V) -> Result<V::Value>
        where
            V: de::Visitor<'de>,
        {
            match &self.0.value {
                TamlValue::$TamlVariant(str) => {
                    let value = str
                        .parse()
                        .map_err(|_| de::Error::invalid_value(de::Unexpected::Other(str), &visitor))
                        .map_err(SerdeError::reporter(self.1, self.0.span.clone()))?;
                    visitor
                        .$visit(value)
                        .map_err(SerdeError::reporter(self.1, self.0.span.clone()))
                }
                _ => invalid_type!(self, visitor),
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

fn unexpected<'a, Position>(value: &'a TamlValue<'a, Position>) -> de::Unexpected<'a> {
    match value {
        TamlValue::String(str) => de::Unexpected::Str(str),
        TamlValue::Integer(str) | TamlValue::Float(str) => de::Unexpected::Other(str),
        TamlValue::List(_) => de::Unexpected::Seq,
        TamlValue::Map(_) => de::Unexpected::Map,
        TamlValue::EnumVariant { key: _, payload } => match payload {
            VariantPayload::Structured(_) => de::Unexpected::StructVariant,
            VariantPayload::Tuple(values) if values.len() == 1 => de::Unexpected::NewtypeVariant,
            VariantPayload::Tuple(_) => de::Unexpected::TupleVariant,
            VariantPayload::Unit => de::Unexpected::UnitVariant,
        },
    }
}

macro_rules! invalid_type {
    ($self:ident, $visitor:ident) => {
        Err(de::Error::invalid_type(
            unexpected(&$self.0.value),
            &$visitor,
        ))
        .map_err(SerdeError::reporter($self.1, $self.0.span.clone()))
    };
}

impl<'a, 'de, Position: Clone + Ord, Reporter: diagReporter<Position>> de::Deserializer<'de>
    for &mut Deserializer<'a, 'de, Position, Reporter>
{
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        match &self.0.value {
            TamlValue::String(_) => self.deserialize_str(visitor),
            TamlValue::Integer(str) => {
                if str.starts_with('-') {
                    if let Ok(i8) = str.parse::<i8>() {
                        visitor
                            .visit_i8(i8)
                            .map_err(SerdeError::reporter(self.1, self.0.span.clone()))
                    } else if let Ok(i16) = str.parse::<i16>() {
                        visitor
                            .visit_i16(i16)
                            .map_err(SerdeError::reporter(self.1, self.0.span.clone()))
                    } else if let Ok(i32) = str.parse::<i32>() {
                        visitor
                            .visit_i32(i32)
                            .map_err(SerdeError::reporter(self.1, self.0.span.clone()))
                    } else if let Ok(i64) = str.parse::<i64>() {
                        visitor
                            .visit_i64(i64)
                            .map_err(SerdeError::reporter(self.1, self.0.span.clone()))
                    } else if let Ok(i128) = str.parse::<i128>() {
                        visitor
                            .visit_i128(i128)
                            .map_err(SerdeError::reporter(self.1, self.0.span.clone()))
                    } else {
                        Err(de::Error::invalid_value(
                            de::Unexpected::Other(str),
                            &"an integer value requiring up to 128 bits",
                        ))
                        .map_err(SerdeError::reporter(self.1, self.0.span.clone()))
                    }
                } else if let Ok(u8) = str.parse::<u8>() {
                    visitor
                        .visit_u8(u8)
                        .map_err(SerdeError::reporter(self.1, self.0.span.clone()))
                } else if let Ok(u16) = str.parse::<u16>() {
                    visitor
                        .visit_u16(u16)
                        .map_err(SerdeError::reporter(self.1, self.0.span.clone()))
                } else if let Ok(u32) = str.parse::<u32>() {
                    visitor
                        .visit_u32(u32)
                        .map_err(SerdeError::reporter(self.1, self.0.span.clone()))
                } else if let Ok(u64) = str.parse::<u64>() {
                    visitor
                        .visit_u64(u64)
                        .map_err(SerdeError::reporter(self.1, self.0.span.clone()))
                } else if let Ok(u128) = str.parse::<u128>() {
                    visitor
                        .visit_u128(u128)
                        .map_err(SerdeError::reporter(self.1, self.0.span.clone()))
                } else {
                    Err(de::Error::invalid_value(
                        de::Unexpected::Other(str),
                        &"an integer value requiring up to 128 bits",
                    ))
                    .map_err(SerdeError::reporter(self.1, self.0.span.clone()))
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
            } if key == "true" => visitor
                .visit_bool(true)
                .map_err(SerdeError::reporter(self.1, self.0.span.clone())),
            TamlValue::EnumVariant {
                key,
                payload: VariantPayload::Unit,
            } if key == "false" => visitor
                .visit_bool(false)
                .map_err(SerdeError::reporter(self.1, self.0.span.clone())),

            _ => Err(de::Error::custom("Expected boolean `true` or `false`."))
                .map_err(SerdeError::reporter(self.1, self.0.span.clone())),
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
                let value = str.parse().map_err(|_| Error)?;
                visitor
                    .visit_char(value)
                    .map_err(SerdeError::reporter(self.1, self.0.span.clone()))
            }
            _ => invalid_type!(self, visitor),
        }
    }
    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        match &self.0.value {
            TamlValue::String(str) => match str {
                Woc::Owned(string) => visitor.visit_str(string),
                Woc::Borrowed(str) => visitor.visit_borrowed_str(str),
            }
            .map_err(SerdeError::reporter(self.1, self.0.span.clone())),
            _ => invalid_type!(self, visitor),
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
        visitor.visit_some(self) // Plain forward.
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        if matches!(&self.0.value, TamlValue::List(list) if list.is_empty()) {
            visitor
                .visit_unit()
                .map_err(SerdeError::reporter(self.1, self.0.span.clone()))
        } else {
            Err(Error)
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
        visitor.visit_newtype_struct(self) // Plain forward.
    }
    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        match &self.0.value {
            TamlValue::List(list) => {
                de::Deserializer::deserialize_seq(ListDeserializer(&list, self.1), visitor)
            }
            _ => invalid_type!(self, visitor),
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
            _ => invalid_type!(self, visitor),
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
            TamlValue::Map(map) => de::Deserializer::deserialize_map(
                MapDeserializer {
                    map,
                    span: self.0.span.clone(),
                    reporter: self.1,
                },
                visitor,
            )
            .map_err(SerdeError::reporter(self.1, self.0.span.clone())),
            _ => invalid_type!(self, visitor),
        }
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        const EXTRA_FIELDS: &str = "taml::extra_fields";

        match &self.0.value {
            TamlValue::Map(map) => {
                if !fields.contains(&EXTRA_FIELDS) {
                    // The default Deserialize doesn't check for extra fields, but TAML should be strict.
                    // We can report all of these diagnostics and continue parsing for a bit.
                    let mut status = Ok(());
                    for key in map
                        .keys()
                        .filter(|key| !fields.contains(&key.as_ref()))
                        .collect::<Vec<_>>()
                        .tap_mut(|keys| keys.sort_by_key(|key| &key.span.start))
                    {
                        status = Err(SerdeError::reporter(self.1, key.span.clone())(
                            de::Error::unknown_field(key, fields),
                        ));
                    }

                    let result = de::Deserializer::deserialize_map(
                        MapDeserializer {
                            map,
                            span: self.0.span.clone(),
                            reporter: self.1,
                        },
                        visitor,
                    )
                    .map_err(SerdeError::reporter(self.1, self.0.span.clone()));

                    status.and(result)
                } else {
                    let known_fields: Vec<_> = fields
                        .iter()
                        .copied()
                        .filter(|name| *name != EXTRA_FIELDS)
                        .collect();
                    let (known, extra): (IndexMap<_, _>, IndexMap<_, _>) = map
                        .iter()
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .partition(|(key, _)| known_fields.contains(&key.as_ref()));
                    let mut known = known;
                    assert!(known
                        .insert(
                            Key {
                                name: Woc::Borrowed(EXTRA_FIELDS),
                                span: self.0.span.clone(),
                            },
                            Taml {
                                span: self.0.span.clone(),
                                value: TamlValue::Map(extra)
                            },
                        )
                        .is_none());
                    de::Deserializer::deserialize_map(
                        MapDeserializer {
                            map: &known,
                            span: self.0.span.clone(),
                            reporter: self.1,
                        },
                        visitor,
                    )
                    .map_err(SerdeError::reporter(self.1, self.0.span.clone()))
                }
            }
            _ => invalid_type!(self, visitor),
        }
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
        #[cfg(feature = "serde-object-assist")]
        lazy_static::lazy_static! {
            static ref HINT: std::sync::Mutex<Option<serde_object_assistant_extra::VariantKind>> = std::sync::Mutex::default();
        }

        #[cfg(feature = "serde-object-assist")]
        #[linkme::distributed_slice(serde_object_assistant_extra::ENUM_VARIANT_ASSISTS)]
        static ENUM_VARIANT_ASSIST: fn() -> Option<serde_object_assistant_extra::VariantKind> =
            || HINT.lock().unwrap().take();

        struct EnumAccess<'a, 'de, Position, Reporter: diagReporter<Position>>(
            &'a Key<'de, Position>,
            &'a VariantPayload<'de, Position>,
            &'a mut Reporter,
        );

        impl<'a, 'de, Position: Clone + Ord, Reporter: diagReporter<Position>> de::EnumAccess<'de>
            for EnumAccess<'a, 'de, Position, Reporter>
        {
            type Error = Error;
            type Variant = VariantAccess<'a, 'de, Position, Reporter>;

            fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
            where
                V: de::DeserializeSeed<'de>,
            {
                let value = seed.deserialize(KeyDeserializer(self.0, self.2))?;

                #[cfg(feature = "serde-object-assist")]
                {
                    use serde_object_assistant_extra::VariantKind;
                    *HINT.lock().unwrap() = match &self.1 {
                        VariantPayload::Structured(map) => VariantKind::Struct(
                            map.keys()
                                .map(|k| k.name.to_string().into())
                                .collect::<Vec<_>>()
                                .into(),
                        ),
                        VariantPayload::Tuple(list) if list.len() == 1 => VariantKind::Newtype,
                        VariantPayload::Tuple(list) => VariantKind::Tuple(list.len()),
                        VariantPayload::Unit => VariantKind::Unit,
                    }
                    .pipe(Some);
                }

                Ok((
                    value,
                    VariantAccess {
                        payload: self.1,
                        span: self.0.span.clone(),
                        reporter: self.2,
                    },
                ))
            }
        }

        struct VariantAccess<'a, 'de, Position: Clone, Reporter: diagReporter<Position>> {
            payload: &'a VariantPayload<'de, Position>,
            span: Range<Position>,
            reporter: &'a mut Reporter,
        };

        impl<'a, 'de, Position: Clone + Ord, Reporter: diagReporter<Position>>
            de::VariantAccess<'de> for VariantAccess<'a, 'de, Position, Reporter>
        {
            type Error = Error;

            fn unit_variant(self) -> Result<()> {
                match self.payload {
                    VariantPayload::Unit => Ok(()),
                    _ => Err(Error), //TODO
                }
            }

            fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
            where
                T: de::DeserializeSeed<'de>,
            {
                match self.payload {
                    VariantPayload::Tuple(values) if values.len() == 1 => {
                        seed.deserialize(&mut Deserializer(&values[0], self.reporter))
                    }
                    VariantPayload::Tuple(values) => Err(de::Error::invalid_length(
                        values.len(),
                        &"tuple variant of length 1",
                    )),
                    _ => Err(Error), //TODO
                }
            }

            fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value>
            where
                V: de::Visitor<'de>,
            {
                match self.payload {
                    VariantPayload::Tuple(values) if values.len() == len => {
                        de::Deserializer::deserialize_seq(
                            ListDeserializer(values, self.reporter),
                            visitor,
                        )
                    }
                    VariantPayload::Tuple(values) => {
                        Err(de::Error::invalid_length(values.len(), &visitor))
                    }
                    _ => Err(Error), //TODO
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
                match self.payload {
                    VariantPayload::Structured(fields) => de::Deserializer::deserialize_map(
                        MapDeserializer {
                            span: self.span.clone(),
                            map: fields,
                            reporter: self.reporter,
                        },
                        visitor,
                    )
                    .map_err(SerdeError::reporter(self.reporter, self.span.clone())),
                    _ => Err(Error), //TODO
                }
            }
        }

        match &self.0.value {
            TamlValue::EnumVariant { key, payload } => {
                visitor.visit_enum(EnumAccess(key, payload, self.1)) // Plain forward, hopefully.
            }
            _ => invalid_type!(self, visitor),
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
        visitor
            .visit_unit()
            .map_err(SerdeError::reporter(self.1, self.0.span.clone()))
    }

    fn is_human_readable(&self) -> bool {
        true
    }
}

struct KeyDeserializer<'a, 'de, Position, Reporter: diagReporter<Position>>(
    &'a Key<'de, Position>,
    &'a mut Reporter,
);

impl<'a, 'de, Position: Clone, Reporter: diagReporter<Position>> de::Deserializer<'de>
    for KeyDeserializer<'a, 'de, Position, Reporter>
{
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        visitor
            .visit_str(self.0)
            .map_err(SerdeError::reporter(self.1, self.0.span.clone()))
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum identifier ignored_any
    }
}

struct MapDeserializer<'a, 'de, Position, Reporter: diagReporter<Position>> {
    map: &'a Map<'de, Position>,
    span: Range<Position>,
    reporter: &'a mut Reporter,
}

impl<'a, 'de, Position: Clone + Ord, Reporter: diagReporter<Position>> de::Deserializer<'de>
    for MapDeserializer<'a, 'de, Position, Reporter>
{
    type Error = SerdeError;

    fn deserialize_any<V>(self, visitor: V) -> SerdeResult<V::Value>
    where
        V: de::Visitor<'de>,
    {
        struct MapAccess<'a, 'de, Position, Reporter: diagReporter<Position>> {
            iter: MapIter<'a, 'de, Position>,
            next_value: Option<&'a Taml<'de, Position>>,
            reporter: &'a mut Reporter,
        }

        impl<'a, 'de, Position: Clone + Ord, Reporter: diagReporter<Position>> de::MapAccess<'de>
            for MapAccess<'a, 'de, Position, Reporter>
        {
            type Error = SerdeError;

            fn next_key_seed<K: de::DeserializeSeed<'de>>(
                &mut self,
                seed: K,
            ) -> SerdeResult<Option<K::Value>> {
                self.iter
                    .next()
                    .map(|(k, v)| {
                        self.next_value = Some(v);
                        seed.deserialize(KeyDeserializer(k, self.reporter))
                    })
                    .transpose()
                    .map_err(SerdeError::silence)
            }

            fn next_value_seed<V: de::DeserializeSeed<'de>>(
                &mut self,
                seed: V,
            ) -> SerdeResult<V::Value> {
                seed.deserialize(&mut Deserializer(
                    self.next_value
                        .expect("next_value_seed called before next_key_seed"),
                    self.reporter,
                ))
                .map_err(SerdeError::silence)
            }
        }

        visitor
            .visit_map(MapAccess {
                iter: self.map.iter(),
                next_value: None,
                reporter: self.reporter,
            })
            .map_err(SerdeError::reporter(self.reporter, self.span.clone()))
            .map_err(SerdeError::silence)
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

impl<'a, 'de, Position: Clone + Ord, Reporter: diagReporter<Position>> de::Deserializer<'de>
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

        impl<'a, 'de, Position: Clone + Ord, Reporter: diagReporter<Position>> de::SeqAccess<'de>
            for ListAccess<'a, 'de, Position, Reporter>
        {
            type Error = Error;

            fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
            where
                T: de::DeserializeSeed<'de>,
            {
                self.0
                    .next()
                    .map(|t| seed.deserialize(&mut Deserializer(t, self.1)))
                    .transpose()
            }

            fn size_hint(&self) -> Option<usize> {
                match self.0.size_hint() {
                    (min, Some(max)) if min == max => Some(min),
                    _ => None,
                }
            }
        }

        visitor.visit_seq(ListAccess(self.0.iter(), self.1)) // Plain forward, hopefully.
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum identifier ignored_any
    }
}
