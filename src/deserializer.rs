use {
    crate::{Error, Expected, Result},
    serde::{
        de::{self, Visitor},
        Deserialize,
    },
    std::ops::{AddAssign, MulAssign, Neg},
};

// SEE: https://serde.rs/impl-deserializer.html

pub struct Deserializer<'de> {
    input: &'de str,
}

impl<'de> Deserializer<'de> {
    pub fn from_str(input: &'de str) -> Self {
        Self { input }
    }
}

pub fn from_str<'a, T: Deserialize<'a>>(input: &'a str) -> Result<'a, T> {
    let mut deserializer = Deserializer::from_str(input);
    let t = T::deserialize(&mut deserializer)?;
    if deserializer.input.is_empty() {
        Ok(t)
    } else {
        Err(Error::TrailingCharacters(deserializer.input))
    }
}

impl<'de> Deserializer<'de> {
    fn peek_char(&self) -> Result<'static, char> {
        self.input.chars().next().ok_or(Error::EndOfInput)
    }

    fn next_char(&mut self) -> Result<'static, char> {
        let c = self.peek_char()?;
        self.input = &self.input[c.len_utf8()..];
        Ok(c)
    }

    fn parse_bool(&mut self) -> Result<'de, bool> {
        const t: &str = "true";
        const f: &str = "false";
        if self.input.starts_with(t) {
            self.input = &self.input[t.len()..];
            Ok(true)
        } else if self.input.starts_with(f) {
            self.input = &self.input[f.len()..];
            Ok(false)
        } else {
            Err(Error::Expected {
                expected: Expected::Boolean,
                rest: self.input,
            })
        }
    }

    //TODO: Rework!
    //TODO: Make checked!
    fn parse_unsigned<T>(&mut self) -> Result<'de, T>
    where
        T: AddAssign<T> + MulAssign<T> + From<u8>,
    {
        let mut int = match self.next_char()? {
            ch @ '0'..='9' => T::from(ch as u8 - b'0'),
            _ => {
                return Err(Error::Expected {
                    expected: Expected::Integer,
                    rest: self.input,
                });
            }
        };
        loop {
            match self.input.chars().next() {
                Some(ch @ '0'..='9') => {
                    self.input = &self.input[1..];
                    int *= T::from(10);
                    int += T::from(ch as u8 - b'0');
                }
                _ => {
                    return Ok(int);
                }
            }
        }
    }

    //TODO: Rework!
    //TODO: Make checked!
    fn parse_signed<T>(&mut self) -> Result<'de, T>
    where
        T: Neg<Output = T> + AddAssign<T> + MulAssign<T> + From<i8>,
    {
        let negate = if self.peek_char()? == '-' {
            self.next_char().unwrap();
            true
        } else {
            false
        };
        let mut int = match self.next_char()? {
            ch @ '0'..='9' => T::from((ch as u8 - b'0') as i8),
            _ => {
                return Err(Error::Expected {
                    expected: Expected::Integer,
                    rest: self.input,
                });
            }
        };

        loop {
            match self.input.chars().next() {
                Some(ch @ '0'..='9') => {
                    self.input = &self.input[1..];
                    int *= T::from(10);
                    int += T::from((ch as u8 - b'0') as i8);
                }
                _ => {
                    return Ok(if negate { -int } else { int });
                }
            }
        }
    }

    fn unsupported<T>(&mut self) -> Result<'de, T> {
        unimplemented!("{} not supported.", std::any::type_name::<T>())
    }
}

macro_rules! defn {
    ($fn:ident => $parse:ident | $visit:ident) => {
        fn $fn<V: Visitor<'de>>(self, visitor: V) -> Result<'de, V::Value> {
            visitor.$visit(self.$parse()?)
        }
    };
}

impl<'de, 'a> de::Deserializer<'de> for &'a mut Deserializer<'de> {
    type Error = Error<'de>;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<'de, V::Value>
    where
        V: Visitor<'de>,
    {
        match self.peek_char()? {
            _ => todo!(),
        }
    }

    defn!(deserialize_bool => parse_bool | visit_bool);

    defn!(deserialize_i8 => parse_signed | visit_i8);
    defn!(deserialize_i16 => parse_signed | visit_i16);
    defn!(deserialize_i32 => parse_signed | visit_i32);
    defn!(deserialize_i64 => parse_signed | visit_i64);
    defn!(deserialize_i128 => parse_signed | visit_i128); //TODO: >=1.26

    defn!(deserialize_u8 => parse_unsigned | visit_u8);
    defn!(deserialize_u16 => parse_unsigned | visit_u16);
    defn!(deserialize_u32 => parse_unsigned | visit_u32);
    defn!(deserialize_u64 => parse_unsigned | visit_u64);
    defn!(deserialize_u128 => parse_unsigned | visit_u128); //TODO: >=1.26

    defn!(deserialize_f32 => parse_float | visit_f32);
    defn!(deserialize_f64 => parse_float | visit_f64);

    defn!(deserialize_char => parse_char | visit_char);
    defn!(deserialize_str => parse_str | visit_str);
    defn!(deserialize_string => parse_string | visit_string);

    defn!(deserialize_bytes => unsupported | visit_bytes);
    defn!(deserialize_byte_buf => unsupported | visit_byte_buf);

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<'de, V::Value> {
        unimplemented!()
    }

    fn deserialize_unit<V: Visitor<'de>>(self, visitor: V) -> Result<'de, V::Value> {
        unimplemented!()
    }

    fn deserialize_unit_struct<V: Visitor<'de>>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<'de, V::Value> {
        unimplemented!()
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<'de, V::Value> {
        unimplemented!()
    }

    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> Result<'de, V::Value> {
        unimplemented!()
    }

    fn deserialize_tuple<V: Visitor<'de>>(self, len: usize, visitor: V) -> Result<'de, V::Value> {
        unimplemented!()
    }

    fn deserialize_tuple_struct<V: Visitor<'de>>(
        self,
        name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<'de, V::Value> {
        unimplemented!()
    }

    fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> Result<'de, V::Value> {
        unimplemented!()
    }

    fn deserialize_struct<V: Visitor<'de>>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<'de, V::Value> {
        unimplemented!()
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<'de, V::Value> {
        unimplemented!()
    }

    fn deserialize_identifier<V: Visitor<'de>>(self, visitor: V) -> Result<'de, V::Value> {
        unimplemented!()
    }

    fn deserialize_ignored_any<V: Visitor<'de>>(self, visitor: V) -> Result<'de, V::Value> {
        unimplemented!()
    }

    // is_human_readable => true
}
