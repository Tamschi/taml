use {
    logos::Logos,
    smartstring::alias::String,
    std::fmt::{Display, Formatter, Result as fmtResult},
};

mod quoting;

use quoting::Woc;

fn trim_leading_0(mut s: &str) -> &str {
    while s.len() >= 2 && s.as_bytes()[0] == b'0' && (b'0'..=b'9').contains(&s.as_bytes()[1]) {
        s = &s[1..]
    }
    s
}

fn trim_trailing_0(mut s: &str) -> &str {
    while s.len() >= 2
        && s.as_bytes()[s.len() - 1] == b'0'
        && (b'0'..=b'9').contains(&s.as_bytes()[s.len() - 2])
    {
        s = &s[..s.len() - 1]
    }
    s
}

#[derive(Logos, Debug, Clone, PartialEq)]
pub enum Token<'a> {
    #[regex(r"//[^\n]+", |lex| &lex.slice()[2..])]
    Comment(&'a str),

    #[token("#")]
    HeadingHash,

    #[token("\n")]
    Newline,

    #[token("[")]
    Brac,
    #[token("]")]
    Ket,

    #[token("{")]
    Bra,
    #[token("}")]
    Ce,

    #[token("(")]
    Paren,
    #[token(")")]
    Thesis,

    #[token(",")]
    Comma,

    #[token(".")]
    Period,

    #[regex(r#""([^\\"]|\\\\|\\")*""#, |lex| quoting::unescape_string_contents(&lex.slice()[1..lex.slice().len() - 1]))]
    String(Woc<'a, String, str>),

    #[regex(r"-?\d+\.\d+", |lex| trim_trailing_0(trim_leading_0(lex.slice())))]
    Float(&'a str),

    #[regex(r"-?\d+", |lex| trim_leading_0(lex.slice()))]
    Integer(&'a str),

    #[token(":")]
    Colon,

    #[regex(r"[a-zA-Z_][a-zA-Z\-_0-9]*", |lex| Woc::Borrowed(lex.slice()))]
    #[regex(r"`([^\\`]|\\\\|\\`)*`", |lex| quoting::unescape_quoted_identifier(lex.slice()))]
    Identifier(Woc<'a, String, str>),

    #[error]
    #[regex(r"[ \r\t]+", logos::skip)]
    Error,
}

impl<'a> Display for Token<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmtResult {
        match self {
            Token::Comment(str) => write!(f, "//{}", str),
            Token::HeadingHash => write!(f, "#"),
            Token::Newline => writeln!(f),
            Token::Brac => write!(f, "["),
            Token::Ket => write!(f, "]"),
            Token::Bra => write!(f, "{{"),
            Token::Ce => write!(f, "}}"),
            Token::Paren => write!(f, "("),
            Token::Thesis => write!(f, ")"),
            Token::Comma => write!(f, ","),
            Token::Period => write!(f, "."),
            Token::String(str) => write!(f, r#""{}""#, quoting::escape_string_contents(str)),
            Token::Float(str) => write!(f, "{}", str),
            Token::Integer(str) => write!(f, "{}", str),
            Token::Colon => write!(f, ":"),
            Token::Identifier(str) => write!(f, "{}", quoting::escape_identifier(str)),
            Token::Error => panic!(),
        }
    }
}

#[cfg(test)]
#[test]
fn lex() {
    let source = r#"//This is a comment
    # [[loops].{sound, volume}]
    "$sewer/amb_drips", 0000.8
    "$sewer/amb_flies", 0.1000
    "$sewer/amb_hum", 000.0500
    
    # [moments]
    sound: "$sewer/moments/*"
    layers: 1
    first-interval-no-min: true
    interval-range: (10, 0060)
    volume-range: (0.1, 0.15)
    "#;

    let lex = Token::lexer(source);

    let tokens: Vec<_> = lex.collect();

    use Token::*;
    assert_eq!(
        tokens.as_slice(),
        &[
            Comment("This is a comment"),
            Newline,
            HeadingHash,
            Brac,
            Brac,
            Identifier(Woc::Borrowed("loops")),
            Ket,
            Period,
            Bra,
            Identifier(Woc::Borrowed("sound")),
            Comma,
            Identifier(Woc::Borrowed("volume")),
            Ce,
            Ket,
            Newline,
            String(Woc::Borrowed("$sewer/amb_drips")),
            Comma,
            Float("0.8"),
            Newline,
            String(Woc::Borrowed("$sewer/amb_flies")),
            Comma,
            Float("0.1"),
            Newline,
            String(Woc::Borrowed("$sewer/amb_hum")),
            Comma,
            Float("0.05"),
            Newline,
            Newline,
            HeadingHash,
            Brac,
            Identifier(Woc::Borrowed("moments")),
            Ket,
            Newline,
            Identifier(Woc::Borrowed("sound")),
            Colon,
            String(Woc::Borrowed("$sewer/moments/*")),
            Newline,
            Identifier(Woc::Borrowed("layers")),
            Colon,
            Integer("1"),
            Newline,
            Identifier(Woc::Borrowed("first-interval-no-min")),
            Colon,
            Identifier(Woc::Borrowed("true")),
            Newline,
            Identifier(Woc::Borrowed("interval-range")),
            Colon,
            Paren,
            Integer("10"),
            Comma,
            Integer("60"),
            Thesis,
            Newline,
            Identifier(Woc::Borrowed("volume-range")),
            Colon,
            Paren,
            Float("0.1"),
            Comma,
            Float("0.15"),
            Thesis,
            Newline
        ][..]
    );
}
