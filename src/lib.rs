#![doc(html_root_url = "https://docs.rs/taml/0.0.8")]
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

/// Shared variant payload data structure for decoded strings (`<…:…>`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decoded<'a, Position> {
	pub encoding: Cow<'a, String, str>,
	pub encoding_span: Range<Position>,
	pub decoded: Cow<'a, String, str>,
	pub decoded_span: Range<Position>,
}
