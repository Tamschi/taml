#![doc(html_root_url = "https://docs.rs/taml/0.0.2")]
#[warn(clippy::pedantic)]
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
