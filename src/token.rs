use crate::DataLiteral;
use cervine::Cow;
use gnaw::Unshift as _;
use lazy_transform_str::{
	escape_double_quotes, unescape_backslashed_verbatim, Transform as _, TransformedPart,
};
use logos::Logos;
use smartstring::alias::String;
use std::{
	fmt::{Display, Formatter, Result as fmtResult},
	iter,
};

#[must_use = "pure function"]
pub fn escape_greater(string: &str) -> Cow<String, str> {
	string.transform(|rest| match rest.unshift().unwrap() {
		c @ ('\\' | '>') => {
			let mut changed = String::from(r"\");
			changed.push(c);
			TransformedPart::Changed(changed)
		}
		_ => TransformedPart::Unchanged,
	})
}

fn escape_identifier(string: &str) -> Cow<String, str> {
	let mut quote = match string.chars().next() {
		Some(first) => first == '-' || first.is_ascii_digit(),
		None => true,
	};
	let escaped_name = string.transform(|rest| match rest.unshift().unwrap() {
		c @ ('\\' | '`') => {
			quote = true;
			let mut changed = String::from(r"\");
			changed.push(c);
			TransformedPart::Changed(changed)
		}
		c => {
			if !(('a'..='z').contains(&c)
				|| ('A'..='Z').contains(&c)
				|| c == '-' || c == '_'
				|| ('0'..'9').contains(&c))
			{
				quote = true
			}
			TransformedPart::Unchanged
		}
	});
	if quote {
		let mut quoted = String::from("`");
		quoted.push_str(&escaped_name);
		quoted.push('`');
		Cow::Owned(quoted)
	} else {
		escaped_name
	}
}

fn unescape_quoted_identifier(string: &str) -> Cow<String, str> {
	assert!(string.starts_with('`'));
	assert!(string.ends_with('`'));
	let string = &string['`'.len_utf8()..string.len() - '`'.len_utf8()];
	let mut escaped = false;
	string.transform(|rest| match rest.unshift().unwrap() {
		'\\' if !escaped => {
			escaped = true;
			TransformedPart::Changed(String::new())
		}
		_ => {
			// This function can be really lenient only because we already filter out invalid escapes with the lexer regex.
			escaped = false;
			TransformedPart::Unchanged
		}
	})
}

fn trim_leading_0s(mut s: &str) -> &str {
	while s.len() >= 2 && s.as_bytes()[0] == b'0' && (b'0'..=b'9').contains(&s.as_bytes()[1]) {
		s = &s[1..]
	}
	s
}

fn trim_trailing_0s(mut s: &str) -> &str {
	while s.len() >= 2
		&& s.as_bytes()[s.len() - 1] == b'0'
		&& (b'0'..=b'9').contains(&s.as_bytes()[s.len() - 2])
	{
		s = &s[..s.len() - 1]
	}
	s
}

#[derive(Logos, Debug, Clone, PartialEq, Eq)]
#[logos(type Position = usize)]
pub enum Token<'a, Position> {
	#[regex(r"//[^\r\n]+", |lex| lex.slice()[2..].trim_end_matches([' ', '\t'].as_ref()))]
	Comment(&'a str),

	#[regex("#+", |lex| lex.slice().chars().count())]
	HeadingHashes(usize),

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

	#[regex(r#""([^\\"]|\\\\|\\")*""#, |lex| unescape_backslashed_verbatim(&lex.slice()[1..lex.slice().len() - 1]))]
	String(Cow<'a, String, str>),

	#[regex(r#"<[a-zA-Z_][a-zA-Z\-_0-9]*:([^\\>]|\\\\|\\>)*>"#, |lex| {
		let (encoding, unencoded_data) = lex.slice()[1..lex.slice().len() - 1].split_once(':').unwrap();
		DataLiteral {
			encoding: Cow::Borrowed(encoding),
			encoding_span: lex.span().start + 1..lex.span().start + 1 + encoding.len(),
			unencoded_data: unescape_backslashed_verbatim(unencoded_data),
			unencoded_data_span: lex.span().end - 1 - unencoded_data.len()..lex.span().end - 1,
		}
	})]
	#[regex(r#"<`([^\\`]|\\\\|\\`)*`:([^\\>]|\\\\|\\>)*>"#, |lex| {
		let (encoding, unencoded_data) = lex.slice()[1..lex.slice().len() - 1].split_once(':').unwrap();
		DataLiteral {
			encoding: unescape_quoted_identifier(encoding),
			encoding_span: lex.span().start + 1..lex.span().start + 1 + encoding.len(),
			unencoded_data: unescape_backslashed_verbatim(unencoded_data),
			unencoded_data_span: lex.span().end - 1 - unencoded_data.len()..lex.span().end - 1,
		}
	})]
	DataLiteral(DataLiteral<'a, Position>),

	#[regex(r"-?\d+\.\d+", |lex| trim_trailing_0s(trim_leading_0s(lex.slice())))]
	Decimal(&'a str),

	#[regex(r"-?\d+", |lex| trim_leading_0s(lex.slice()))]
	Integer(&'a str),

	#[token(":")]
	Colon,

	#[regex(r"[a-zA-Z_][a-zA-Z\-_0-9]*", |lex| Cow::Borrowed(lex.slice()))]
	#[regex(r"`([^\\`]|\\\\|\\`)*`", |lex| unescape_quoted_identifier(lex.slice()))]
	Identifier(Cow<'a, String, str>),

	#[error]
	#[regex(r"[ \r\t]+", logos::skip)]
	Error,
}

impl<'a, Position> Display for Token<'a, Position> {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmtResult {
		match self {
			Token::Comment(str) => write!(f, "//{}", str),
			Token::HeadingHashes(count) => {
				write!(f, "{}", iter::repeat('#').take(*count).collect::<String>())
			}
			Token::Newline => writeln!(f),
			Token::Brac => write!(f, "["),
			Token::Ket => write!(f, "]"),
			Token::Bra => write!(f, "{{"),
			Token::Ce => write!(f, "}}"),
			Token::Paren => write!(f, "("),
			Token::Thesis => write!(f, ")"),
			Token::Comma => write!(f, ","),
			Token::Period => write!(f, "."),
			Token::DataLiteral(DataLiteral {
				encoding,
				unencoded_data,
				..
			}) => {
				write!(f, "<{}:{}>", encoding, escape_greater(unencoded_data))
			}
			Token::String(str) => write!(f, r#""{}""#, escape_double_quotes(str)),
			Token::Decimal(str) | Token::Integer(str) => write!(f, "{}", str),
			Token::Colon => write!(f, ":"),
			Token::Identifier(str) => write!(f, "{}", escape_identifier(str)),
			Token::Error => panic!(),
		}
	}
}

#[cfg(test)]
#[test]
#[allow(clippy::enum_glob_use)]
fn lex() {
	use Token::*;

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

	assert_eq!(
		tokens.as_slice(),
		&[
			Comment("This is a comment"),
			Newline,
			HeadingHashes(1),
			Brac,
			Brac,
			Identifier(Cow::Borrowed("loops")),
			Ket,
			Period,
			Bra,
			Identifier(Cow::Borrowed("sound")),
			Comma,
			Identifier(Cow::Borrowed("volume")),
			Ce,
			Ket,
			Newline,
			String(Cow::Borrowed("$sewer/amb_drips")),
			Comma,
			Decimal("0.8"),
			Newline,
			String(Cow::Borrowed("$sewer/amb_flies")),
			Comma,
			Decimal("0.1"),
			Newline,
			String(Cow::Borrowed("$sewer/amb_hum")),
			Comma,
			Decimal("0.05"),
			Newline,
			Newline,
			HeadingHashes(1),
			Brac,
			Identifier(Cow::Borrowed("moments")),
			Ket,
			Newline,
			Identifier(Cow::Borrowed("sound")),
			Colon,
			String(Cow::Borrowed("$sewer/moments/*")),
			Newline,
			Identifier(Cow::Borrowed("layers")),
			Colon,
			Integer("1"),
			Newline,
			Identifier(Cow::Borrowed("first-interval-no-min")),
			Colon,
			Identifier(Cow::Borrowed("true")),
			Newline,
			Identifier(Cow::Borrowed("interval-range")),
			Colon,
			Paren,
			Integer("10"),
			Comma,
			Integer("60"),
			Thesis,
			Newline,
			Identifier(Cow::Borrowed("volume-range")),
			Colon,
			Paren,
			Decimal("0.1"),
			Comma,
			Decimal("0.15"),
			Thesis,
			Newline
		][..]
	);
}
