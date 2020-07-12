use {serde::de, std::fmt::Display};

//mod deserializer;
mod parser;
mod token;

type Result<'a, T> = std::result::Result<T, Error<'a>>;

#[derive(Debug)]
#[non_exhaustive]
pub enum Error<'a> {
    TrailingCharacters(&'a str),
    EndOfInput,
    Expected { expected: Expected, rest: &'a str },
}

impl<'a> de::Error for Error<'a> {
    fn custom<T>(_: T) -> Self
    where
        T: std::fmt::Display,
    {
        unimplemented!()
    }
}
impl<'a> std::error::Error for Error<'a> {}
impl<'a> Display for Error<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

#[derive(Debug)]
#[non_exhaustive]
pub enum Expected {
    Boolean,
    Integer,
}
