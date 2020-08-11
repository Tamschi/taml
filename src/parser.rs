use {
    crate::{
        diagnostics::{
            self, Diagnostic, DiagnosticLabel, DiagnosticLabelPriority, DiagnosticType, Reporter,
        },
        token::Token as lexerToken,
    },
    indexmap::{map, IndexMap},
    opaque_unwrap::OpaqueUnwrap as _,
    smartstring::alias::String,
    std::{
        hash::Hash,
        iter::{self, Peekable},
        ops::{Deref, Range},
    },
    try_match::try_match,
    woc::Woc,
};

pub trait IntoToken<'a, Position> {
    fn into_token(self) -> Token<'a, Position>;
}

#[derive(Debug)]
pub struct Token<'a, Position> {
    token: lexerToken<'a>,
    span: Range<Position>,
}

impl<'a> IntoToken<'a, ()> for lexerToken<'a> {
    fn into_token(self) -> Token<'a, ()> {
        Token {
            token: self,
            span: ()..(),
        }
    }
}

impl<'a, Position> IntoToken<'a, Position> for (lexerToken<'a>, Range<Position>) {
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
    String(Woc<'a, String, str>),
    Integer(&'a str),
    Float(&'a str),
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

struct PathSegment<'a, Position: Clone> {
    base: Vec<BasicPathElement<'a, Position>>,
    tabular: Option<TabularPathSegment<'a, Position>>,
}

#[derive(Clone)]
struct BasicPathElement<'a, Position: Clone> {
    key: BasicPathElementKey<'a, Position>,
    variant: Option<Key<'a, Position>>,
}

impl<'a, Position: Clone> BasicPathElement<'a, Position> {
    fn span(&self) -> Range<Position> {
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

struct TabularPathSegment<'a, Position: Clone> {
    base: Vec<BasicPathElement<'a, Position>>,
    multi: Option<(Vec<TabularPathSegment<'a, Position>>, Range<Position>)>,
}

#[derive(Clone, Debug)]
pub struct Key<'a, Position> {
    pub name: Woc<'a, String, str>,
    pub span: Range<Position>,
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

impl<'a, Position: Clone> TabularPathSegment<'a, Position> {
    fn arity(&self) -> usize {
        match &self.multi {
            None => 1,
            Some(multi) => multi.0.iter().map(Self::arity).sum(),
        }
    }

    fn assign(
        &self,
        selection: &mut Map<'a, Position>,
        values: &mut impl Iterator<Item = Taml<'a, Position>>,
        reporter: &mut impl Reporter<Position>,
    ) -> Result<(), ()> {
        if self.base.is_empty() && self.multi.is_none() {
            //TODO: Make sure these aren't accepted by the parser.
            unreachable!("Completely empty tabular path segments are invalid.")
        } else if self.base.is_empty() {
            for child in self.multi.as_ref().unwrap().0.iter() {
                child.assign(selection, values, reporter)?
            }

            return Ok(());
        }

        let selection = instantiate(
            selection,
            self.base.iter().take(self.base.len() - 1).cloned(),
            reporter,
        )?;

        fn placeholder<'a, Position: Clone>(position: Position) -> Taml<'a, Position> {
            Taml {
                span: position.clone()..position,
                value: TamlValue::String(Woc::Borrowed("PLACEHOLDER")),
            }
        }

        let selection: &mut Taml<'a, Position> = match &self.base.last().unwrap().key {
            BasicPathElementKey::Plain(key) => try_match!(
                map::Entry::Vacant(vacant)
                = selection.entry(key.clone())
                => vacant.insert(placeholder(key.span.start.clone()))
            )
            .opaque_unwrap(),

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
                *selection = Taml {
                    span: multi.1.clone(),
                    value: TamlValue::Map(IndexMap::new()),
                };
                selection.unwrap_map_mut()
            };
            for child in multi.0.iter() {
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

pub fn parse<'a, Position: Clone>(
    iter: impl IntoIterator<Item = impl IntoToken<'a, Position>>,
    reporter: &mut impl Reporter<Position>,
) -> Result<Map<'a, Position>, ()> {
    let mut iter = iter.into_iter().map(|t| t.into_token()).peekable();

    let mut taml = Map::new();

    let mut path = vec![];

    let mut selection = &mut taml;

    while let Some(next) = iter.peek().map(|t| &t.token) {
        match next {
            lexerToken::Error => {
                // Stop parsing but collect all the tokenizer reporter.
                reporter.report_many_with(|| {
                    iter::once(iter.next().unwrap().span)
                        .chain(iter.filter_map(|t| {
                            if let Token {
                                token: lexerToken::Error,
                                span,
                            } = t
                            {
                                Some(span)
                            } else {
                                None
                            }
                        }))
                        .map(|span| Diagnostic {
                            r#type: DiagnosticType::UnrecognizedToken,
                            labels: vec![DiagnosticLabel::new::<&'static str, _, _>(
                                None,
                                span,
                                DiagnosticLabelPriority::Primary,
                            )],
                        })
                });
                return Err(());
            }

            lexerToken::Comment(_) => {
                assert!(matches!(iter.next().unwrap().token, lexerToken::Comment(_)))
            }

            lexerToken::HeadingHashes(_) => {
                let (depth, hashes_span) = match iter.next().unwrap() {
                    Token {
                        token: lexerToken::HeadingHashes(count),
                        span,
                    } => (count, span),
                    _ => unreachable!(),
                };

                path.truncate(depth - 1);
                if path.len() != depth - 1 {
                    reporter.report_with(|| Diagnostic {
                            r#type: DiagnosticType::HeadingTooDeep,
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
                    .and_then(|s: &PathSegment<Position>| s.tabular.as_ref())
                    .is_some()
                {
                    reporter.report_with(|| Diagnostic {
                            r#type: DiagnosticType::SubsectionInTabularSection,
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
                    } = tabular.base.first().unwrap()
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
            }

            lexerToken::Newline => assert_eq!(iter.next().unwrap().token, lexerToken::Newline),

            _ => {
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

                        assert!(values.next().is_none());
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
                                r#type: DiagnosticType::KeyPreviouslyDefined,
                                labels: vec![DiagnosticLabel::new(
                                    "This key has already been assigned a value.",
                                    key.span,
                                    DiagnosticLabelPriority::Primary,
                                )],
                            });
                            return Err(());
                        }
                    }
                }
            }
        }
    }

    Ok(taml)
}

fn parse_path_segment<'a, 'b, 'c, Position: Clone>(
    iter: &mut Peekable<impl Iterator<Item = Token<'a, Position>>>,
    reporter: &mut impl Reporter<Position>,
) -> Result<PathSegment<'a, Position>, ()> {
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
                .opaque_unwrap();
                base.push(BasicPathElement {
                    key: BasicPathElementKey::Plain(Key{name:key, span:key_span}),
                    variant: if iter.peek().map(|t| &t.token) == Some(&lexerToken::Colon) {
                        assert_eq!(iter.next().unwrap().token, lexerToken::Colon);
                        if !matches!(
                            iter.peek().map(|t| &t.token),
                            Some(lexerToken::Identifier(_))
                        ) {
                            reporter.report_with(||Diagnostic {
                                r#type: DiagnosticType::MissingVariantIdentifier,
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
                        ).opaque_unwrap()
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
                        .opaque_unwrap();
                        let ket_end = match iter.peek().map(|t| &t.token) {
                            Some(lexerToken::Ket) => try_match!(
                                Token { token: lexerToken::Ket, span }
                                = iter.next().unwrap()
                                => span.end
                            )
                            .opaque_unwrap(),
                            _ => {
                                reporter.report_with(|| Diagnostic {
                                    r#type: DiagnosticType::UnclosedListKey,
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
                            }
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
                                        r#type: DiagnosticType::MissingVariantIdentifier,
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
                                .opaque_unwrap()
                            } else {
                                None
                            },
                        })
                    }
                    Some(lexerToken::Brac) => {
                        tabular = Some(parse_tabular_path_segment(iter, reporter)?);
                        match iter.peek().map(|t| &t.token) {
                            Some(lexerToken::Ket) => {
                                assert_eq!(iter.next().unwrap().token, lexerToken::Ket)
                            }
                            _ => {
                                reporter.report_with(|| Diagnostic {
                                    r#type: DiagnosticType::UnclosedTabularPathSection,
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
                    }
                    _ => {
                        reporter.report_with(|| Diagnostic {
                            r#type: DiagnosticType::ExpectedPathSegment,
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
                    r#type: DiagnosticType::ExpectedPathSegment,
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
            Some(lexerToken::Newline) | Some(lexerToken::Comment(_)) => break,
            Some(lexerToken::Period) => assert_eq!(iter.next().unwrap().token, lexerToken::Period),
            _ => {
                reporter.report_with(|| Diagnostic {
                    r#type: DiagnosticType::InvalidPathContinuation,
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

fn parse_tabular_path_segments<'a, Position: Clone>(
    iter: &mut Peekable<impl Iterator<Item = Token<'a, Position>>>,
    reporter: &mut impl Reporter<Position>,
) -> Result<Vec<TabularPathSegment<'a, Position>>, ()> {
    let mut segments = vec![];
    while !matches!(
        iter.peek().map(|t| &t.token),
        None | Some(lexerToken::Ce) | Some(lexerToken::Ket)
    ) {
        segments.push(parse_tabular_path_segment(iter, reporter)?);

        match iter.peek().map(|t| &t.token) {
            Some(lexerToken::Comma) => assert_eq!(iter.next().unwrap().token, lexerToken::Comma),
            _ => break,
        }
    }
    Ok(segments)
}

fn parse_tabular_path_segment<'a, Position: Clone>(
    iter: &mut Peekable<impl Iterator<Item = Token<'a, Position>>>,
    reporter: &mut impl Reporter<Position>,
) -> Result<TabularPathSegment<'a, Position>, ()> {
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
                match iter.peek().map(|t| &t.token) {
                    Some(lexerToken::Ce) => {
                        let ce = iter.next().unwrap();
                        assert_eq!(ce.token, lexerToken::Ce);
                        return Ok(TabularPathSegment {
                            base,
                            multi: Some((multi, bra_span.start..ce.span.end)),
                        });
                    }
                    _ => {
                        reporter.report_with(|| Diagnostic {
                            r#type: DiagnosticType::UnclosedTabularPathMultiSegment,
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
                        return Err(());
                    }
                }
            }

            //TODO: Deduplicate the code
            Some(lexerToken::Identifier(_)) => {
                let (key_name, span) = try_match!(
                    Token {
                        span: _1,
                        token: lexerToken::Identifier(_0),
                    } = iter.next().unwrap()
                )
                .opaque_unwrap();

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
                                r#type: DiagnosticType::MissingVariantIdentifier,
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
                        .opaque_unwrap()
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
                .opaque_unwrap();
                match iter.peek().map(|t| &t.token) {
                    Some(lexerToken::Identifier(_)) => {
                        let (str, str_span) = try_match!(
                            Token {
                                span: _1,
                                token: lexerToken::Identifier(_0),
                            } = iter.next().unwrap()
                        )
                        .opaque_unwrap();

                        let ket_end = match iter.peek().map(|t| &t.token) {
                            Some(lexerToken::Ket) => {
                                let ket = iter.next().unwrap();
                                assert_eq!(ket.token, lexerToken::Ket);
                                ket.span.end
                            }
                            _ => {
                                reporter.report_with(|| Diagnostic {
                                    r#type: DiagnosticType::ExpectedListIdentifier,
                                    labels: vec![DiagnosticLabel::new::<&'static str, _, _>(
                                        None,
                                        iter.next().map(|t| t.span),
                                        DiagnosticLabelPriority::Primary,
                                    )],
                                });
                                return Err(());
                            }
                        };
                        base.push(BasicPathElement {
                                key: BasicPathElementKey::List{key: Key{name: str, span: str_span} , span: brac_start..ket_end},
                                variant: if iter.peek().map(|t| &t.token)
                                    == Some(&lexerToken::Colon)
                                {
                                    assert_eq!(iter.next().unwrap().token, lexerToken::Colon);
                                    if matches!(
                                        iter.peek().map(|t| &t.token),
                                        Some(lexerToken::Identifier(_))
                                    ) {
                                        try_match!(
                                            Token{span, token:lexerToken::Identifier(str)}
                                            = iter.next().unwrap()
                                            => Some(Key{name: str, span})
                                        ).opaque_unwrap()
                                    } else {
                                        reporter.report_with(||Diagnostic {
                                            r#type: DiagnosticType::MissingVariantIdentifier,
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
                    }
                    _ => {
                        reporter.report_with(|| Diagnostic {
                            r#type: DiagnosticType::ExpectedListIdentifier,
                            labels: vec![DiagnosticLabel::new::<&'static str, _, _>(
                                None,
                                iter.next().map(|t| t.span),
                                DiagnosticLabelPriority::Primary,
                            )],
                        });
                        return Err(());
                    }
                }
            }
            _ => {
                reporter.report_with(|| Diagnostic {
                    r#type: DiagnosticType::ExpectedTabularPathSegment,
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

fn get_last_mut<'a, 'b, 'c, Position: Clone + 'c>(
    mut selected: &'a mut Map<'b, Position>,
    path: impl IntoIterator<Item = &'c PathSegment<'b, Position>>,
) -> &'a mut Map<'b, Position>
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
        .opaque_unwrap();
        for path_element in base {
            let map = selected;
            let value = match &path_element.key {
                BasicPathElementKey::Plain(key) => &mut map.get_mut(key).unwrap().value,

                BasicPathElementKey::List { key, .. } => try_match!(
                    TamlValue::List(selected)
                    = &mut map.get_mut(key).unwrap().value
                    => &mut selected.last_mut().unwrap().value
                )
                .opaque_unwrap(),
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

fn instantiate<'a, 'b, Position: Clone>(
    mut selection: &'a mut Map<'b, Position>,
    path: impl IntoIterator<Item = BasicPathElement<'b, Position>>,
    reporter: &mut impl Reporter<Position>,
) -> Result<&'a mut Map<'b, Position>, ()> {
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
                                r#type: DiagnosticType::NonMapValueSelected,
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
                            r#type: DiagnosticType::DuplicateEnumInstantiation,
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

fn parse_key_value_pair<'a, Position: Clone>(
    iter: &mut Peekable<impl Iterator<Item = Token<'a, Position>>>,
    reporter: &mut impl Reporter<Position>,
) -> Result<(Key<'a, Position>, Taml<'a, Position>), ()> {
    Ok(match iter.peek().map(|t| &t.token) {
        Some(lexerToken::Identifier(_)) => {
            let (key_name, key_span) = try_match!(
                Token {
                    token: lexerToken::Identifier(_0),
                    span: _1,
                } = iter.next().unwrap()
            )
            .opaque_unwrap();

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
                assert!(matches!(iter.next().unwrap(), Token{token:lexerToken::Colon,..}))
            } else {
                reporter.report_with(|| Diagnostic {
                    r#type: DiagnosticType::ExpectedKeyValuePair,
                    labels: vec![DiagnosticLabel::new(
                        "Expected colon.",
                        iter.next().map(|t| t.span.start.clone()..t.span.start),
                        DiagnosticLabelPriority::Primary,
                    )],
                });
                return Err(());
            }
            (key, parse_value(iter, reporter)?)
        }

        _ => {
            reporter.report_with(||Diagnostic {
                r#type: DiagnosticType::ExpectedKeyValuePair,
                labels: vec![DiagnosticLabel ::new(
                    "Structured sections can only contain subsections and key-value pairs.\nKey-value pairs must start with an identifier.",
                    iter.next().map(|t| t.span),
                    DiagnosticLabelPriority::Primary,
                )],
            });
            return Err(());
        }
    })
}

fn parse_values_line<'a, Position: Clone>(
    iter: &mut Peekable<impl Iterator<Item = Token<'a, Position>>>,
    count: usize,
    reporter: &mut impl Reporter<Position>,
) -> Result<Vec<Taml<'a, Position>>, ()> {
    let mut values = vec![];
    values.push(parse_value(iter, reporter)?);
    for _ in 1..count {
        if iter.peek().map(|t| &t.token) == Some(&lexerToken::Comma) {
            assert_eq!(iter.next().unwrap().token, lexerToken::Comma);
            values.push(parse_value(iter, reporter)?)
        } else {
            reporter.report_with(|| Diagnostic {
                r#type: DiagnosticType::ValuesLineTooShort,
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

fn string<
    'a: 'b,
    'b,
    Iter: 'b + Iterator<Item = Token<'a, Position>>,
    Reporter: 'b,
    Position: 'b,
>() -> impl 'b + FnOnce(&mut Peekable<Iter>, &mut Reporter) -> Option<Result<Taml<'a, Position>, ()>>
{
    combi::match_token!(|_| Token {
        token: lexerToken::String(str),
        span,
    } => Ok(Taml {
        value: TamlValue::String(str),
        span,
    }))
}

fn float<
    'a: 'b,
    'b,
    Iter: 'b + Iterator<Item = Token<'a, Position>>,
    Reporter: 'b,
    Position: 'b,
>() -> impl 'b + FnOnce(&mut Peekable<Iter>, &mut Reporter) -> Option<Result<Taml<'a, Position>, ()>>
{
    combi::match_token!(|_| Token {
        token: lexerToken::Float(str),
        span,
    } => Ok(Taml {
        value: TamlValue::Float(str),
        span,
    }))
}

fn integer<
    'a: 'b,
    'b,
    Iter: 'b + Iterator<Item = Token<'a, Position>>,
    Reporter: 'b,
    Position: 'b,
>() -> impl 'b + FnOnce(&mut Peekable<Iter>, &mut Reporter) -> Option<Result<Taml<'a, Position>, ()>>
{
    combi::match_token!(|_| Token {
        token: lexerToken::Integer(str),
        span,
    } => Ok(Taml {
        value: TamlValue::Integer(str),
        span,
    }))
}

fn key<'a: 'b, 'b, Iter: 'b + Iterator<Item = Token<'a, Position>>, Reporter: 'b, Position: 'b>(
) -> impl 'b + FnOnce(&mut Peekable<Iter>, &mut Reporter) -> Option<Result<Key<'a, Position>, ()>> {
    combi::match_token!(|_| Token {
        token: lexerToken::Identifier(name),
        span,
    } => Ok(Key { name, span }))
}

fn paren<'a, Iter: 'a + Iterator<Item = Token<'a, Position>>, Reporter: 'a, Position: 'a>(
) -> impl 'a + FnOnce(&mut Peekable<Iter>, &mut Reporter) -> Option<Result<Range<Position>, ()>> {
    combi::match_token!(|_| Token { token: lexerToken::Paren, span } => Ok(span))
}

fn thesis<'a, Iter: 'a + Iterator<Item = Token<'a, Position>>, Reporter: 'a, Position: 'a>(
) -> impl 'a + FnOnce(&mut Peekable<Iter>, &mut Reporter) -> Option<Result<Range<Position>, ()>> {
    combi::match_token!(|_| Token { token: lexerToken::Thesis, span } => Ok(span))
}

fn comma<'a, Iter: 'a + Iterator<Item = Token<'a, Position>>, Reporter: 'a, Position: 'a>(
) -> impl 'a + FnOnce(&mut Peekable<Iter>, &mut Reporter) -> Option<Result<Range<Position>, ()>> {
    combi::match_token!(|_| Token { token: lexerToken::Comma, span } => Ok(span))
}

fn raw_list<
    'a: 'b,
    'b,
    Iter: 'a + Iterator<Item = Token<'a, Position>>,
    Reporter: 'a + diagnostics::Reporter<Position>,
    Position: 'a + Clone,
>() -> impl 'b
       + FnOnce(
    &mut Peekable<Iter>,
    &mut Reporter,
) -> Option<Result<(Vec<Taml<'a, Position>>, Range<Position>), ()>> {
    combi::sequence_open((
        paren(),
        |paren_span| {
            combi::map_closed(
                combi::first_match_closed((
                    combi::match_peek!(Iter => |_: &mut Reporter| Some(Token { token: lexerToken::Thesis, span: _ }) => Some(Ok(vec![]))),
                    combi::todo_closed(),
                )),
                |value: Vec<Taml<'a, Position>>, _| Ok((paren_span, value)),
            )
        },
        |(paren_span, value): (Range<Position>, _)| {
            combi::required(
                combi::map_open(thesis(), {
                    let start = paren_span.start.clone();
                    |thesis_span, _| Ok((value, start..thesis_span.end))
                }),
                |next: Option<&Token<'a, Position>>, reporter: &mut Reporter| {
                    reporter.report_with(|| Diagnostic {
                        r#type: DiagnosticType::UnclosedList,
                        labels: vec![
                            DiagnosticLabel::new(
                                "This list starts here...",
                                paren_span,
                                DiagnosticLabelPriority::Auxiliary,
                            ),
                            DiagnosticLabel::new(
                                "...but is unclosed at this point.",
                                next.map(|t| t.span.start.clone()..t.span.start.clone()),
                                DiagnosticLabelPriority::Primary,
                            ),
                        ],
                    });
                    Err(())
                },
            )
        },
    ))
}

fn variant<
    'a: 'b,
    'b,
    Iter: 'a + Iterator<Item = Token<'a, Position>>,
    Reporter: 'a + diagnostics::Reporter<Position>,
    Position: 'a + Clone,
>() -> impl 'b + FnOnce(&mut Peekable<Iter>, &mut Reporter) -> Option<Result<Taml<'a, Position>, ()>>
{
    combi::sequence_open((key(), |key: Key<'a, Position>| {
        combi::first_match_closed((
            combi::map_open(raw_list(), {
                let key = key.clone();
                |(list, list_span), _| {
                    Ok(Taml {
                        span: key.span.start.clone()..list_span.end,
                        value: TamlValue::EnumVariant {
                            key,
                            payload: VariantPayload::Tuple(list),
                        },
                    })
                }
            }),
            combi::accept(|_, _| {
                Ok(Taml {
                    span: key.span.clone(),
                    value: TamlValue::EnumVariant {
                        key,
                        payload: VariantPayload::Unit,
                    },
                })
            }),
        ))
    }))
}

fn list<
    'a: 'b,
    'b,
    Iter: 'a + Iterator<Item = Token<'a, Position>>,
    Reporter: 'a + diagnostics::Reporter<Position>,
    Position: 'a + Clone,
>() -> impl 'b + FnOnce(&mut Peekable<Iter>, &mut Reporter) -> Option<Result<Taml<'a, Position>, ()>>
{
    combi::map_open(raw_list(), |(vec, span), _| {
        Ok(Taml {
            span,
            value: TamlValue::List(vec),
        })
    })
}

fn value<
    'a: 'b,
    'b,
    Iter: 'a + Iterator<Item = Token<'a, Position>>,
    Reporter: 'a + diagnostics::Reporter<Position>,
    Position: 'a + Clone,
>() -> impl 'b + FnOnce(&mut Peekable<Iter>, &mut Reporter) -> Result<Taml<'a, Position>, ()> {
    combi::required(
        combi::first_match_open((string(), float(), integer(), variant(), list())),
        |next: Option<&Token<'a, Position>>, reporter: &mut Reporter| {
            reporter.report_with(|| Diagnostic {
                r#type: DiagnosticType::ExpectedValue,
                labels: vec![DiagnosticLabel::new::<&'static str, _, _>(
                    None,
                    next.map(|t| t.span.clone()),
                    DiagnosticLabelPriority::Primary,
                )],
            });
            Err(())
        },
    )
}

fn newline<'a, Iter: 'a + Iterator<Item = Token<'a, Position>>, Reporter: 'a, Position: 'a>(
) -> impl 'a + FnOnce(&mut Peekable<Iter>, &mut Reporter) -> Option<Result<Range<Position>, ()>> {
    combi::match_token!(|_| Token { token: lexerToken::Newline, span } => Ok(span))
}

fn comment<'a, Iter: 'a + Iterator<Item = Token<'a, Position>>, Reporter: 'a, Position: 'a>(
) -> impl 'a + FnOnce(&mut Peekable<Iter>, &mut Reporter) -> Option<Result<Range<Position>, ()>> {
    combi::match_token!(|_| Token { token: lexerToken::Comment(_), span } => Ok(span))
}

fn line_separation<
    'a,
    Iter: 'a + Iterator<Item = Token<'a, Position>>,
    Reporter: 'a,
    Position: 'a,
>() -> impl 'a + FnOnce(&mut Peekable<Iter>, &mut Reporter) -> Option<Result<Range<Position>, ()>> {
    let separator = || combi::first_match_open((comment(), newline()));

    combi::sequence_open((separator(), move |first_range: Range<Position>| {
        combi::map_closed(combi::repeat(move |_| separator()), |further_ranges, _| {
            Ok(first_range.start
                ..further_ranges
                    .into_iter()
                    .last()
                    .map(|last| last.end)
                    .unwrap_or(first_range.end))
        })
    }))
}

fn parse_value<'a, Position: Clone>(
    iter: &mut Peekable<impl Iterator<Item = Token<'a, Position>>>,
    reporter: &mut impl Reporter<Position>,
) -> Result<Taml<'a, Position>, ()> {
    fn err<'a, Position>(
        span: impl Into<Option<Range<Position>>>,
        reporter: &mut impl Reporter<Position>,
    ) -> Result<Taml<'a, Position>, ()> {
        reporter.report_with(|| Diagnostic {
            r#type: DiagnosticType::ExpectedValue,
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
                    if matches!(iter.peek().map(|t| &t.token), None| Some(&lexerToken::Comment(_))| Some(&lexerToken::Newline))
                    {
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
                        r#type: DiagnosticType::UnclosedList,
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
            (lexerToken::Float(str), span) => Taml {
                value: TamlValue::Float(str),
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
                    .opaque_unwrap()
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

            (_, span) => return err(span, reporter),
        })
    } else {
        err(None, reporter)
    }
}
