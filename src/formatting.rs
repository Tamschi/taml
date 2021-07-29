use crate::token::Token;

#[derive(Debug)]
pub struct CanonicalFormatScanner {
	state: State,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Recommendation {
	Recommended,
	Required,
	PrependSpace,
	PrependSpaceRequired,
	PrependNewline,
	PrependTwoNewlines,
	SkipToken,
}

#[derive(Debug, PartialEq, Eq)]
enum State {
	MultiNewlineOrInitial,
	Hashed,
	SingleNewline,
	Identifier,
	Number,
	Comment,
	Error,
	Other,
	ColonOrComma,
}

impl CanonicalFormatScanner {
	#[must_use]
	pub fn new() -> Self {
		Self {
			state: State::MultiNewlineOrInitial,
		}
	}
}

impl Default for CanonicalFormatScanner {
	fn default() -> Self {
		Self::new()
	}
}

impl CanonicalFormatScanner {
	/// # Panics
	///
	/// This function panics in some cases where the input could not have been created by correctly parsing a text file.
	pub fn next<Position>(&mut self, token: &Token<Position>) -> Recommendation {
		#[allow(clippy::match_same_arms)]
		let recommendation = match (&self.state, token) {
			(State::Error, _) | (_, Token::Error) => Recommendation::PrependSpaceRequired,

			(State::Comment, Token::Newline) => Recommendation::Required,
			(State::Comment, _) => {
				panic!("Invalid token sequence: Comments can only be followed by newlines")
			}

			(State::MultiNewlineOrInitial, Token::Newline) => Recommendation::SkipToken,
			(State::SingleNewline, Token::Newline) => Recommendation::Recommended,
			(_, Token::Newline) => Recommendation::Required,

			(State::MultiNewlineOrInitial, Token::HeadingHashes(_)) => Recommendation::Required,
			(State::SingleNewline, Token::HeadingHashes(_)) => Recommendation::PrependNewline,
			(_, Token::HeadingHashes(_)) => Recommendation::PrependTwoNewlines,
			(State::Hashed, _) => Recommendation::PrependSpace,

			(State::Identifier, Token::Identifier(_)) => Recommendation::PrependSpaceRequired,

			(
				State::Number,
				Token::Decimal(_)
				| Token::Integer(_)
				| Token::InvalidZeroPrefixedDecimal(_)
				| Token::InvalidZeroPrefixedInteger(_),
			) => Recommendation::PrependSpaceRequired,

			(State::SingleNewline | State::MultiNewlineOrInitial, Token::Comment(_)) => {
				Recommendation::Required
			}
			(_, Token::Comment(_)) => Recommendation::PrependSpace,

			(State::ColonOrComma, _) => Recommendation::PrependSpace,

			(_, _) => Recommendation::Required,
		};

		self.state = match token {
			Token::HeadingHashes(_) => State::Hashed,
			Token::Newline
				if self.state == State::MultiNewlineOrInitial
					|| self.state == State::SingleNewline =>
			{
				State::MultiNewlineOrInitial
			}
			Token::Newline => State::SingleNewline,
			Token::Comment(_) => State::Comment,
			Token::Decimal(_)
			| Token::Integer(_)
			| Token::InvalidZeroPrefixedDecimal(_)
			| Token::InvalidZeroPrefixedInteger(_) => State::Number,
			Token::Identifier(_) => State::Identifier,
			Token::Colon | Token::Comma => State::ColonOrComma,
			Token::Error => State::Error,

			// Intentionally not `_` so that this fails to compile when tokens are added.
			// When adding match arms here, also add them above if necessary.
			Token::Brac
			| Token::Ket
			| Token::Bra
			| Token::Ce
			| Token::Paren
			| Token::Thesis
			| Token::Period
			| Token::String(_)
			| Token::DataLiteral(_) => State::Other,
		};

		recommendation
	}
}

//TODO: Test stability
