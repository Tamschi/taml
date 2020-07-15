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
    pub fn next(&mut self, token: &Token) -> Recommendation {
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

            (State::Hashed, Token::HeadingHash)
            | (State::MultiNewlineOrInitial, Token::HeadingHash) => Recommendation::Required,
            (State::SingleNewline, Token::HeadingHash) => Recommendation::PrependNewline,
            (_, Token::HeadingHash) => Recommendation::PrependTwoNewlines,
            (State::Hashed, _) => Recommendation::PrependSpace,

            (State::Identifier, Token::Identifier(_)) => Recommendation::PrependSpaceRequired,

            (State::Number, Token::Float(_)) | (State::Number, Token::Integer(_)) => {
                Recommendation::PrependSpaceRequired
            }

            (State::SingleNewline, Token::Comment(_))
            | (State::MultiNewlineOrInitial, Token::Comment(_)) => Recommendation::Required,
            (_, Token::Comment(_)) => Recommendation::PrependSpace,

            (State::ColonOrComma, _) => Recommendation::PrependSpace,

            (_, _) => Recommendation::Required,
        };

        self.state = match token {
            Token::HeadingHash => State::Hashed,
            Token::Newline
                if self.state == State::MultiNewlineOrInitial
                    || self.state == State::SingleNewline =>
            {
                State::MultiNewlineOrInitial
            }
            Token::Newline => State::SingleNewline,
            Token::Comment(_) => State::Comment,
            Token::Float(_) | Token::Integer(_) => State::Number,
            Token::Identifier(_) => State::Identifier,
            Token::Colon | Token::Comma => State::ColonOrComma,
            Token::Error => State::Error,
            _ => State::Other,
        };

        recommendation
    }
}

//TODO: Test stability
