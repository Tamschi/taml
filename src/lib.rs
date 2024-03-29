//! TAML is a configuration file format combining some aspects of Markdown, CSV, TOML, YAML and Rust.
//!
//! [![Zulip Chat](https://img.shields.io/endpoint?label=chat&url=https%3A%2F%2Fiteration-square-automation.schichler.dev%2F.netlify%2Ffunctions%2Fstream_subscribers_shield%3Fstream%3Dproject%252Ftaml)](https://iteration-square.schichler.dev/#narrow/stream/project.2Ftaml)

#![doc(html_root_url = "https://docs.rs/taml/0.0.11")]
#![warn(clippy::pedantic, missing_docs)]
#![allow(
	clippy::semicolon_if_nothing_returned,
	clippy::trivially_copy_pass_by_ref,
	clippy::result_unit_err
)]
// FIXME
#![allow(missing_docs)]

use cervine::Cow;
use core::{fmt::Debug, ops::Range};
use smartstring::alias::String;

#[cfg(doctest)]
#[doc = include_str!("../README.md")]
mod readme {}

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

/// Implemented by types usable as `Position` generic type parameter in this library.
pub trait Position: Debug + Clone + Default + PartialEq {
	/// Adds `self` to both limits of `local_range` and returns the result in [`Some`].  
	/// If this operation does not make sense, [`None`] is returned instead.
	fn offset_range(&self, local_range: Range<usize>) -> Option<Range<Self>>;
}

impl Position for usize {
	fn offset_range(&self, local_range: Range<usize>) -> Option<Range<Self>> {
		Some(self + local_range.start..self + local_range.end)
	}
}

impl Position for () {
	fn offset_range(&self, _local_range: Range<usize>) -> Option<Range<Self>> {
		None
	}
}

pub use smartstring::validate;
