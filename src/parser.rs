//TODO: This entire file needs to have its types simplified.

use {
    crate::{
        diagnostics::{
            Diagnostic, DiagnosticLabel, DiagnosticLabelPriority, DiagnosticType, Diagnostics,
        },
        token::Token as lexerToken,
    },
    smartstring::alias::String,
    std::{
        collections::{hash_map, HashMap},
        iter::{self, Peekable},
        ops::Range,
    },
    woc::Woc,
};

pub trait IntoToken<'a, Position> {
    fn into_token(self) -> Token<'a, Position>;
}

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

#[derive(Debug)]
pub enum Taml<'a> {
    String(Woc<'a, String, str>),
    Integer(&'a str),
    Float(&'a str),
    List(List<'a>),
    Map(Map<'a>),
    //TODO: Refactor these into a common variant.
    StructuredVariant { variant: Key<'a>, fields: Map<'a> },
    TupleVariant { variant: Key<'a>, values: List<'a> },
    UnitVariant { variant: Key<'a> },
}

struct PathSegment<'a> {
    base: Vec<BasicPathElement<'a>>,
    tabular: Option<TabularPathSegment<'a>>,
}

#[derive(Clone)]
struct BasicPathElement<'a> {
    key: BasicPathElementKey<'a>,
    variant: Option<Key<'a>>,
}

#[derive(Clone)]
enum BasicPathElementKey<'a> {
    Plain(Key<'a>),
    List(Key<'a>),
}

struct TabularPathSegment<'a> {
    base: Vec<BasicPathElement<'a>>,
    multi: Option<Vec<TabularPathSegment<'a>>>,
}

pub type Key<'a> = Woc<'a, String, str>;

pub type Map<'a> = HashMap<Key<'a>, Taml<'a>>;
pub type MapIter<'iter, 'taml> = hash_map::Iter<'iter, Key<'taml>, Taml<'taml>>;

pub type List<'a> = Vec<Taml<'a>>;
pub type ListIter<'iter, 'taml> = std::slice::Iter<'iter, Taml<'taml>>;

impl<'a> TabularPathSegment<'a> {
    fn arity(&self) -> usize {
        match &self.multi {
            None => 1,
            Some(multi) => multi.iter().map(Self::arity).sum(),
        }
    }

    fn assign(
        &self,
        selection: &mut Map<'a>,
        values: &mut impl Iterator<Item = Taml<'a>>,
    ) -> Result<(), ()> {
        if self.base.is_empty() && self.multi.is_none() {
            //TODO: Make sure these aren't accepted by the parser.
            unreachable!("Completely empty tabular path segments are invalid.")
        } else if self.base.is_empty() {
            for child in self.multi.as_ref().unwrap() {
                child.assign(selection, values)?
            }

            return Ok(());
        }

        let selection = instantiate(
            selection,
            self.base.iter().take(self.base.len() - 1).cloned(),
        )?;

        let selection = match &self.base.last().unwrap().key {
            BasicPathElementKey::Plain(key) => selection
                .entry(key.clone())
                .or_insert_with(|| Taml::String(Woc::Borrowed("PLACEHOLDER"))),
            BasicPathElementKey::List(key) => {
                let list = match selection
                    .entry(key.clone())
                    .or_insert_with(|| Taml::List(vec![]))
                {
                    Taml::List(list) => list,
                    _ => unreachable!(),
                };
                list.push(Taml::String(Woc::Borrowed("PLACEHOLDER")));
                list.last_mut().unwrap()
            }
        };

        let variant = self.base.last().unwrap().variant.as_ref();

        if let Some(multi) = &self.multi {
            let selection = if let Some(variant) = variant {
                *selection = Taml::StructuredVariant {
                    variant: variant.clone(),
                    fields: Map::new(),
                };
                match selection {
                    Taml::StructuredVariant { variant: _, fields } => fields,
                    _ => unreachable!(),
                }
            } else {
                *selection = Taml::Map(HashMap::new());
                match selection {
                    Taml::Map(map) => map,
                    _ => unreachable!(),
                }
            };
            for child in multi {
                child.assign(selection, values.by_ref())?
            }
            Ok(())
        } else {
            if let Some(variant) = variant {
                *selection = Taml::TupleVariant {
                    variant: variant.clone(),
                    values: match values.next().ok_or(())? {
                        Taml::List(list) => list,
                        _ => return Err(()),
                    },
                }
            } else {
                *selection = values.next().ok_or(())?;
            }
            Ok(())
        }
    }
}

pub fn parse<'a, Position: Clone>(
    iter: impl IntoIterator<Item = impl IntoToken<'a, Position>>,
    diagnostics: &mut impl Diagnostics<Position>,
) -> Result<Map<'a>, ()> {
    let mut iter = iter.into_iter().map(|t| t.into_token()).peekable();

    let mut taml = Map::new();

    let mut path = vec![];

    let mut selection = &mut taml;

    while let Some(next) = iter.peek().map(|t| &t.token) {
        match next {
            lexerToken::Error => {
                // Stop parsing but collect all the tokenizer diagnostics.
                diagnostics.extend_with(|| {
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
                            labels: vec![DiagnosticLabel::new(
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
                    diagnostics.push_with(|| Diagnostic {
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
                    .and_then(|s: &PathSegment| s.tabular.as_ref())
                    .is_some()
                {
                    diagnostics.push_with(|| Diagnostic {
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

                let new_segment = match parse_path_segment(&mut iter, diagnostics) {
                    Ok(new_segment) => new_segment,
                    Err(()) => return Err(()),
                };

                selection = match instantiate(
                    get_last_mut(&mut taml, path.iter()),
                    new_segment.base.iter().cloned(),
                ) {
                    Ok(selection) => selection,
                    Err(()) => return Err(()),
                };

                if let Some(tabular) = new_segment.tabular.as_ref() {
                    // Create lists for empty headings too.
                    if let BasicPathElement {
                        key: BasicPathElementKey::List(key),
                        variant: _,
                    } = tabular.base.first().unwrap()
                    {
                        selection
                            .entry(key.clone())
                            .or_insert_with(|| Taml::List(List::new()));
                    } else {
                        unreachable!()
                    }
                }

                path.push(new_segment);
            }

            lexerToken::Newline => assert_eq!(iter.next().unwrap().token, lexerToken::Newline),

            _ =>
            {
                #[allow(clippy::single_match_else)]
                match path.last().and_then(|s| s.tabular.as_ref()) {
                    Some(tabular) => {
                        let n = tabular.arity();
                        let mut values = match parse_values_line(&mut iter, n, diagnostics) {
                            Ok(values) => values,
                            Err(()) => return Err(()),
                        };

                        if let Err(()) = tabular.assign(selection, &mut values.drain(..)) {
                            return Err(());
                        };

                        assert!(values.is_empty());
                    }
                    None => {
                        let ((key, key_span), value) =
                            match parse_key_value_pair(&mut iter, diagnostics) {
                                Ok(kv) => kv,
                                Err(()) => return Err(()),
                            };
                        if let hash_map::Entry::Vacant(vacant) = selection.entry(key) {
                            vacant.insert(value);
                        } else {
                            diagnostics.push_with(|| Diagnostic {
                                r#type: DiagnosticType::KeyPreviouslyDefined,
                                labels: vec![DiagnosticLabel::new(
                                    "This key has already been assigned a value.",
                                    key_span,
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
    diagnostics: &mut impl Diagnostics<Position>,
) -> Result<PathSegment<'a>, ()> {
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
            Some(lexerToken::Identifier(_)) => match iter.next().unwrap().token {
                 lexerToken::Identifier(str)=> base.push(BasicPathElement {
                    key: BasicPathElementKey::Plain(str),
                    variant: if iter.peek().map(|t| &t.token) == Some(&lexerToken::Colon) {
                        assert_eq!(iter.next().unwrap().token, lexerToken::Colon);
                        if !matches!(
                            iter.peek().map(|t| &t.token),
                            Some(lexerToken::Identifier(_))
                        ) {
                            diagnostics.push_with(||Diagnostic {
                                r#type: DiagnosticType::MissingVariantIdentifier,
                                labels: vec![DiagnosticLabel::new(
                                    "Colons in (non-tabular) paths must be followed by a variant identifier (for a structured enum).",
                                    iter.next().map(|t| t.span),
                                    DiagnosticLabelPriority::Primary,
                                )],
                            });
                            return Err(());
                        }
                        match iter.next().unwrap().token {
                            lexerToken::Identifier(str) => Some(str),
                            _ => unreachable!(),
                        }
                    } else {
                        None
                    },
                }),
                _ => unreachable!(),
            },
            Some(lexerToken::Brac) => {
                let Token { token, span: brac_span} = iter.next().unwrap();
                assert_eq!(token, lexerToken::Brac);
                match iter.peek().map(|t| &t.token) {
                    Some(lexerToken::Identifier(_)) => match iter.next().unwrap().token {
                        lexerToken::Identifier(str) => {
                            match iter.peek().map(|t| &t.token) {
                                Some(lexerToken::Ket) => assert_eq!(iter.next().unwrap().token, lexerToken::Ket),
                                _ => {
                                    diagnostics.push_with(||Diagnostic{
                                        r#type: DiagnosticType::UnclosedListKey,
                                        labels: vec![
                                            DiagnosticLabel::new("The list key is opened here...", brac_span, DiagnosticLabelPriority::Auxiliary),
                                            DiagnosticLabel::new("...but not closed at this point.\nExpected ].", iter.next().map(|t| t.span.start.clone()..t.span.start), DiagnosticLabelPriority::Primary),
                                        ],
                                    });
                                    return Err(());
                                },
                            }
                            base.push(BasicPathElement {
                                key: BasicPathElementKey::List(str),
                                variant: if iter.peek().map(|t| &t.token) == Some(&lexerToken::Colon) {
                                    assert_eq!(iter.next().unwrap().token, lexerToken::Colon);
                                    if !matches!(iter.peek().map(|t| &t.token), Some(lexerToken::Identifier(_))) {
                                        diagnostics.push_with(||Diagnostic{
                                            r#type: DiagnosticType::MissingVariantIdentifier,
                                            labels: vec![DiagnosticLabel::new(
                                                "Colons in headings must be followed by an identifier.",
                                                iter.next().map(|t| t.span),
                                                DiagnosticLabelPriority::Primary,
                                            )]
                                        });
                                        return Err(());
                                    }
                                    match iter.next().unwrap().token {
                                        lexerToken::Identifier(str) => Some(str),
                                        _ => unreachable!(),
                                    }
                                } else {
                                    None
                                },
                            })
                        }
                        _ => unreachable!(),
                    },
                    Some(lexerToken::Brac) => {
                        tabular = Some(parse_tabular_path_segment(iter, diagnostics)?);
                        match iter.peek().map(|t| &t.token) {
                            Some(lexerToken::Ket) => assert_eq!(iter.next().unwrap().token, lexerToken::Ket),
                            _ =>{
                                diagnostics.push_with(||Diagnostic{
                                    r#type: DiagnosticType::UnclosedTabularPathSection,
                                    labels: vec![
                                        DiagnosticLabel::new("The tabular section is opened here...", brac_span, DiagnosticLabelPriority::Auxiliary),
                                        DiagnosticLabel::new("...but not closed at this point.\nExpected ].", iter.next().map(|t| t.span.start.clone()..t.span.start), DiagnosticLabelPriority::Primary),
                                    ],
                                });
                                return Err(());
                            },
                        }
                    }
                    _ => {
                        diagnostics.push_with(||Diagnostic {
                            r#type: DiagnosticType::ExpectedPathSegment,
                            labels: vec![DiagnosticLabel::new(
                                "Expected [ or an identifier here.",
                                iter.next().map(|t| t.span),
                                DiagnosticLabelPriority::Primary)]});
                        return Err(());
                    },
                }
            }
            Some(_) => {
                diagnostics.push_with(||Diagnostic {
                    r#type: DiagnosticType::ExpectedPathSegment,
                    labels: vec![DiagnosticLabel::new(
                        "Expected [ or an identifier here.",
                        iter.next().map(|t| t.span),
                        DiagnosticLabelPriority::Primary,
                    )]
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
                diagnostics.push_with(|| Diagnostic {
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
    diagnostics: &mut impl Diagnostics<Position>,
) -> Result<Vec<TabularPathSegment<'a>>, ()> {
    let mut segments = vec![];
    while !matches!(
        iter.peek().map(|t| &t.token),
        None | Some(lexerToken::Ce) | Some(lexerToken::Ket)
    ) {
        segments.push(parse_tabular_path_segment(iter, diagnostics)?);

        match iter.peek().map(|t| &t.token) {
            Some(lexerToken::Comma) => assert_eq!(iter.next().unwrap().token, lexerToken::Comma),
            _ => break,
        }
    }
    Ok(segments)
}

fn parse_tabular_path_segment<'a, Position: Clone>(
    iter: &mut Peekable<impl Iterator<Item = Token<'a, Position>>>,
    diagnostics: &mut impl Diagnostics<Position>,
) -> Result<TabularPathSegment<'a>, ()> {
    let mut base = vec![];
    let mut multi = None;
    loop {
        match iter.peek().map(|t| &t.token) {
            Some(lexerToken::Bra) => {
                let Token { token, span } = iter.next().unwrap();
                assert_eq!(token, lexerToken::Bra);
                multi = Some(parse_tabular_path_segments(iter, diagnostics)?);
                match iter.peek().map(|t| &t.token) {
                    Some(lexerToken::Ce) => assert_eq!(iter.next().unwrap().token, lexerToken::Ce),
                    _ => {
                        diagnostics.push_with(|| Diagnostic {
                            r#type: DiagnosticType::UnclosedTabularPathMultiSegment,
                            labels: vec![
                                DiagnosticLabel::new(
                                    "This multi segment starts here...",
                                    span,
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
            Some(lexerToken::Identifier(_)) => match iter.next().unwrap().token {
                lexerToken::Identifier(str) => base.push(BasicPathElement {
                    key: BasicPathElementKey::Plain(str),
                    variant: if iter.peek().map(|t| &t.token) == Some(&lexerToken::Colon) {
                        assert_eq!(iter.next().unwrap().token, lexerToken::Colon);
                        if !matches!(
                            iter.peek().map(|t| &t.token),
                            Some(lexerToken::Identifier(_))
                        ) {
                            diagnostics.push_with(|| Diagnostic {
                                r#type: DiagnosticType::MissingVariantIdentifier,
                                labels: vec![DiagnosticLabel::new(
                                    None,
                                    iter.next().map(|t| t.span.start.clone()..t.span.start),
                                    DiagnosticLabelPriority::Primary,
                                )],
                            });
                            return Err(());
                        }
                        match iter.next().unwrap().token {
                            lexerToken::Identifier(str) => Some(str),
                            _ => unreachable!(),
                        }
                    } else {
                        None
                    },
                }),
                _ => unreachable!(),
            },

            Some(lexerToken::Brac) => {
                assert_eq!(iter.next().unwrap().token, lexerToken::Brac);
                match iter.peek().map(|t| &t.token) {
                    Some(lexerToken::Identifier(_)) => match iter.next().unwrap().token {
                        lexerToken::Identifier(str) => {
                            match iter.peek().map(|t| &t.token) {
                                Some(lexerToken::Ket) => {
                                    assert_eq!(iter.next().unwrap().token, lexerToken::Ket)
                                }
                                _ => {
                                    diagnostics.push_with(|| Diagnostic {
                                        r#type: DiagnosticType::ExpectedListIdentifier,
                                        labels: vec![DiagnosticLabel::new(
                                            None,
                                            iter.next().map(|t| t.span),
                                            DiagnosticLabelPriority::Primary,
                                        )],
                                    });
                                    return Err(());
                                }
                            };
                            base.push(BasicPathElement {
                                key: BasicPathElementKey::List(str),
                                variant: if iter.peek().map(|t| &t.token)
                                    == Some(&lexerToken::Colon)
                                {
                                    assert_eq!(iter.next().unwrap().token, lexerToken::Colon);
                                    if matches!(
                                        iter.peek().map(|t| &t.token),
                                        Some(lexerToken::Identifier(_))
                                    ) {
                                        match iter.next().unwrap().token {
                                            lexerToken::Identifier(str) => Some(str),
                                            _ => unreachable!(),
                                        }
                                    }else{
                                        diagnostics.push_with(||Diagnostic {
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
                        _ => unreachable!(),
                    },
                    _ => {
                        diagnostics.push_with(|| Diagnostic {
                            r#type: DiagnosticType::ExpectedListIdentifier,
                            labels: vec![DiagnosticLabel::new(
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
                diagnostics.push_with(|| Diagnostic {
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
            _ => break,
        }
        if multi.is_some() {
            break;
        }
    }

    Ok(TabularPathSegment { base, multi })
}

fn get_last_mut<'a, 'b, 'c>(
    mut selected: &'a mut Map<'b>,
    path: impl IntoIterator<Item = &'c PathSegment<'b>>,
) -> &'a mut Map<'b>
where
    'b: 'c,
{
    for segment in path {
        match segment {
            PathSegment {
                tabular: Some(_), ..
            } => unreachable!(),
            PathSegment {
                base,
                tabular: None,
            } => {
                for path_element in base {
                    let map = selected;
                    let value = match &path_element.key {
                        BasicPathElementKey::Plain(key) => map.get_mut(key.as_ref()),

                        BasicPathElementKey::List(key) => match map.get_mut(key.as_ref()) {
                            Some(Taml::List(selected)) => selected.last_mut(),
                            _ => unreachable!(),
                        },
                    };

                    selected = match (value, path_element.variant.as_ref()) {
                        (Some(Taml::Map(map)), None) => map,
                        (
                            Some(Taml::StructuredVariant {
                                variant: existing_variant,
                                fields,
                            }),
                            Some(expected_variant),
                        ) if existing_variant.as_ref() == expected_variant.as_ref() => fields,
                        _ => unreachable!(),
                    };
                }
            }
        }
    }

    selected
}

fn instantiate<'a, 'b>(
    mut selection: &'a mut Map<'b>,
    path: impl IntoIterator<Item = BasicPathElement<'b>>,
) -> Result<&'a mut Map<'b>, ()> {
    for path_element in path {
        selection = match path_element.key {
            BasicPathElementKey::Plain(key) => {
                let entry = selection.entry(key.clone());
                let taml = match (entry, path_element.variant) {
                    (hash_map::Entry::Occupied(occupied), None) => occupied.into_mut(),
                    (hash_map::Entry::Occupied(_), Some(_)) => return Err(()),
                    (hash_map::Entry::Vacant(vacant), None) => vacant.insert(Taml::Map(Map::new())),
                    (hash_map::Entry::Vacant(vacant), Some(variant)) => {
                        vacant.insert(Taml::StructuredVariant {
                            variant,
                            fields: Map::new(),
                        })
                    }
                };
                match taml {
                    Taml::Map(map) => map,
                    _ => return Err(()),
                }
            }
            BasicPathElementKey::List(key) => {
                let list = selection
                    .entry(key.clone())
                    .or_insert_with(|| Taml::List(vec![]));
                match list {
                    Taml::List(list) => {
                        if let Some(variant) = path_element.variant {
                            list.push(Taml::StructuredVariant {
                                variant,
                                fields: Map::new(),
                            });
                            match list.last_mut().unwrap() {
                                Taml::StructuredVariant { fields, .. } => fields,
                                _ => unreachable!(),
                            }
                        } else {
                            list.push(Taml::Map(Map::new()));
                            match list.last_mut().unwrap() {
                                Taml::Map(map) => map,
                                _ => unreachable!(),
                            }
                        }
                    }
                    _ => unreachable!(),
                }
            }
        };
    }
    Ok(selection)
}

fn parse_key_value_pair<'a, Position: Clone>(
    iter: &mut Peekable<impl Iterator<Item = Token<'a, Position>>>,
    diagnostics: &mut impl Diagnostics<Position>,
) -> Result<((Key<'a>, Range<Position>), Taml<'a>), ()> {
    Ok(match iter.peek().map(|t| &t.token) {
        Some(lexerToken::Identifier(_)) => match iter.next().unwrap() {
            Token {
                token: lexerToken::Identifier(key),
                span: key_span,
            } => {
                if matches!(
                    iter.peek(),
                    Some(&Token {
                        token: lexerToken::Colon,
                        ..
                    })
                ) {
                    assert!(matches!(iter.next().unwrap(), Token{token:lexerToken::Colon,..}))
                } else {
                    diagnostics.push_with(|| Diagnostic {
                        r#type: DiagnosticType::ExpectedKeyValuePair,
                        labels: vec![DiagnosticLabel::new(
                            "Expected colon.",
                            iter.next().map(|t| t.span.start.clone()..t.span.start),
                            DiagnosticLabelPriority::Primary,
                        )],
                    });
                    return Err(());
                }
                ((key, key_span), parse_value(iter, diagnostics)?)
            }
            _ => unreachable!(),
        },

        _ => {
            diagnostics.push_with(||Diagnostic {
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
    diagnostics: &mut impl Diagnostics<Position>,
) -> Result<Vec<Taml<'a>>, ()> {
    let mut values = vec![];
    values.push(parse_value(iter, diagnostics)?);
    for _ in 1..count {
        if iter.peek().map(|t| &t.token) == Some(&lexerToken::Comma) {
            assert_eq!(iter.next().unwrap().token, lexerToken::Comma);
            values.push(parse_value(iter, diagnostics)?)
        } else {
            diagnostics.push_with(|| Diagnostic {
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

fn parse_value<'a, Position: Clone>(
    iter: &mut Peekable<impl Iterator<Item = Token<'a, Position>>>,
    diagnostics: &mut impl Diagnostics<Position>,
) -> Result<Taml<'a>, ()> {
    fn err<'a, Position>(
        span: impl Into<Option<Range<Position>>>,
        diagnostics: &mut impl Diagnostics<Position>,
    ) -> Result<Taml<'a>, ()> {
        diagnostics.push_with(|| Diagnostic {
            r#type: DiagnosticType::ExpectedValue,
            labels: vec![DiagnosticLabel::new(
                None,
                span,
                DiagnosticLabelPriority::Primary,
            )],
        });
        Err(())
    }

    if let Some(Token { token, span }) = iter.next() {
        Ok(match token {
            lexerToken::Paren => {
                let mut items = vec![];
                while iter.peek().map(|t| &t.token) != Some(&lexerToken::Thesis) {
                    if matches!(iter.peek().map(|t| &t.token), None| Some(&lexerToken::Comment(_))| Some(&lexerToken::Newline))
                    {
                        // Defer to unclosed list error.
                        break;
                    }

                    items.push(parse_value(iter, diagnostics)?);
                    match iter.peek().map(|t| &t.token) {
                        Some(lexerToken::Comma) => {
                            assert_eq!(iter.next().unwrap().token, lexerToken::Comma)
                        }
                        _ => break,
                    }
                }
                if iter.peek().map(|t| &t.token) == Some(&lexerToken::Thesis) {
                    assert_eq!(iter.next().unwrap().token, lexerToken::Thesis);
                    Taml::List(items)
                } else {
                    diagnostics.push_with(|| Diagnostic {
                        r#type: DiagnosticType::UnclosedList,
                        labels: vec![
                            DiagnosticLabel::new(
                                "This list starts here...",
                                span,
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

            lexerToken::String(str) => Taml::String(str),
            lexerToken::Float(str) => Taml::Float(str),
            lexerToken::Integer(str) => Taml::Integer(str),
            lexerToken::Identifier(str) => {
                if iter.peek().map(|t| &t.token) == Some(&lexerToken::Paren) {
                    match parse_value(iter, diagnostics)? {
                        Taml::List(list) => Taml::TupleVariant {
                            variant: str,
                            values: list,
                        },
                        _ => unreachable!(),
                    }
                } else {
                    Taml::UnitVariant { variant: str }
                }
            }

            _ => return err(span, diagnostics),
        })
    } else {
        err(None, diagnostics)
    }
}
