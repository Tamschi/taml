use crate::{
	diagnostics::{Diagnostic, DiagnosticLabel, DiagnosticLabelPriority, DiagnosticType, Reporter},
	token::Token as lexerToken,
	DataLiteral, Position,
};
use cervine::Cow;
use debugless_unwrap::DebuglessUnwrap as _;
use indexmap::{map, IndexMap};
use smartstring::alias::String;
use std::{
	borrow::Borrow,
	fmt::Debug,
	hash::Hash,
	iter::{self, Peekable},
	ops::{Deref, Range},
};
use try_match::try_match;

//FIXME: This entire file really needs to be refactored to filter out error tokens early,
// and to then type-safely match only on a subset. (Time for an enum subsetting macro?)

pub trait IntoToken<'a, Position> {
	fn into_token(self) -> Token<'a, Position>;
}

#[derive(Debug)]
pub struct Token<'a, Position> {
	token: lexerToken<'a, Position>,
	span: Range<Position>,
}

impl<'a> IntoToken<'a, ()> for lexerToken<'a, ()> {
	fn into_token(self) -> Token<'a, ()> {
		Token {
			token: self,
			span: ()..(),
		}
	}
}

impl<'a, Position> IntoToken<'a, Position> for (lexerToken<'a, Position>, Range<Position>) {
	fn into_token(self) -> Token<'a, Position> {
		Token {
			token: self.0,
			span: self.1,
		}
	}
}

#[derive(Debug, Clone)]
pub struct Taml<'a, Position> {
	pub value: TamlValue<'a, Position>,
	pub span: Range<Position>,
}

impl<'a, Position> Taml<'a, Position> {
	fn unwrap_list_mut(&mut self) -> &mut List<'a, Position> {
		match &mut self.value {
			TamlValue::List(list) => list,
			_ => panic!("Expected list."),
		}
	}

	fn unwrap_map_mut(&mut self) -> &mut Map<'a, Position> {
		match &mut self.value {
			TamlValue::Map(map) => map,
			_ => panic!("Expected map."),
		}
	}

	fn unwrap_variant_structured_mut(&mut self) -> &mut Map<'a, Position> {
		match &mut self.value {
			TamlValue::EnumVariant {
				key: _,
				payload: VariantPayload::Structured(map),
			} => map,
			_ => panic!("Expected structured variant."),
		}
	}
}

#[derive(Debug, Clone)]
pub enum TamlValue<'a, Position> {
	String(Cow<'a, String, str>),
	DataLiteral(DataLiteral<'a, Position>),
	Integer(&'a str),
	Decimal(&'a str),
	List(List<'a, Position>),
	Map(Map<'a, Position>),
	EnumVariant {
		key: Key<'a, Position>,
		payload: VariantPayload<'a, Position>,
	},
}

#[derive(Debug, Clone)]
pub enum VariantPayload<'a, Position> {
	Structured(Map<'a, Position>),
	Tuple(List<'a, Position>),
	Unit,
}

struct PathSegment<'a, P: Position> {
	base: Vec<BasicPathElement<'a, P>>,
	tabular: Option<TabularPathSegment<'a, P>>,
}

#[derive(Clone)]
struct BasicPathElement<'a, P: Position> {
	key: BasicPathElementKey<'a, P>,
	variant: Option<Key<'a, P>>,
}

impl<'a, P: Position> BasicPathElement<'a, P> {
	fn span(&self) -> Range<P> {
		self.variant.as_ref().map_or_else(
			|| self.key.span().clone(),
			|variant| self.key.span().start.clone()..variant.span.end.clone(),
		)
	}
}

#[derive(Clone)]
enum BasicPathElementKey<'a, Position> {
	Plain(Key<'a, Position>),
	List {
		key: Key<'a, Position>,
		span: Range<Position>,
	},
}

impl<'a, Position> BasicPathElementKey<'a, Position> {
	fn span(&self) -> &Range<Position> {
		match self {
			BasicPathElementKey::Plain(key) => &key.span,
			BasicPathElementKey::List { span, .. } => span,
		}
	}
}

struct TabularPathSegment<'a, P: Position> {
	base: Vec<BasicPathElement<'a, P>>,
	multi: Option<(Vec<TabularPathSegment<'a, P>>, Range<P>)>,
}

#[derive(Clone, Debug)]
pub struct Key<'a, Position> {
	pub name: Cow<'a, String, str>,
	pub span: Range<Position>,
}

impl<'a, Position> Borrow<str> for Key<'a, Position> {
	fn borrow(&self) -> &str {
		self.name.as_ref()
	}
}

impl<'a, Position> Deref for Key<'a, Position> {
	type Target = str;
	fn deref(&self) -> &Self::Target {
		self.name.as_ref()
	}
}

impl<'a, Position> AsRef<str> for Key<'a, Position> {
	fn as_ref(&self) -> &str {
		self.name.as_ref()
	}
}

impl<'a, Position> Hash for Key<'a, Position> {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		self.name.hash(state)
	}
}
impl<'a, Position, Rhs: AsRef<str> + ?Sized> PartialEq<Rhs> for Key<'a, Position> {
	fn eq(&self, other: &Rhs) -> bool {
		self.as_ref() == other.as_ref()
	}
}
impl<'a, Position> Eq for Key<'a, Position> {}

pub type Map<'a, Position> = IndexMap<Key<'a, Position>, Taml<'a, Position>>;
pub type MapIter<'iter, 'taml, Position> =
	map::Iter<'iter, Key<'taml, Position>, Taml<'taml, Position>>;

pub type List<'a, Position> = Vec<Taml<'a, Position>>;
pub type ListIter<'iter, 'taml, Position> = std::slice::Iter<'iter, Taml<'taml, Position>>;

impl<'a, P: Position> TabularPathSegment<'a, P> {
	fn arity(&self) -> usize {
		match &self.multi {
			None => 1,
			Some(multi) => multi.0.iter().map(Self::arity).sum(),
		}
	}

	fn assign(
		&self,
		selection: &mut Map<'a, P>,
		values: &mut impl Iterator<Item = Taml<'a, P>>,
		reporter: &mut impl Reporter<P>,
	) -> Result<(), ()> {
		#![allow(clippy::items_after_statements)]
		#![allow(clippy::option_if_let_else)]

		if self.base.is_empty() && self.multi.is_none() {
			//TODO: Make sure these aren't accepted by the parser.
			unreachable!("Completely empty tabular path segments are invalid.")
		} else if self.base.is_empty() {
			for child in &self.multi.as_ref().unwrap().0 {
				child.assign(selection, values, reporter)?
			}

			return Ok(());
		}

		let selection = instantiate(
			selection,
			self.base.iter().take(self.base.len() - 1).cloned(),
			reporter,
		)?;

		fn placeholder<'a, P: Position>(position: P) -> Taml<'a, P> {
			Taml {
				span: position.clone()..position,
				value: TamlValue::String(Cow::Borrowed("PLACEHOLDER")),
			}
		}

		let selection: &mut Taml<'a, P> = match &self.base.last().unwrap().key {
			BasicPathElementKey::Plain(key) => try_match!(
				map::Entry::Vacant(vacant)
				= selection.entry(key.clone())
				=> vacant.insert(placeholder(key.span.start.clone()))
			)
			.debugless_unwrap(),

			BasicPathElementKey::List { key, span } => {
				let list = selection
					.entry(key.clone())
					.or_insert_with(|| Taml {
						value: TamlValue::List(vec![]),
						span: span.clone(),
					})
					.unwrap_list_mut();
				list.push(placeholder(span.start.clone()));
				list.last_mut().unwrap()
			}
		};

		let variant = self.base.last().unwrap().variant.as_ref();

		//TODO: Report not enough values.
		if let Some(multi) = &self.multi {
			let selection = if let Some(variant) = variant {
				*selection = Taml {
					span: variant.span.start.clone()..multi.1.end.clone(),
					value: TamlValue::EnumVariant {
						key: variant.clone(),
						payload: VariantPayload::Structured(Map::new()),
					},
				};
				selection.unwrap_variant_structured_mut()
			} else {
				{
					*selection = Taml {
						span: multi.1.clone(),
						value: TamlValue::Map(IndexMap::new()),
					};
					selection.unwrap_map_mut()
				}
			};
			for child in &multi.0 {
				child.assign(selection, values.by_ref(), reporter)?
			}
			Ok(())
		} else {
			if let Some(variant) = variant {
				*selection = match values.next().ok_or(())? {
					Taml {
						span,
						value: TamlValue::List(list),
					} => Taml {
						span,
						value: TamlValue::EnumVariant {
							key: variant.clone(),
							payload: VariantPayload::Tuple(list),
						},
					},
					_ => return Err(()), //TODO: Report
				};
			} else {
				*selection = values.next().ok_or(())?;
			}
			Ok(())
		}
	}
}

/// Parses TAML tokens into a map representing the contained structure.
///
/// Any errors and warnings are reported via `reporter`.
///
/// # Errors
///
/// Iff the given input from `iter` is invalid.
pub fn parse<'a, P: Position>(
	iter: impl IntoIterator<Item = impl IntoToken<'a, P>>,
	reporter: &mut impl Reporter<P>,
) -> Result<Map<'a, P>, ()> {
	#![allow(clippy::items_after_statements)]
	#![allow(clippy::too_many_lines)]

	let mut iter = iter.into_iter().map(IntoToken::into_token).peekable();

	let mut taml = Map::new();

	let mut path = vec![];

	let mut selection = &mut taml;

	#[derive(Debug, Copy, Clone, Eq, PartialEq)]
	enum ParserState {
		LineStart,
		Comment,
		Other,
	}
	impl ParserState {
		fn can_comment(&self) -> bool {
			match self {
				ParserState::Comment => false,
				ParserState::LineStart | ParserState::Other => true,
			}
		}

		fn can_heading(&self) -> bool {
			*self == ParserState::LineStart
		}

		fn can_data(&self) -> bool {
			*self == ParserState::LineStart
		}
	}

	let mut state = ParserState::LineStart;
	while let Some(next) = iter.peek().map(|t| &t.token) {
		state = match next {
			lexerToken::Error => {
				// Stop parsing but collect all the tokenizer reporter.
				reporter.report_many_with(|| {
					iter.filter_map(|t| {
						if let Token {
							token: lexerToken::Error,
							span,
						} = t
						{
							Some(span)
						} else {
							None
						}
					})
					.map(|span| Diagnostic {
						type_: DiagnosticType::UnrecognizedToken,
						labels: vec![DiagnosticLabel::new::<&'static str, _, _>(
							None,
							span,
							DiagnosticLabelPriority::Primary,
						)],
					})
				});
				return Err(());
			}

			lexerToken::Comment(_) if state.can_comment() => {
				let comment = iter.next();
				debug_assert!(matches!(
					comment.expect("unreachable").token,
					lexerToken::Comment(_)
				));
				ParserState::Comment
			}
			lexerToken::Comment(_) => {
				reporter.report_with(|| Diagnostic {
                    type_: DiagnosticType::MisplacedComment,
                    labels: vec![DiagnosticLabel::new(
                        "This comment appears after another comment without newline in-between, which shouldn't be possible.",
                        iter.next().expect("unreachable").span,
                        DiagnosticLabelPriority::Primary,
                    )]
                });
				ParserState::Comment
			}

			lexerToken::HeadingHashes(_) if state.can_heading() => {
				let (depth, hashes_span) = match iter.next().expect("unreachable") {
					Token {
						token: lexerToken::HeadingHashes(count),
						span,
					} => (count, span),
					_ => unreachable!(),
				};

				path.truncate(depth - 1);
				if path.len() != depth - 1 {
					reporter.report_with(|| Diagnostic {
                            type_: DiagnosticType::HeadingTooDeep,
                            labels: vec![
                                DiagnosticLabel::new(
                                    "This heading is nested more than one level deeper than the previous one.",
                                    hashes_span,
                                    DiagnosticLabelPriority::Primary,
                                )
                            ],
                        });
					return Err(());
				}

				if path
					.last()
					.and_then(|s: &PathSegment<P>| s.tabular.as_ref())
					.is_some()
				{
					reporter.report_with(|| Diagnostic {
                            type_: DiagnosticType::SubsectionInTabularSection,
                            labels: vec![
                                DiagnosticLabel::new(
                                    "This heading is nested inside a tabular section, which is not supported.",
                                    hashes_span,
                                    DiagnosticLabelPriority::Primary,
                                )
                            ],
                        });
					return Err(());
				}

				let new_segment = match parse_path_segment(&mut iter, reporter) {
					Ok(new_segment) => new_segment,
					Err(()) => return Err(()),
				};

				selection = match instantiate(
					get_last_mut(&mut taml, path.iter()),
					new_segment.base.iter().cloned(),
					reporter,
				) {
					Ok(selection) => selection,
					Err(()) => return Err(()),
				};

				if let Some(tabular) = new_segment.tabular.as_ref() {
					// Create lists for empty headings too.
					if let BasicPathElement {
						key: BasicPathElementKey::List { key, span },
						variant: _,
					} = tabular.base.first().expect("unreachable")
					{
						selection.entry(key.clone()).or_insert_with(|| Taml {
							span: span.clone(),
							value: TamlValue::List(List::new()),
						});
					} else {
						unreachable!()
					}
				}

				path.push(new_segment);

				ParserState::Other
			}
			lexerToken::HeadingHashes(_) => {
				let start = iter.next().expect("unreachable").span.start;
				reporter.report_with(|| Diagnostic {
					type_: DiagnosticType::MisplacedHeading,
					labels: vec![DiagnosticLabel::new(
						"Expected newline before heading.",
						start.clone()..start,
						DiagnosticLabelPriority::Primary,
					)],
				});
				ParserState::Comment
			}

			lexerToken::Newline => {
				let newline = iter.next();
				debug_assert_eq!(newline.expect("unreachable").token, lexerToken::Newline);
				ParserState::LineStart
			}

			// Data
			_ if state.can_data() => {
				#[allow(clippy::single_match_else)]
				match path.last().and_then(|s| s.tabular.as_ref()) {
					Some(tabular) => {
						let n = tabular.arity();
						let values = match parse_values_line(&mut iter, n, reporter) {
							Ok(values) => values,
							Err(()) => return Err(()),
						};

						let mut values = values.into_iter();
						if let Err(()) = tabular.assign(selection, &mut values, reporter) {
							return Err(());
						};

						debug_assert!(values.next().is_none());
					}
					None => {
						let (key, value) = match parse_key_value_pair(&mut iter, reporter) {
							Ok(kv) => kv,
							Err(()) => return Err(()),
						};
						//TODO: Also report occupied.
						if let map::Entry::Vacant(vacant) = selection.entry(key.clone()) {
							vacant.insert(value);
						} else {
							reporter.report_with(|| Diagnostic {
								type_: DiagnosticType::KeyPreviouslyDefined,
								labels: vec![DiagnosticLabel::new(
									"This key has already been assigned a value.",
									key.span,
									DiagnosticLabelPriority::Primary,
								)],
							});
							return Err(());
						}
					}
				};

				ParserState::Other
			}
			_ => {
				let start = iter.next().expect("unreachable").span.start;
				reporter.report_with(|| Diagnostic {
					type_: DiagnosticType::MisplacedData,
					labels: vec![DiagnosticLabel::new(
						if path.last().and_then(|s| s.tabular.as_ref()).is_some() {
							"Expected either a comma (to continue this row) or a newline (before the next table row) here."
						} else {
							"Expected a newline before next key-value-pair."
						},
						start.clone()..start,
						DiagnosticLabelPriority::Primary,
					)],
				});
				ParserState::Comment
			}
		}
	}

	Ok(taml)
}

fn parse_path_segment<'a, 'b, 'c, P: Position>(
	iter: &mut Peekable<impl Iterator<Item = Token<'a, P>>>,
	reporter: &mut impl Reporter<P>,
) -> Result<PathSegment<'a, P>, ()> {
	#![allow(clippy::too_many_lines)]

	let mut base = vec![];
	let mut tabular = None;

	if let Some(next) = iter.peek().map(|t| &t.token) {
		if matches!(next, lexerToken::Comment(_) | lexerToken::Newline) {
			return Ok(PathSegment { base, tabular });
		}
	}

	//TODO: Deduplicate the code here.
	loop {
		match iter.peek().map(|t| &t.token) {
			None => break,
			Some(lexerToken::Identifier(_)) => {
				let (key, key_span) = try_match!(
					Token {
						span: _1,
						token: lexerToken::Identifier(_0),
					} = iter.next().unwrap()
				)
				.debugless_unwrap();
				base.push(BasicPathElement {
                    key: BasicPathElementKey::Plain(Key{name:key, span:key_span}),
                    variant: if iter.peek().map(|t| &t.token) == Some(&lexerToken::Colon) {
                        assert_eq!(iter.next().unwrap().token, lexerToken::Colon);
                        if !matches!(
                            iter.peek().map(|t| &t.token),
                            Some(lexerToken::Identifier(_))
                        ) {
                            reporter.report_with(||Diagnostic {
                                type_: DiagnosticType::MissingVariantIdentifier,
                                labels: vec![DiagnosticLabel::new(
                                    "Colons in (non-tabular) paths must be followed by a variant identifier (for a structured enum).",
                                    iter.next().map(|t| t.span),
                                    DiagnosticLabelPriority::Primary,
                                )],
                            });
                            return Err(());
                        }
                        try_match!(
                            Token {
                                span: variant_span,
                                token:lexerToken::Identifier(variant),
                            } = iter.next().unwrap()
                            => Some(Key {
                                name: variant,
                                span:variant_span,
                            })
                        ).debugless_unwrap()
                    } else {
                        None
                    },
                })
			}
			Some(lexerToken::Brac) => {
				let Token {
					token,
					span: brac_span,
				} = iter.next().unwrap();
				assert_eq!(token, lexerToken::Brac);
				match iter.peek().map(|t| &t.token) {
					Some(lexerToken::Identifier(_)) => {
						let (key, key_span) = try_match!(
							Token {
								span: _1,
								token: lexerToken::Identifier(_0),
							} = iter.next().unwrap()
						)
						.debugless_unwrap();
						let ket_end = if let Some(lexerToken::Ket) = iter.peek().map(|t| &t.token) {
							try_match!(
								Token { token: lexerToken::Ket, span }
								= iter.next().unwrap()
								=> span.end
							)
							.debugless_unwrap()
						} else {
							reporter.report_with(|| Diagnostic {
								type_: DiagnosticType::UnclosedListKey,
								labels: vec![
									DiagnosticLabel::new(
										"The list key is opened here...",
										brac_span,
										DiagnosticLabelPriority::Auxiliary,
									),
									DiagnosticLabel::new(
										"...but not closed at this point.\nExpected ].",
										iter.next().map(|t| t.span.start.clone()..t.span.start),
										DiagnosticLabelPriority::Primary,
									),
								],
							});
							return Err(());
						};
						base.push(BasicPathElement {
							key: BasicPathElementKey::List {
								key: Key {
									name: key,
									span: key_span,
								},
								span: brac_span.start..ket_end,
							},
							variant: if iter.peek().map(|t| &t.token) == Some(&lexerToken::Colon) {
								assert_eq!(iter.next().unwrap().token, lexerToken::Colon);
								if !matches!(
									iter.peek().map(|t| &t.token),
									Some(lexerToken::Identifier(_))
								) {
									reporter.report_with(|| Diagnostic {
										type_: DiagnosticType::MissingVariantIdentifier,
										labels: vec![DiagnosticLabel::new(
											"Colons in headings must be followed by an identifier.",
											iter.next().map(|t| t.span),
											DiagnosticLabelPriority::Primary,
										)],
									});
									return Err(());
								}
								try_match!(
									Token{
										span: variant_span,
										token: lexerToken::Identifier(variant),
									} = iter.next().unwrap()
									=> Some(Key{
										name: variant,
										span:variant_span,
									})
								)
								.debugless_unwrap()
							} else {
								None
							},
						})
					}
					Some(lexerToken::Brac) => {
						tabular = Some(parse_tabular_path_segment(iter, reporter)?);
						if let Some(lexerToken::Ket) = iter.peek().map(|t| &t.token) {
							assert_eq!(iter.next().unwrap().token, lexerToken::Ket)
						} else {
							reporter.report_with(|| Diagnostic {
								type_: DiagnosticType::UnclosedTabularPathSection,
								labels: vec![
									DiagnosticLabel::new(
										"The tabular section is opened here...",
										brac_span,
										DiagnosticLabelPriority::Auxiliary,
									),
									DiagnosticLabel::new(
										"...but not closed at this point.\nExpected ].",
										iter.next().map(|t| t.span.start.clone()..t.span.start),
										DiagnosticLabelPriority::Primary,
									),
								],
							});
							return Err(());
						}
					}

					Some(lexerToken::InvalidIdentifierWithVerbatimCarriageReturn(str)) => {
						let str = *str;
						reporter.report_with(|| Diagnostic {
							type_: DiagnosticType::VerbatimCarriageReturnInsideLiteral,
							labels: cr_labels(str, iter.next().unwrap().span, Some('`')),
						});
						return Err(());
					}

					_ => {
						reporter.report_with(|| Diagnostic {
							type_: DiagnosticType::ExpectedPathSegment,
							labels: vec![DiagnosticLabel::new(
								"Expected [ or an identifier here.",
								iter.next().map(|t| t.span),
								DiagnosticLabelPriority::Primary,
							)],
						});
						return Err(());
					}
				}
			}
			Some(_) => {
				reporter.report_with(|| Diagnostic {
					type_: DiagnosticType::ExpectedPathSegment,
					labels: vec![DiagnosticLabel::new(
						"Expected [ or an identifier here.",
						iter.next().map(|t| t.span),
						DiagnosticLabelPriority::Primary,
					)],
				});
				return Err(());
			}
		}

		if tabular.is_some() {
			break;
		}
		match iter.peek().map(|t| &t.token) {
			Some(lexerToken::Newline | lexerToken::Comment(_)) => break,
			Some(lexerToken::Period) => assert_eq!(iter.next().unwrap().token, lexerToken::Period),
			_ => {
				reporter.report_with(|| Diagnostic {
					type_: DiagnosticType::InvalidPathContinuation,
					labels: vec![DiagnosticLabel::new(
						"Expected a period or end of line",
						iter.next().map(|t| t.span),
						DiagnosticLabelPriority::Primary,
					)],
				});
				return Err(());
			}
		}
	}

	Ok(PathSegment { base, tabular })
}

fn parse_tabular_path_segments<'a, P: Position>(
	iter: &mut Peekable<impl Iterator<Item = Token<'a, P>>>,
	reporter: &mut impl Reporter<P>,
) -> Result<Vec<TabularPathSegment<'a, P>>, ()> {
	let mut segments = vec![];
	while !matches!(
		iter.peek().map(|t| &t.token),
		None | Some(lexerToken::Ce | lexerToken::Ket)
	) {
		segments.push(parse_tabular_path_segment(iter, reporter)?);

		match iter.peek().map(|t| &t.token) {
			Some(lexerToken::Comma) => assert_eq!(iter.next().unwrap().token, lexerToken::Comma),
			_ => break,
		}
	}
	Ok(segments)
}

fn parse_tabular_path_segment<'a, P: Position>(
	iter: &mut Peekable<impl Iterator<Item = Token<'a, P>>>,
	reporter: &mut impl Reporter<P>,
) -> Result<TabularPathSegment<'a, P>, ()> {
	#![allow(clippy::too_many_lines)]

	let mut base = vec![];
	loop {
		match iter.peek().map(|t| &t.token) {
			Some(lexerToken::Bra) => {
				let Token {
					token: bra,
					span: bra_span,
				} = iter.next().unwrap();
				assert_eq!(bra, lexerToken::Bra);
				let multi = parse_tabular_path_segments(iter, reporter)?;
				return if let Some(lexerToken::Ce) = iter.peek().map(|t| &t.token) {
					let ce = iter.next().unwrap();
					assert_eq!(ce.token, lexerToken::Ce);
					Ok(TabularPathSegment {
						base,
						multi: Some((multi, bra_span.start..ce.span.end)),
					})
				} else {
					reporter.report_with(|| Diagnostic {
						type_: DiagnosticType::UnclosedTabularPathMultiSegment,
						labels: vec![
							DiagnosticLabel::new(
								"This multi segment starts here...",
								bra_span,
								DiagnosticLabelPriority::Auxiliary,
							),
							DiagnosticLabel::new(
								"...but is not closed at this point.",
								iter.next().map(|t| t.span.start.clone()..t.span.start),
								DiagnosticLabelPriority::Primary,
							),
						],
					});
					Err(())
				};
			}

			//TODO: Deduplicate the code
			Some(lexerToken::Identifier(_)) => {
				let (key_name, span) = try_match!(
					Token {
						span: _1,
						token: lexerToken::Identifier(_0),
					} = iter.next().unwrap()
				)
				.debugless_unwrap();

				base.push(BasicPathElement {
					key: BasicPathElementKey::Plain(Key {
						name: key_name,
						span,
					}),
					variant: if iter.peek().map(|t| &t.token) == Some(&lexerToken::Colon) {
						assert_eq!(iter.next().unwrap().token, lexerToken::Colon);
						if !matches!(
							iter.peek().map(|t| &t.token),
							Some(lexerToken::Identifier(_))
						) {
							reporter.report_with(|| Diagnostic {
								type_: DiagnosticType::MissingVariantIdentifier,
								labels: vec![DiagnosticLabel::new::<&'static str, _, _>(
									None,
									iter.next().map(|t| t.span.start.clone()..t.span.start),
									DiagnosticLabelPriority::Primary,
								)],
							});
							return Err(());
						}
						try_match!(
							Token {
								span,
								token: lexerToken::Identifier(variant_name),
							} = iter.next().unwrap()
							=> Some(Key {
								name: variant_name,
								span,
							})
						)
						.debugless_unwrap()
					} else {
						None
					},
				})
			}

			Some(lexerToken::Brac) => {
				let brac_start = try_match!(
					Token { token: lexerToken::Brac, span }
					= iter.next().unwrap()
					=> span.start
				)
				.debugless_unwrap();
				if let Some(lexerToken::Identifier(_)) = iter.peek().map(|t| &t.token) {
					let (str, str_span) = try_match!(
						Token {
							span: _1,
							token: lexerToken::Identifier(_0),
						} = iter.next().unwrap()
					)
					.debugless_unwrap();

					let ket_end = if let Some(lexerToken::Ket) = iter.peek().map(|t| &t.token) {
						let ket = iter.next().unwrap();
						assert_eq!(ket.token, lexerToken::Ket);
						ket.span.end
					} else {
						reporter.report_with(|| Diagnostic {
							type_: DiagnosticType::ExpectedListIdentifier,
							labels: vec![DiagnosticLabel::new::<&'static str, _, _>(
								None,
								iter.next().map(|t| t.span),
								DiagnosticLabelPriority::Primary,
							)],
						});
						return Err(());
					};
					base.push(BasicPathElement {
						key: BasicPathElementKey::List {
							key: Key {
								name: str,
								span: str_span,
							},
							span: brac_start..ket_end,
						},
						variant: if iter.peek().map(|t| &t.token) == Some(&lexerToken::Colon) {
							assert_eq!(iter.next().unwrap().token, lexerToken::Colon);
							if matches!(
								iter.peek().map(|t| &t.token),
								Some(lexerToken::Identifier(_))
							) {
								try_match!(
									Token{span, token:lexerToken::Identifier(str)}
									= iter.next().unwrap()
									=> Some(Key{name: str, span})
								)
								.debugless_unwrap()
							} else {
								reporter.report_with(|| Diagnostic {
									type_: DiagnosticType::MissingVariantIdentifier,
									labels: vec![DiagnosticLabel::new(
										"Colons in paths must be followed by a variant identifier.",
										iter.next().map(|t| t.span),
										DiagnosticLabelPriority::Primary,
									)],
								});
								return Err(());
							}
						} else {
							None
						},
					})
				} else {
					reporter.report_with(|| Diagnostic {
						type_: DiagnosticType::ExpectedListIdentifier,
						labels: vec![DiagnosticLabel::new::<&'static str, _, _>(
							None,
							iter.next().map(|t| t.span),
							DiagnosticLabelPriority::Primary,
						)],
					});
					return Err(());
				}
			}

			Some(lexerToken::InvalidIdentifierWithVerbatimCarriageReturn(str)) => {
				let str = *str;
				reporter.report_with(|| Diagnostic {
					type_: DiagnosticType::VerbatimCarriageReturnInsideLiteral,
					labels: cr_labels(str, iter.next().unwrap().span, Some('`')),
				});
				return Err(());
			}

			_ => {
				reporter.report_with(|| Diagnostic {
					type_: DiagnosticType::ExpectedTabularPathSegment,
					labels: vec![DiagnosticLabel::new(
						"Expected {, [ or an identifier here.",
						iter.next().map(|t| t.span),
						DiagnosticLabelPriority::Primary,
					)],
				});
				return Err(());
			}
		}

		match iter.peek().map(|t| &t.token) {
			Some(lexerToken::Period) => assert_eq!(iter.next().unwrap().token, lexerToken::Period),
			_ => return Ok(TabularPathSegment { base, multi: None }),
		}
	}
}

fn get_last_mut<'a, 'b, 'c, P: Position + 'c>(
	mut selected: &'a mut Map<'b, P>,
	path: impl IntoIterator<Item = &'c PathSegment<'b, P>>,
) -> &'a mut Map<'b, P>
where
	'b: 'c,
{
	for segment in path {
		let base = try_match!(
			PathSegment {
				base,
				tabular: None
			} = segment
			=> base
		)
		.debugless_unwrap();
		for path_element in base {
			let map = selected;
			let value = match &path_element.key {
				BasicPathElementKey::Plain(key) => &mut map.get_mut(key).unwrap().value,

				BasicPathElementKey::List { key, .. } => try_match!(
					TamlValue::List(selected)
					= &mut map.get_mut(key).unwrap().value
					=> &mut selected.last_mut().unwrap().value
				)
				.debugless_unwrap(),
			};

			selected = match (value, path_element.variant.as_ref()) {
				(TamlValue::Map(map), None) => map,
				(
					TamlValue::EnumVariant {
						key: existing_variant,
						payload: VariantPayload::Structured(fields),
					},
					Some(expected_variant),
				) if existing_variant == expected_variant => fields,
				_ => unreachable!(),
			};
		}
	}
	selected
}

fn instantiate<'a, 'b, P: Position>(
	mut selection: &'a mut Map<'b, P>,
	path: impl IntoIterator<Item = BasicPathElement<'b, P>>,
	reporter: &mut impl Reporter<P>,
) -> Result<&'a mut Map<'b, P>, ()> {
	for path_element in path {
		selection = match &path_element.key {
			BasicPathElementKey::Plain(key) => {
				let contains_key = selection.contains_key(key);
				match (contains_key, path_element.variant.clone()) {
					(true, None) => match selection.get_mut(key).unwrap() {
						Taml {
							span: _,
							value: TamlValue::Map(map),
						} => map,
						Taml { span, .. } => {
							reporter.report_with(|| Diagnostic {
								type_: DiagnosticType::NonMapValueSelected,
								labels: vec![
									DiagnosticLabel::new(
										"This key is assigned something other than a map here...",
										span.clone(),
										DiagnosticLabelPriority::Auxiliary,
									),
									DiagnosticLabel::new(
										"...but is selected as map here.",
										path_element.span(),
										DiagnosticLabelPriority::Primary,
									),
								],
							});
							return Err(());
						}
					},
					(true, Some(_)) => {
						reporter.report_with(|| Diagnostic {
							type_: DiagnosticType::DuplicateEnumInstantiation,
							labels: vec![
								DiagnosticLabel::new(
									"This enum value has already been assigned here...",
									selection.get(key).unwrap().span.clone(),
									DiagnosticLabelPriority::Auxiliary,
								),
								DiagnosticLabel::new(
									"...but another value is instantiated here.",
									path_element.span(),
									DiagnosticLabelPriority::Primary,
								),
							],
						});
						return Err(());
					}
					(false, None) => selection
						.entry(key.clone())
						.or_insert(Taml {
							span: path_element.span(),
							value: TamlValue::Map(Map::new()),
						})
						.unwrap_map_mut(),
					(false, Some(variant)) => selection
						.entry(key.clone())
						.or_insert(Taml {
							span: path_element.span(),
							value: TamlValue::EnumVariant {
								key: variant.clone(),
								payload: VariantPayload::Structured(Map::new()),
							},
						})
						.unwrap_variant_structured_mut(),
				}
			}
			BasicPathElementKey::List { key, span } => {
				let list = selection
					.entry(key.clone())
					.or_insert_with({
						let span = span.clone();
						|| Taml {
							span,
							value: TamlValue::List(vec![]),
						}
					})
					.unwrap_list_mut();

				#[allow(clippy::option_if_let_else)]
				if let Some(variant) = path_element.variant {
					list.push(Taml {
						span: variant.span.clone(),
						value: TamlValue::EnumVariant {
							key: variant,
							payload: VariantPayload::Structured(Map::new()),
						},
					});
					list.last_mut().unwrap().unwrap_variant_structured_mut()
				} else {
					list.push(Taml {
						span: span.clone(),
						value: TamlValue::Map(Map::new()),
					});
					list.last_mut().unwrap().unwrap_map_mut()
				}
			}
		};
	}
	Ok(selection)
}

fn parse_key_value_pair<'a, P: Position>(
	iter: &mut Peekable<impl Iterator<Item = Token<'a, P>>>,
	reporter: &mut impl Reporter<P>,
) -> Result<(Key<'a, P>, Taml<'a, P>), ()> {
	Ok(
		if let Some(lexerToken::Identifier(_)) = iter.peek().map(|t| &t.token) {
			let (key_name, key_span) = try_match!(
				Token {
					token: lexerToken::Identifier(_0),
					span: _1,
				} = iter.next().unwrap()
			)
			.debugless_unwrap();

			let key = Key {
				name: key_name,
				span: key_span,
			};
			if matches!(
				iter.peek(),
				Some(&Token {
					token: lexerToken::Colon,
					..
				})
			) {
				assert!(matches!(
					iter.next().unwrap(),
					Token {
						token: lexerToken::Colon,
						..
					}
				))
			} else {
				reporter.report_with(|| Diagnostic {
					type_: DiagnosticType::ExpectedKeyValuePair,
					labels: vec![DiagnosticLabel::new(
						"Expected colon.",
						iter.next().map(|t| t.span.start.clone()..t.span.start),
						DiagnosticLabelPriority::Primary,
					)],
				});
				return Err(());
			}
			(key, parse_value(iter, reporter)?)
		} else {
			reporter.report_with(||Diagnostic {
                type_: DiagnosticType::ExpectedKeyValuePair,
                labels: vec![DiagnosticLabel ::new(
                    "Structured sections can only contain subsections and key-value pairs.\nKey-value pairs must start with an identifier.",
                    iter.next().map(|t| t.span),
                    DiagnosticLabelPriority::Primary,
                )],
            });
			return Err(());
		},
	)
}

fn parse_values_line<'a, P: Position>(
	iter: &mut Peekable<impl Iterator<Item = Token<'a, P>>>,
	count: usize,
	reporter: &mut impl Reporter<P>,
) -> Result<Vec<Taml<'a, P>>, ()> {
	let mut values = vec![parse_value(iter, reporter)?];
	for _ in 1..count {
		if iter.peek().map(|t| &t.token) == Some(&lexerToken::Comma) {
			assert_eq!(iter.next().unwrap().token, lexerToken::Comma);
			values.push(parse_value(iter, reporter)?)
		} else {
			reporter.report_with(|| Diagnostic {
				type_: DiagnosticType::ValuesLineTooShort,
				labels: vec![DiagnosticLabel::new(
					"Expected comma here.",
					iter.next().map(|t| t.span.start.clone()..t.span.start),
					DiagnosticLabelPriority::Primary,
				)],
			});
			return Err(());
		}
	}
	if iter.peek().map(|t| &t.token) == Some(&lexerToken::Comma) {
		assert_eq!(iter.next().unwrap().token, lexerToken::Comma);
	}
	Ok(values)
}

fn parse_value<'a, P: Position>(
	iter: &mut Peekable<impl Iterator<Item = Token<'a, P>>>,
	reporter: &mut impl Reporter<P>,
) -> Result<Taml<'a, P>, ()> {
	#![allow(clippy::too_many_lines)]

	fn err<'a, Position>(
		span: impl Into<Option<Range<Position>>>,
		reporter: &mut impl Reporter<Position>,
	) -> Result<Taml<'a, Position>, ()> {
		reporter.report_with(|| Diagnostic {
			type_: DiagnosticType::ExpectedValue,
			labels: vec![DiagnosticLabel::new::<&'static str, _, _>(
				None,
				span,
				DiagnosticLabelPriority::Primary,
			)],
		});
		Err(())
	}

	if let Some(Token { token, span }) = iter.next() {
		Ok(match (token, span) {
			(lexerToken::Paren, paren_span) => {
				let mut items = vec![];
				while iter.peek().map(|t| &t.token) != Some(&lexerToken::Thesis) {
					if matches!(
						iter.peek().map(|t| &t.token),
						None | Some(&(lexerToken::Comment(_) | lexerToken::Newline))
					) {
						// Defer to unclosed list error.
						break;
					}

					items.push(parse_value(iter, reporter)?);
					match iter.peek().map(|t| &t.token) {
						Some(lexerToken::Comma) => {
							assert_eq!(iter.next().unwrap().token, lexerToken::Comma)
						}
						_ => break,
					}
				}
				if iter.peek().map(|t| &t.token) == Some(&lexerToken::Thesis) {
					let thesis = iter.next().unwrap();
					assert_eq!(thesis.token, lexerToken::Thesis);
					Taml {
						value: TamlValue::List(items),
						span: paren_span.start..thesis.span.end,
					}
				} else {
					reporter.report_with(|| Diagnostic {
						type_: DiagnosticType::UnclosedList,
						labels: vec![
							DiagnosticLabel::new(
								"This list starts here...",
								paren_span,
								DiagnosticLabelPriority::Auxiliary,
							),
							DiagnosticLabel::new(
								"...but is unclosed at this point.",
								iter.next().map(|t| t.span.start.clone()..t.span.start),
								DiagnosticLabelPriority::Primary,
							),
						],
					});
					return Err(());
				}
			}

			(lexerToken::String(str), span) => Taml {
				value: TamlValue::String(str),
				span,
			},
			(lexerToken::DataLiteral(data_literal), span) => Taml {
				value: TamlValue::DataLiteral(data_literal),
				span,
			},
			(lexerToken::Decimal(str), span) => Taml {
				value: TamlValue::Decimal(str),
				span,
			},
			(lexerToken::Integer(str), span) => Taml {
				value: TamlValue::Integer(str),
				span,
			},

			// Enum variant
			(lexerToken::Identifier(str), key_span) => {
				if iter.peek().map(|t| &t.token) == Some(&lexerToken::Paren) {
					try_match!(
						Taml {
							span: list_span,
							value: TamlValue::List(list),
						} = parse_value(iter, reporter)?
						=> Taml {
							span: key_span.start.clone()..list_span.end,
							value: TamlValue::EnumVariant {
								key: Key {
									name: str,
									span: key_span,
								},
								payload: VariantPayload::Tuple(list),
							},
						}
					)
					.debugless_unwrap()
				} else {
					Taml {
						span: key_span.clone(),
						value: TamlValue::EnumVariant {
							key: Key {
								name: str,
								span: key_span,
							},
							payload: VariantPayload::Unit,
						},
					}
				}
			}

			// Errors
			(lexerToken::InvalidZeroPrefixedDecimal(_), span) => {
				reporter.report_with(|| Diagnostic {
					type_: DiagnosticType::ZeroPrefixedDecimalFound,
					labels: vec![
						DiagnosticLabel::new::<&'static str, _, _>(
							None,
							span,
							DiagnosticLabelPriority::Primary,
						),
						DiagnosticLabel::new::<&'static str, _, _>(
							"TAML does not support optional zero prefixes on numbers, as they could be confused with octal literals.",
							None,
							DiagnosticLabelPriority::Auxiliary,
						),
					],
				});
				return Err(());
			}

			(lexerToken::InvalidZeroPrefixedInteger(_), span) => {
				reporter.report_with(|| Diagnostic {
					type_: DiagnosticType::ZeroPrefixedIntegerFound,
					labels: vec![
						DiagnosticLabel::new::<&'static str, _, _>(
							None,
							span,
							DiagnosticLabelPriority::Primary,
						),
						DiagnosticLabel::new::<&'static str, _, _>(
							"TAML does not support optional zero prefixes on numbers, as they could be confused with octal literals.",
							None,
							DiagnosticLabelPriority::Auxiliary,
						),
					],
				});
				return Err(());
			}

			(
				lexerToken::InvalidDataLiteralWithVerbatimCarriageReturn(invalid_data_literal),
				_span,
			) => {
				if invalid_data_literal.encoding.contains('\r') {
					reporter.report_with(|| Diagnostic {
						type_: DiagnosticType::VerbatimCarriageReturnInsideLiteral,
						labels: cr_labels(
							invalid_data_literal.encoding,
							invalid_data_literal.encoding_span.clone(),
							Some('`'),
						),
					});
				}
				if invalid_data_literal.unencoded_data.contains('\r') {
					reporter.report_with(|| Diagnostic {
						type_: DiagnosticType::VerbatimCarriageReturnInsideLiteral,
						labels: cr_labels(
							invalid_data_literal.unencoded_data,
							invalid_data_literal.unencoded_data_span,
							None,
						),
					});
				}
				return Err(());
			}

			(lexerToken::InvalidIdentifierWithVerbatimCarriageReturn(str), span) => {
				reporter.report_with(|| Diagnostic {
					type_: DiagnosticType::VerbatimCarriageReturnInsideLiteral,
					labels: cr_labels(str, span, Some('`')),
				});
				return Err(());
			}

			(lexerToken::InvalidStringWithVerbatimCarriageReturn(str), span) => {
				reporter.report_with(|| Diagnostic {
					type_: DiagnosticType::VerbatimCarriageReturnInsideLiteral,
					labels: cr_labels(str, span, Some('"')),
				});
				return Err(());
			}

			(_, span) => return err(span, reporter),
		})
	} else {
		err(None, reporter)
	}
}

fn cr_labels<P: Position>(
	str: &str,
	span: Range<P>,
	delimiter: Option<char>,
) -> Vec<DiagnosticLabel<P>> {
	let delimiter_len = delimiter.map_or(0, char::len_utf8);
	str.char_indices()
		.filter_map(|(i, c)| (c == '\r').then(|| i))
		.map(|i| DiagnosticLabel {
			caption: None,
			span: span
				.start
				.offset_range(delimiter_len + i..delimiter_len + i + '\r'.len_utf8()),
			priority: DiagnosticLabelPriority::Primary,
		})
		.chain(iter::once(DiagnosticLabel::new(
			"Hint: Either delete these code points or escape them as `\\r`.",
			None,
			DiagnosticLabelPriority::Auxiliary,
		)))
		// TODO
		// .chain(iter::once(DiagnosticLabel::new(
		// 	"Hint: `taml fix --erase-cr <file>` or `taml fix --escape-cr  <file>` will do this for you.",
		// 	None,
		// 	DiagnosticLabelPriority::Auxiliary,
		// )))
		.collect()
}
