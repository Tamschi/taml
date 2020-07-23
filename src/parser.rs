//TODO: This entire file needs to have its types simplified.

use {
    crate::token::Token as lexerToken,
    enum_properties::enum_properties,
    smallvec::{smallvec, SmallVec},
    smartstring::alias::String,
    std::{
        collections::{hash_map, HashMap},
        fmt::Display,
        iter::{FromIterator, Peekable},
        ops::Range,
    },
    woc::Woc,
};

trait IntoToken<'a, Position> {
    fn into_token(self) -> Token<'a, Position>;
}

struct Token<'a, Position> {
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

#[derive(Clone, Copy, Debug)]
enum DiagnosticLevel {
    Warning,
    Error,
}

pub struct DiagnosticTypeProperties {
    level: DiagnosticLevel,
    code: usize,
    title: &'static str,
}

enum_properties! {
    #[derive(Clone, Copy, Debug)]
    #[non_exhaustive]
    pub enum DiagnosticType: DiagnosticTypeProperties {
        HeadingTooDeep {
            code: 1,
            level: DiagnosticLevel::Error,
            title: "Heading too deep",
        },

        SubsectionInTabularSection {
            code: 2,
            level: DiagnosticLevel::Error,
            title: "Subsection in tabular section",
        },

        MissingVariantIdentifier {
            code: 3,
            level: DiagnosticLevel::Error,
            title: "Missing variant identifier",
        },

        ExpectedKeyValuePair {
            code: 4,
            level: DiagnosticLevel::Error,
            title: "Expected key-value pair",
        },

        ExpectedValue {
            code: 5,
            level: DiagnosticLevel::Error,
            title: "Expected value",
        },

        UnclosedList {
            code: 6,
            level: DiagnosticLevel::Error,
            title: "Unclosed list",
        },

        ValuesLineTooShort {
            code: 7,
            level: DiagnosticLevel::Error,
            title: "Values line too short",
        }
    }
}

impl Display for DiagnosticType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TAML{:04} {}", self.code, self.title)
    }
}

#[derive(Debug, Clone, Copy)]
enum DiagnosticLabelPriority {
    Primary,
    Auxiliary,
}

#[derive(Debug, Clone)]
pub struct DiagnosticLabel<Position> {
    caption: Option<&'static str>,
    span: Option<Range<Position>>,
    priority: DiagnosticLabelPriority,
}

impl<Position> DiagnosticLabel<Position> {
    pub fn new(
        caption: impl Into<Option<&'static str>>,
        span: impl Into<Option<Range<Position>>>,
        priority: DiagnosticLabelPriority,
    ) -> Self {
        Self {
            caption: caption.into(),
            span: span.into(),
            priority,
        }
    }
}

#[derive(Debug, Clone)]
struct Diagnostic<Position> {
    r#type: DiagnosticType,
    labels: Vec<DiagnosticLabel<Position>>,
}

pub type Diagnostics<Position> = SmallVec<[Diagnostic<Position>; 0]>;

//TODO: Implement specific errors and Display.
#[derive(Debug)]
#[non_exhaustive]
pub enum Expected {
    ValueAlreadyExistsInStructuredEnumTargetSlot,
    StructuredEnumVariantIdentifier,
    Unspecific,
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

//TODO: Public API.

impl<'a, Position> FromIterator<Token<'a, Position>>
    for (Result<Map<'a>, ()>, Diagnostics<Position>)
{
    fn from_iter<I: IntoIterator<Item = Token<'a, Position>>>(iter: I) -> Self {
        let mut iter = iter.into_iter().peekable();

        let mut taml = Map::new();
        let mut diagnostics = smallvec![];

        let mut path = vec![];

        let mut selection = &mut taml;

        while let Some(next) = iter.peek().map(|t| t.token) {
            match next {
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
                        diagnostics.push(Diagnostic {
                            r#type: DiagnosticType::HeadingTooDeep,
                            labels: vec![
                                DiagnosticLabel::new(
                                    "This heading is nested more than one level deeper than the previous one.",
                                    hashes_span,
                                    DiagnosticLabelPriority::Primary,
                                )
                            ],
                        });
                        return (Err(()), diagnostics);
                    }

                    if path
                        .last()
                        .and_then(|s: &PathSegment| s.tabular.as_ref())
                        .is_some()
                    {
                        diagnostics.push(Diagnostic {
                            r#type: DiagnosticType::SubsectionInTabularSection,
                            labels: vec![
                                DiagnosticLabel::new(
                                    "This heading is nested inside a tabular section, which is not supported.",
                                    hashes_span,
                                    DiagnosticLabelPriority::Primary,
                                )
                            ],
                        });
                        return (Err(()), diagnostics);
                    }

                    let new_segment = match parse_path_segment(&mut iter, &mut diagnostics) {
                        Ok(new_segment) => new_segment,
                        Err(()) => return (Err(()), diagnostics),
                    };

                    selection = match get_last_mut(&mut taml, path.iter())
                        .and_then(|selection| selection.ok_or(()))
                        .and_then(|selection| {
                            instantiate(selection, new_segment.base.iter().cloned())
                        }) {
                        Ok(selection) => selection,
                        Err(()) => return (Err(()), diagnostics),
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
                Token::Newline => assert_eq!(iter.next().unwrap(), Token::Newline),
                _ =>
                {
                    #[allow(clippy::single_match_else)]
                    match path.last().and_then(|s| s.tabular.as_ref()) {
                        Some(tabular) => {
                            let n = tabular.arity();
                            let mut values = parse_values_line(&mut iter, n)?;

                            tabular
                                .assign(selection, &mut values.drain(..))
                                .map_err(|()| Expected::Unspecific)?;

                            if !values.is_empty() {
                                return Err(Expected::Unspecific);
                            }
                        }
                        None => {
                            let kv =
                                parse_key_value_pair(&mut iter)?.ok_or(Expected::Unspecific)?;
                            if let hash_map::Entry::Vacant(vacant) = selection.entry(kv.0) {
                                vacant.insert(kv.1);
                            } else {
                                return Err(Expected::Unspecific);
                            }
                        }
                    }
                }
            }
        }

        Ok(taml)
    }
}

fn parse_path_segment<'a, 'b, 'c, Position>(
    iter: &mut Peekable<impl Iterator<Item = Token<'a, Position>>>,
    diagnostics: &mut Diagnostics<Position>,
) -> Result<PathSegment<'a>, ()> {
    let mut base = vec![];
    let mut tabular = None;

    if let Some(next) = iter.peek().map(|t| t.token) {
        if matches!(next, lexerToken::Comment(_) | lexerToken::Newline) {
            return Ok(PathSegment { base, tabular });
        }
    }

    //TODO: Deduplicate the code here.
    loop {
        match iter.peek() {
            None => break,
            Some(Token{token:lexerToken::Identifier(_),..}) => match iter.next().unwrap() {
                Token {
                    token: lexerToken::Identifier(str),
                    ..
                } => base.push(BasicPathElement {
                    key: BasicPathElementKey::Plain(str),
                    variant: if iter.peek().map(|t| &t.token) == Some(&lexerToken::Colon) {
                        assert_eq!(iter.next().unwrap().token, lexerToken::Colon);
                        if !matches!(
                            iter.peek().map(|t| t.token),
                            Some(lexerToken::Identifier(_))
                        ) {
                            diagnostics.push(Diagnostic {
                                r#type: DiagnosticType::MissingVariantIdentifier,
                                labels: vec![DiagnosticLabel::new(
                                    "Colons in (non-tabular) paths must be followed by a variant identifier (for a structured enum).",
                                    iter.peek().map(|t| t.span),
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
            Some(Token::Brac) => {
                assert_eq!(iter.next().unwrap(), Token::Brac);
                match iter.peek().ok_or(Expected::Unspecific)? {
                    Token::Identifier(_) => match iter.next().unwrap() {
                        Token::Identifier(str) => {
                            match iter.peek() {
                                Some(Token::Ket) => assert_eq!(iter.next().unwrap(), Token::Ket),
                                _ => return Err(Expected::Unspecific),
                            }
                            base.push(BasicPathElement {
                                key: BasicPathElementKey::List(str),
                                variant: if iter.peek() == Some(&Token::Colon) {
                                    assert_eq!(iter.next().unwrap(), Token::Colon);
                                    if !matches!(iter.peek(), Some(Token::Identifier(_))) {
                                        return Err(Expected::StructuredEnumVariantIdentifier);
                                    }
                                    match iter.next().unwrap() {
                                        Token::Identifier(str) => Some(str),
                                        _ => unreachable!(),
                                    }
                                } else {
                                    None
                                },
                            })
                        }
                        _ => unreachable!(),
                    },
                    Token::Brac => {
                        tabular = Some(parse_tabular_path_segment(iter)?);
                        match iter.peek() {
                            Some(Token::Ket) => assert_eq!(iter.next().unwrap(), Token::Ket),
                            _ => return Err(Expected::Unspecific),
                        }
                    }
                    _ => return Err(Expected::Unspecific),
                }
            }
            Some(_) => return Err(Expected::Unspecific),
        }

        if tabular.is_some() {
            break;
        }
        match iter.peek() {
            Some(Token::Newline) => break,
            Some(Token::Period) => assert_eq!(iter.next().unwrap(), Token::Period),
            _ => return Err(Expected::Unspecific),
        }
    }

    Ok(PathSegment { base, tabular })
}

fn parse_tabular_path_segments<'a, Position>(
    iter: &mut Peekable<impl Iterator<Item = Token<'a, Position>>>,
) -> Result<Vec<TabularPathSegment<'a>>, Expected> {
    let mut segments = vec![];
    while !matches!(
        iter.peek().ok_or(Expected::Unspecific)?,
        Token::Ce | Token::Ket
    ) {
        segments.push(parse_tabular_path_segment(iter)?);

        match iter.peek() {
            Some(Token::Comma) => assert_eq!(iter.next().unwrap(), Token::Comma),
            _ => break,
        }
    }
    Ok(segments)
}

fn parse_tabular_path_segment<'a, Position>(
    iter: &mut Peekable<impl Iterator<Item = Token<'a, Position>>>,
) -> Result<TabularPathSegment<'a>, Expected> {
    let mut base = vec![];
    let mut multi = None;
    loop {
        match iter.peek() {
            Some(Token::Bra) => {
                assert_eq!(iter.next().unwrap(), Token::Bra);
                multi = Some(parse_tabular_path_segments(iter)?);
                match iter.peek() {
                    Some(Token::Ce) => assert_eq!(iter.next().unwrap(), Token::Ce),
                    _ => return Err(Expected::Unspecific),
                }
            }

            //TODO: Deduplicate the code
            Some(Token::Identifier(_)) => match iter.next().unwrap() {
                Token::Identifier(str) => base.push(BasicPathElement {
                    key: BasicPathElementKey::Plain(str),
                    variant: if iter.peek() == Some(&Token::Colon) {
                        assert_eq!(iter.next().unwrap(), Token::Colon);
                        if !matches!(iter.peek(), Some(Token::Identifier(_))) {
                            return Err(Expected::StructuredEnumVariantIdentifier);
                        }
                        match iter.next().unwrap() {
                            Token::Identifier(str) => Some(str),
                            _ => unreachable!(),
                        }
                    } else {
                        None
                    },
                }),
                _ => unreachable!(),
            },

            Some(Token::Brac) => {
                assert_eq!(iter.next().unwrap(), Token::Brac);
                match iter.peek() {
                    Some(Token::Identifier(_)) => match iter.next().unwrap() {
                        Token::Identifier(str) => {
                            match iter.peek() {
                                Some(Token::Ket) => assert_eq!(iter.next().unwrap(), Token::Ket),
                                _ => return Err(Expected::Unspecific),
                            };
                            base.push(BasicPathElement {
                                key: BasicPathElementKey::List(str),
                                variant: if iter.peek() == Some(&Token::Colon) {
                                    assert_eq!(iter.next().unwrap(), Token::Colon);
                                    if !matches!(iter.peek(), Some(Token::Identifier(_))) {
                                        return Err(Expected::StructuredEnumVariantIdentifier);
                                    }
                                    match iter.next().unwrap() {
                                        Token::Identifier(str) => Some(str),
                                        _ => unreachable!(),
                                    }
                                } else {
                                    None
                                },
                            })
                        }
                        _ => unreachable!(),
                    },
                    _ => return Err(Expected::Unspecific),
                }
            }
            _ => return Err(Expected::Unspecific),
        }

        match iter.peek() {
            Some(Token::Period) => assert_eq!(iter.next().unwrap(), Token::Period),
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
) -> Result<Option<&'a mut Map<'b>>, ()>
where
    'b: 'c,
{
    for segment in path {
        match segment {
            PathSegment {
                tabular: Some(_), ..
            } => return Err(()),
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
                            Some(_) => return Err(()),
                            None => return Ok(None),
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
                        (Some(_), _) => return Err(()),
                        (None, _) => return Ok(None),
                    };
                }
            }
        }
    }

    Ok(Some(selected))
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

fn parse_key_value_pair<'a, Position>(
    iter: &mut Peekable<impl Iterator<Item = Token<'a, Position>>>,
    diagnostics: &mut Diagnostics<Position>,
) -> Result<(Key<'a>, Taml<'a>), ()> {
    Ok(match iter.peek() {
        Some(Token {
            token: lexerToken::Identifier(_),
            ..
        }) => match iter.next().unwrap().token {
            lexerToken::Identifier(key) => {
                if matches!(
                    iter.peek(),
                    Some(&Token {
                        token: lexerToken::Colon,
                        ..
                    })
                ) {
                    assert!(matches!(iter.next().unwrap(), Token{token:lexerToken::Colon,..}))
                } else {
                    diagnostics.push(Diagnostic {
                        r#type: DiagnosticType::ExpectedKeyValuePair,
                        labels: vec![DiagnosticLabel::new(
                            "Expected colon.",
                            iter.peek().map(|t| t.span.start..t.span.start),
                            DiagnosticLabelPriority::Primary,
                        )],
                    });
                    return Err(());
                }
                (key, parse_value(iter, diagnostics)?)
            }
            _ => unreachable!(),
        },

        _ => {
            diagnostics.push(Diagnostic {
                r#type: DiagnosticType::ExpectedKeyValuePair,
                labels: vec![DiagnosticLabel ::new(
                    "Structured sections can only contain subsections and key-value pairs.\nKey-value pairs must start with an identifier.",
                    iter.peek().map(|t| t.span),
                    DiagnosticLabelPriority::Primary,
                )],
            });
            return Err(());
        }
    })
}

fn parse_values_line<'a, Position>(
    iter: &mut Peekable<impl Iterator<Item = Token<'a, Position>>>,
    count: usize,
    diagnostics: &mut Diagnostics<Position>,
) -> Result<Vec<Taml<'a>>, ()> {
    let mut values = vec![];
    values.push(parse_value(iter, diagnostics)?);
    for _ in 1..count {
        if iter.peek().map(|t| &t.token) == Some(&lexerToken::Comma) {
            assert_eq!(iter.next().unwrap().token, lexerToken::Comma);
            values.push(parse_value(iter, diagnostics)?)
        } else {
            diagnostics.push(Diagnostic {
                r#type: DiagnosticType::ValuesLineTooShort,
                labels: vec![DiagnosticLabel::new(
                    "Expected comma here.",
                    iter.peek().map(|t| t.span.start..t.span.start),
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

fn parse_value<'a, Position>(
    iter: &mut Peekable<impl Iterator<Item = Token<'a, Position>>>,
    diagnostics: &mut Diagnostics<Position>,
) -> Result<Taml<'a>, ()> {
    fn err<'a, Position>(
        span: impl Into<Option<Range<Position>>>,
        diagnostics: &mut Diagnostics<Position>,
    ) -> Result<Taml<'a>, ()> {
        diagnostics.push(Diagnostic {
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
                    match iter.peek().map(|t| t.token) {
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
                    diagnostics.push(Diagnostic {
                        r#type: DiagnosticType::UnclosedList,
                        labels: vec![
                            DiagnosticLabel::new(
                                "This list starts here...",
                                span,
                                DiagnosticLabelPriority::Auxiliary,
                            ),
                            DiagnosticLabel::new(
                                "...but is unclosed at this point.",
                                iter.peek().map(|t| t.span.start..t.span.start),
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
