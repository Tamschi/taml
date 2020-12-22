#![doc(html_root_url = "https://docs.rs/taml/0.0.3")]
#![warn(clippy::pedantic)]
#![allow(clippy::trivially_copy_pass_by_ref)]

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
