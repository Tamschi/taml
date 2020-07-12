use logos::Logos;

#[derive(Logos, Debug, PartialEq)]
pub enum Token {
    #[regex(r"//[^\n]+")]
    Comment,

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
    String,

    #[regex(r"-?\d+\.\d+")]
    Float,

    #[regex(r"-?\d+")]
    Integer,

    #[token(":")]
    Colon,

    #[regex(r"[a-zA-Z_][a-zA-Z\-_\d]*")]
    Identifier,

    #[error]
    #[regex(r"[ \r\t]+", logos::skip)]
    Error,
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
            Comment,
            Newline,
            // Heading
            HeadingHash,
            Brac,
            Brac,
            Identifier,
            Ket,
            Period,
            Bra,
            Identifier,
            Comma,
            Identifier,
            Ce,
            Ket,
            Newline,
            // Table
            String,
            Comma,
            Float,
            Newline,
            String,
            Comma,
            Float,
            Newline,
            String,
            Comma,
            Float,
            Newline,
            // Empty line
            Newline,
            // Heading
            HeadingHash,
            Brac,
            Identifier,
            Ket,
            Newline,
            // Various key value pairs
            Identifier,
            Colon,
            String,
            Newline,
            Identifier,
            Colon,
            Integer,
            Newline,
            Identifier,
            Colon,
            Identifier,
            Newline,
            Identifier,
            Colon,
            Paren,
            Integer,
            Comma,
            Integer,
            Thesis,
            Newline,
            Identifier,
            Colon,
            Paren,
            Float,
            Comma,
            Float,
            Thesis,
            Newline
        ][..]
    );
}
