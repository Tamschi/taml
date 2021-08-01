use crate::DataLiteral;
use cervine::Cow;
use gnaw::Unshift as _;
use lazy_transform_str::{Transform as _, TransformedPart};
use logos::Logos;
use smartstring::alias::String;
use std::{
	fmt::{Display, Formatter, Result as fmtResult},
	iter,
	ops::Range,
};
use tap::Tap;

/// Data structure for **invalid** data literals (`<…:…>`).
///
/// Unlike in [`DataLiteral`], strings are not unescaped in order to preserve the `'\r'` vs `'\\r'` distinction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidDataLiteral<'a, Position> {
	pub encoding: &'a str,
	pub encoding_span: Range<Position>,
	pub unencoded_data: &'a str,
	pub unencoded_data_span: Range<Position>,
}

#[must_use = "pure function"]
pub fn escape_unencoded_data(string: &str) -> Cow<String, str> {
	string.transform(|rest| match rest.unshift().unwrap() {
		c @ ('\\' | '>') => {
			let mut changed = String::from(r"\");
			changed.push(c);
			TransformedPart::Changed(changed)
		}
		'\r' => TransformedPart::Changed("\\r".into()),
		_ => TransformedPart::Unchanged,
	})
}

macro_rules! define_escape {
	($name:ident, delimiter = $delimiter:literal, always_quote = $always_quote:literal) => {
		fn $name(string: &str) -> Cow<String, str> {
			let mut quote = $always_quote
				|| match string.chars().next() {
					Some(first) => first == '-' || first.is_ascii_digit(),
					None => true,
				};
			let escaped_name = string.transform(|rest| match rest.unshift().unwrap() {
				c @ ('\\' | $delimiter) => {
					quote = true;
					let mut changed = String::from(r"\");
					changed.push(c);
					TransformedPart::Changed(changed)
				}
				'\r' => {
					quote = true;
					TransformedPart::Changed("\\r".into())
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
				let mut quoted = String::from(concat!($delimiter));
				quoted.push_str(&escaped_name);
				quoted.push($delimiter);
				Cow::Owned(quoted)
			} else {
				escaped_name
			}
		}
	};
}

define_escape!(escape_string, delimiter = '"', always_quote = true);
define_escape!(escape_identifier, delimiter = '`', always_quote = false);

fn unescape_verbatim_and_r_to_carriage_return(string: &str) -> Cow<String, str> {
	let mut escaped = false;
	string.transform(|rest| {
		match rest.unshift().unwrap() {
			'\\' if !escaped => {
				escaped = true;
				return TransformedPart::Changed(String::new());
			}
			'r' if escaped => TransformedPart::Changed("\r".into()),
			_ => {
				// This function can be really lenient only because we already filter out invalid escapes with the lexer regex.
				TransformedPart::Unchanged
			}
		}
		.tap(|_| escaped = false)
	})
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

	#[regex("\r?\n")]
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

	#[regex(r#""([^\\"\r]|\\\\|\\"|\\r)*""#, priority = 1000, callback = |lex| unescape_verbatim_and_r_to_carriage_return(&lex.slice()[1..lex.slice().len() - 1]))]
	String(Cow<'a, String, str>),

	/// Unlike in [`Token::String`], the quoted string is not unescaped in order to preserve the `'\r'` vs `'\\r'` distinction.
	#[regex(r#""([^\\"]|\\\\|\\"|\\r)*""#, |lex| &lex.slice()[1..lex.slice().len() - 1])]
	InvalidStringWithVerbatimCarriageReturn(&'a str),

	#[regex(r#"<[a-zA-Z_][a-zA-Z\-_0-9]*:([^\\>]|\\\\|\\>)*>"#, |lex| {
		let (encoding, unencoded_data) = lex.slice()['<'.len_utf8()..lex.slice().len() - '>'.len_utf8()].split_once(':').unwrap(); //FIXME: Broken if identifier contains `:`.
		DataLiteral {
			encoding: Cow::Borrowed(encoding),
			encoding_span: lex.span().start + '<'.len_utf8()..lex.span().start + '<'.len_utf8() + encoding.len(),
			unencoded_data: unescape_verbatim_and_r_to_carriage_return(unencoded_data),
			unencoded_data_span: lex.span().end - 1 - unencoded_data.len()..lex.span().end - 1,
		}
	})]
	#[regex(r#"<`([^\\`\r]|\\\\|\\`|\\r)*`:([^\\>\r]|\\\\|\\>|\\r)*>"#, priority = 1000, callback = |lex| {
		let (encoding, unencoded_data) = lex.slice()['<'.len_utf8()..lex.slice().len() - '>'.len_utf8()].split_once(':').unwrap(); //FIXME: Broken if identifier contains `:`.
		DataLiteral {
			encoding: unescape_verbatim_and_r_to_carriage_return(&encoding['`'.len_utf8()..encoding.len()-'`'.len_utf8()]),
			encoding_span: lex.span().start + '`'.len_utf8()..lex.span().start + '`'.len_utf8() + encoding.len(),
			unencoded_data: unescape_verbatim_and_r_to_carriage_return(unencoded_data),
			unencoded_data_span: lex.span().end - '>'.len_utf8() - unencoded_data.len()..lex.span().end - '>'.len_utf8(),
		}
	})]
	DataLiteral(DataLiteral<'a, Position>),

	/// Unlike in [`Token::DataLiteral`], the strings are not unescaped in order to preserve the `'\r'` vs `'\\r'` distinction.
	#[regex(r#"<`([^\\`]|\\\\|\\`|\\r)*`:([^\\>]|\\\\|\\>|\\r)*>"#, |lex| {
		let (encoding, unencoded_data) = lex.slice()[1..lex.slice().len() - 1].split_once(':').unwrap();
		InvalidDataLiteral {
			encoding,
			encoding_span: lex.span().start + '`'.len_utf8()..lex.span().start + '`'.len_utf8() + encoding.len(),
			unencoded_data,
			unencoded_data_span: lex.span().end - '>'.len_utf8() - unencoded_data.len()..lex.span().end - '>'.len_utf8(),
		}
	})]
	InvalidDataLiteralWithVerbatimCarriageReturn(InvalidDataLiteral<'a, Position>),

	#[regex(r"-?(0|[1-9]\d*)\.\d+", |lex| trim_trailing_0s(lex.slice()))]
	Decimal(&'a str),

	#[regex(r"-?(0\d+)\.\d+", |lex| trim_trailing_0s(lex.slice()))]
	InvalidZeroPrefixedDecimal(&'a str),

	#[regex(r"-?(0|[1-9]\d*)", |lex| lex.slice())]
	Integer(&'a str),

	#[regex(r"-?(0\d+)", |lex| lex.slice())]
	InvalidZeroPrefixedInteger(&'a str),

	#[token(":")]
	Colon,

	#[regex(r"[a-zA-Z_][a-zA-Z\-_0-9]*", |lex| Cow::Borrowed(lex.slice()))]
	#[regex(r"`([^\\`\r]|\\\\|\\`|\\r)*`", priority = 1000, callback = |lex| unescape_verbatim_and_r_to_carriage_return(&lex.slice()['`'.len_utf8()..lex.slice().len() - '`'.len_utf8()]))]
	Identifier(Cow<'a, String, str>),

	/// Unlike in [`Token::Identifier`], the quoted string is not unescaped in order to preserve the `'\r'` vs `'\\r'` distinction.
	#[regex(r"`([^\\`\r]|\\\\|\\`|\\r)*`", |lex| lex.slice()['`'.len_utf8()..lex.slice().len() - '`'.len_utf8()])]
	InvalidIdentifierWithVerbatimCarriageReturn(&'a str),

	#[error]
	#[regex(r"[ \t]+", logos::skip)]
	Error,
}

/// # Panics
///
/// This [`Display`] implementation panics when called on [`Token::Error`].
///
/// It also panics when used on `Invalid…WithCarriageReturn` tokens that cannot be reparsed as such,
/// for example by containing improper escape sequences.
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
				write!(
					f,
					"<{}:{}>",
					escape_identifier(encoding),
					escape_unencoded_data(unencoded_data)
				)
			}
			Self::InvalidDataLiteralWithVerbatimCarriageReturn(invalid_data_literal) => write!(
				f,
				"<`{}`:{}>",
				invalid_data_literal.encoding,
				invalid_data_literal.unencoded_data // FIXME: Assert that at least the escape sequences are okay.
			),
			Token::String(str) => write!(f, "{}", escape_string(str)),
			Token::InvalidStringWithVerbatimCarriageReturn(str) => write!(f, r#""{}""#, str), // FIXME: Assert that at least the escape sequences are okay.
			Token::Decimal(str)
			| Token::Integer(str)
			| Self::InvalidZeroPrefixedDecimal(str)
			| Token::InvalidZeroPrefixedInteger(str) => write!(f, "{}", str),
			Token::Colon => write!(f, ":"),
			Token::Identifier(str) => write!(f, "{}", escape_identifier(str)),
			Token::InvalidIdentifierWithVerbatimCarriageReturn(str) => write!(f, "`{}`", str), // FIXME: Assert that at least the escape sequences are okay.
			Token::Error => panic!("Tried to `Display::fmt` `taml::token::Token::Error`."),
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
    "$sewer/amb_drips", 0.8
    "$sewer/amb_flies", 0.1000
    "$sewer/amb_hum", 0.0500

    # [moments]
    sound: "$sewer/moments/*"
    layers: 1
    first-interval-no-min: true
    interval-range: (10, 60)
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
