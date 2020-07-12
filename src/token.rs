use {
    logos::Logos,
    std::fmt::{Display, Formatter, Result as fmtResult},
};

#[derive(Logos, Debug, Clone, Copy, PartialEq)]
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

    #[regex(r#""([^\\"]|\\\\|\\")*""#)]
    String(&'a str),

    #[regex(r"-?\d+\.\d+")]
    Float(&'a str),

    #[regex(r"-?\d+")]
    Integer(&'a str),

    #[token(":")]
    Colon,

    #[regex(r"[a-zA-Z_][a-zA-Z\-_\d]*")]
    Identifier(&'a str),

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
            Token::String(str) => write!(f, r#""{}""#, str),
            Token::Float(str) => write!(f, "{}", str),
            Token::Integer(str) => write!(f, "{}", str),
            Token::Colon => write!(f, ":"),
            Token::Identifier(str) => write!(f, "{}", str),
            Token::Error => panic!(),
        }
    }
}

#[cfg(test)]
#[test]
fn lex() {
    let source = r#"//This is a comment
    # [[loops].{sound, volume}]
    "$sewer/amb_drips", 0.8
    "$sewer/amb_flies", 0.1
    "$sewer/amb_hum", 0.05
    
    # [moments]
    sound: "$sewer/moments/*"
    layers: 1
    first-interval-no-min: true
    interval-range: (10, 60)
    volume-range: (0.1, 0.15)
    "#;

    let lex = Token::lexer(source);

    let tokens: Vec<_> = lex.collect();

    use Token::*;
    assert_eq!(
        tokens.as_slice(),
        &[
            // Comment
            Comment(""),
            Newline,
            // Heading
            HeadingHash,
            Brac,
            Brac,
            Identifier(""),
            Ket,
            Period,
            Bra,
            Identifier(""),
            Comma,
            Identifier(""),
            Ce,
            Ket,
            Newline,
            // Table
            String(""),
            Comma,
            Float(""),
            Newline,
            String(""),
            Comma,
            Float(""),
            Newline,
            String(""),
            Comma,
            Float(""),
            Newline,
            // Empty line
            Newline,
            // Heading
            HeadingHash,
            Brac,
            Identifier(""),
            Ket,
            Newline,
            // Various key value pairs
            Identifier(""),
            Colon,
            String(""),
            Newline,
            Identifier(""),
            Colon,
            Integer(""),
            Newline,
            Identifier(""),
            Colon,
            Identifier(""),
            Newline,
            Identifier(""),
            Colon,
            Paren,
            Integer(""),
            Comma,
            Integer(""),
            Thesis,
            Newline,
            Identifier(""),
            Colon,
            Paren,
            Float(""),
            Comma,
            Float(""),
            Thesis,
            Newline
        ][..]
    );
}
