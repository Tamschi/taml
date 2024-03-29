use enum_properties::enum_properties;
use std::{
	borrow::Cow, convert::Into, fmt::Display, iter, ops::Range, string::String as stdString,
};

#[derive(Clone, Copy, Debug)]
pub enum DiagnosticLevel {
	Warning,
	Error,
}

pub struct DiagnosticGroupProperties {
	code: char,
}

enum_properties! {
	pub enum DiagnosticGroup: DiagnosticGroupProperties {
		Lexing { code: 'L' },
		Parsing { code: 'P' },
		Deserialising{ code: 'D' },
	}
}

pub struct DiagnosticTypeProperties {
	group: DiagnosticGroup,
	code: usize,
	level: DiagnosticLevel,
	title: &'static str,
}

enum_properties! {
	#[derive(Clone, Copy, Debug, PartialEq, Eq)]
	#[non_exhaustive]
	pub enum DiagnosticType: DiagnosticTypeProperties {
		UnrecognizedToken {
			group: DiagnosticGroup::Lexing,
			code: 0,
			level: DiagnosticLevel::Error,
			title: "Unrecognised token",
		},

		ZeroPrefixedDecimalFound {
			group: DiagnosticGroup::Lexing,
			code: 1,
			level: DiagnosticLevel::Error,
			title: "Zero-prefixed decimal found",
		},

		ZeroPrefixedIntegerFound {
			group: DiagnosticGroup::Lexing,
			code: 2,
			level: DiagnosticLevel::Error,
			title: "Zero-prefixed integer found",
		},

		VerbatimCarriageReturnInsideLiteral {
			group: DiagnosticGroup::Lexing,
			code: 3,
			level: DiagnosticLevel::Error,
			title: "Verbatim U+000D CARRIAGE RETURN (CR) inside literal",
		},

		HeadingTooDeep {
			group: DiagnosticGroup::Parsing,
			code: 1,
			level: DiagnosticLevel::Error,
			title: "Heading too deep",
		},

		SubsectionInTabularSection {
			group: DiagnosticGroup::Parsing,
			code: 2,
			level: DiagnosticLevel::Error,
			title: "Subsection in tabular section",
		},

		MissingVariantIdentifier {
			group: DiagnosticGroup::Parsing,
			code: 3,
			level: DiagnosticLevel::Error,
			title: "Missing variant identifier",
		},

		ExpectedKeyValuePair {
			group: DiagnosticGroup::Parsing,
			code: 4,
			level: DiagnosticLevel::Error,
			title: "Expected key-value pair",
		},

		ExpectedValue {
			group: DiagnosticGroup::Parsing,
			code: 5,
			level: DiagnosticLevel::Error,
			title: "Expected value",
		},

		UnclosedList {
			group: DiagnosticGroup::Parsing,
			code: 6,
			level: DiagnosticLevel::Error,
			title: "Unclosed list",
		},

		ValuesLineTooShort {
			group: DiagnosticGroup::Parsing,
			code: 7,
			level: DiagnosticLevel::Error,
			title: "Values line too short",
		},

		ExpectedListIdentifier {
			group: DiagnosticGroup::Parsing,
			code: 8,
			level: DiagnosticLevel::Error,
			title: "Expected list identifier",
		},

		ExpectedTabularPathSegment {
			group: DiagnosticGroup::Parsing,
			code: 9,
			level: DiagnosticLevel::Error,
			title: "Expected (tabular) path segment",
		},

		UnclosedTabularPathMultiSegment {
			group: DiagnosticGroup::Parsing,
			code: 10,
			level: DiagnosticLevel::Error,
			title: "Unclosed tabular path multi segment",
		},

		ExpectedPathSegment {
			group: DiagnosticGroup::Parsing,
			code: 11,
			level: DiagnosticLevel::Error,
			title: "Expected path segment",
		},

		InvalidPathContinuation {
			group: DiagnosticGroup::Parsing,
			code: 12,
			level: DiagnosticLevel::Error,
			title: "Invalid path continuation",
		},

		KeyPreviouslyDefined {
			group: DiagnosticGroup::Parsing,
			code: 13,
			level: DiagnosticLevel::Error,
			title: "Key previously defined",
		},

		UnclosedListKey {
			group: DiagnosticGroup::Parsing,
			code: 14,
			level: DiagnosticLevel::Error,
			title: "Unclosed list key",
		},

		UnclosedTabularPathSection {
			group: DiagnosticGroup::Parsing,
			code: 15,
			level: DiagnosticLevel::Error,
			title: "Unclosed tabular path section",
		},

		DuplicateEnumInstantiation {
			group: DiagnosticGroup::Parsing,
			code: 15,
			level: DiagnosticLevel::Error,
			title: "Duplicate enum instantiation",
		},

		NonMapValueSelected {
			group: DiagnosticGroup::Parsing,
			code: 16,
			level: DiagnosticLevel::Error,
			title: "Non-map value selected",
		},

		MisplacedComment {
			group: DiagnosticGroup::Parsing,
			code: 17,
			level: DiagnosticLevel::Error,
			title: "Misplaced comment",
		},

		MisplacedHeading {
			group: DiagnosticGroup::Parsing,
			code: 18,
			level: DiagnosticLevel::Error,
			title: "Misplaced heading",
		},

		MisplacedData {
			group: DiagnosticGroup::Parsing,
			code: 19,
			level: DiagnosticLevel::Error,
			title: "Misplaced data",
		},

		CustomErrorFromVisitor {
			group: DiagnosticGroup::Deserialising,
			code: 0,
			level: DiagnosticLevel::Error,
			title: "Custom error from visitor",
		},

		InvalidType {
			group: DiagnosticGroup::Deserialising,
			code: 1,
			level: DiagnosticLevel::Error,
			title: "Invalid type",
		},

		InvalidValue {
			group: DiagnosticGroup::Deserialising,
			code: 2,
			level: DiagnosticLevel::Error,
			title: "Invalid value",
		},

		InvalidLength{
			group: DiagnosticGroup::Deserialising,
			code: 3,
			level: DiagnosticLevel::Error,
			title: "Invalid length",
		},

		UnknownVariant {
			group: DiagnosticGroup::Deserialising,
			code: 4,
			level: DiagnosticLevel::Error,
			title: "Unknown variant",
		},

		UnknownField {
			group: DiagnosticGroup::Deserialising,
			code: 5,
			level: DiagnosticLevel::Error,
			title: "Unknown field",
		},

		MissingField {
			group: DiagnosticGroup::Deserialising,
			code: 6,
			level: DiagnosticLevel::Error,
			title: "Missing field",
		},

		UnknownEncoding {
			group: DiagnosticGroup::Deserialising,
			code: 7,
			level: DiagnosticLevel::Error,
			title: "Unknown encoding",
		},

		EncodeFailed {
			group: DiagnosticGroup::Deserialising,
			code: 8,
			level: DiagnosticLevel::Error,
			title: "Encode failed",
		},
	}
}

impl Display for DiagnosticType {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "TAML-{}{:03} {}", self.group.code, self.code, self.title)
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticLabelPriority {
	Primary,
	Auxiliary,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DiagnosticLabel<Position> {
	pub caption: Option<Cow<'static, str>>,
	pub span: Option<Range<Position>>,
	pub priority: DiagnosticLabelPriority,
}

impl<Position> DiagnosticLabel<Position> {
	pub fn new<
		Caption: Into<Cow<'static, str>>,
		CaptionOption: Into<Option<Caption>>,
		SpanOption: Into<Option<Range<Position>>>,
	>(
		caption: CaptionOption,
		span: SpanOption,
		priority: DiagnosticLabelPriority,
	) -> Self {
		Self {
			caption: caption.into().map(Into::into),
			span: span.into(),
			priority,
		}
	}
}

#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostic<Position> {
	pub type_: DiagnosticType,
	pub labels: Vec<DiagnosticLabel<Position>>,
}

impl<Position> Diagnostic<Position> {
	#[must_use]
	pub fn code(&self) -> stdString {
		format!("TAML-{}{:04}", self.type_.group.code, self.type_.code)
	}

	#[must_use]
	pub fn level(&self) -> DiagnosticLevel {
		self.type_.level
	}

	#[must_use]
	pub fn message(&self) -> &str {
		self.type_.title
	}
}

pub trait Reporter<Position> {
	fn report_with(&mut self, diagnostic: impl FnOnce() -> Diagnostic<Position>) {
		self.report_many_with(|| iter::once_with(diagnostic))
	}
	fn report_many_with<I: IntoIterator<Item = Diagnostic<Position>>>(
		&mut self,
		diagnostics: impl FnOnce() -> I,
	);
}

impl<Position> Reporter<Position> for () {
	fn report_with(&mut self, _diagnostic: impl FnOnce() -> Diagnostic<Position>) {
		// Do nothing.
	}

	fn report_many_with<I: IntoIterator<Item = Diagnostic<Position>>>(
		&mut self,
		_diagnostics: impl FnOnce() -> I,
	) {
		// Do nothing.
	}
}

impl<Position> Reporter<Position> for Vec<Diagnostic<Position>> {
	fn report_many_with<I: IntoIterator<Item = Diagnostic<Position>>>(
		&mut self,
		diagnostics: impl FnOnce() -> I,
	) {
		self.extend(diagnostics())
	}
}
