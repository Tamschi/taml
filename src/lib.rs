#![doc(html_root_url = "https://docs.rs/taml/0.0.9")]
#![warn(clippy::pedantic)]
#![allow(clippy::result_unit_err)]
#![allow(clippy::trivially_copy_pass_by_ref)]

use cervine::Cow;
use core::ops::Range;
use smartstring::alias::String;

#[cfg(doctest)]
pub mod readme {
	doc_comment::doctest!("../README.md");
}

pub mod diagnostics;
pub mod formatting;
pub mod parsing;
mod token;

pub use parsing::parse;
pub use token::Token;

/// Shared variant payload data structure for data literals (`<…:…>`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataLiteral<'a, Position> {
	pub encoding: Cow<'a, String, str>,
	pub encoding_span: Range<Position>,
	pub unencoded_data: Cow<'a, String, str>,
	pub unencoded_data_span: Range<Position>,
}
