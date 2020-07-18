use {
    crate::token::Token,
    smartstring::alias::String,
    std::{
        collections::{hash_map, HashMap},
        iter::{FromIterator, Peekable},
    },
    woc::Woc,
};

#[derive(Debug)]
#[non_exhaustive]
pub enum Expected {
    Unspecific,
}

#[derive(Debug)]
pub enum Taml<'a> {
    String(Woc<'a, String, str>),
    Boolean(bool),
    Integer(&'a str),
    Float(&'a str),
    List(Vec<Taml<'a>>),
    Map(HashMap<Woc<'a, String, str>, Taml<'a>>),
}

struct PathSegment<'a> {
    base: Vec<BasicPathElement<'a>>,
    tabular: Option<TabularPathSegment<'a>>,
}

#[derive(Clone)]
enum BasicPathElement<'a> {
    Plain(Woc<'a, String, str>),
    List(Woc<'a, String, str>),
}

struct TabularPathSegment<'a> {
    base: Vec<BasicPathElement<'a>>,
    multi: Option<Vec<TabularPathSegment<'a>>>,
}

type Map<'a> = HashMap<Woc<'a, String, str>, Taml<'a>>;

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
        let Selection::Map(selection) = Selection::Map(selection)
            .instantiate(self.base.iter().take(self.base.len() - 1).cloned())?;

        //TODO: Fail if keys exist!
        let selection = match self.base.last().unwrap() {
            BasicPathElement::Plain(key) => selection
                .entry(key.clone())
                .or_insert_with(|| Taml::String(Woc::Borrowed("PLACEHOLDER"))),
            BasicPathElement::List(key) => {
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

        if let Some(multi) = &self.multi {
            *selection = Taml::Map(HashMap::new());
            let selection = match selection {
                Taml::Map(map) => map,
                _ => unreachable!(),
            };
            for child in multi {
                child.assign(selection, values.by_ref())?
            }
            Ok(())
        } else {
            *selection = values.next().ok_or(())?;
            Ok(())
        }
    }
}

impl<'a> FromIterator<Token<'a>> for Result<HashMap<Woc<'a, String, str>, Taml<'a>>, Expected> {
    fn from_iter<T: IntoIterator<Item = Token<'a>>>(iter: T) -> Self {
        let mut iter = iter.into_iter().peekable();

        let mut taml = HashMap::new();

        let mut path = vec![];

        let mut selection = Selection::Map(&mut taml);

        while let Some(next) = iter.peek() {
            match next {
                Token::Comment(_) => assert!(matches!(iter.next().unwrap(), Token::Comment(_))),
                Token::HeadingHashes(_) => {
                    let depth = match iter.next().unwrap() {
                        Token::HeadingHashes(count) => count,
                        _ => unreachable!(),
                    };

                    path.truncate(depth - 1);
                    if path.len() != depth - 1 {
                        return Err(Expected::Unspecific);
                    }
                    if path
                        .last()
                        .and_then(|s: &PathSegment| s.tabular.as_ref())
                        .is_some()
                    {
                        return Err(Expected::Unspecific);
                    }

                    let new_segment = parse_path_segment(&mut iter)?;

                    selection = Selection::Map(&mut taml)
                        .get_last_mut(path.iter())
                        .map_err(|()| Expected::Unspecific)?
                        .ok_or(Expected::Unspecific)?
                        .instantiate(new_segment.base.iter().cloned())
                        .map_err(|()| Expected::Unspecific)?;

                    path.push(new_segment);
                }
                Token::Newline => assert_eq!(iter.next().unwrap(), Token::Newline),
                _ => match &mut selection {
                    Selection::Map(selection) => {
                        #[allow(clippy::single_match_else)]
                        match path.last().and_then(|s| s.tabular.as_ref()) {
                            Some(tabular) => {
                                let n = tabular.arity();
                                let values = parse_values_line(&mut iter, n)?;

                                tabular
                                    .assign(*selection, &mut values.into_iter())
                                    .map_err(|()| Expected::Unspecific)?
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
                },
            }
        }

        Ok(taml)
    }
}

//TODO: Fix lifetimes.
fn parse_path_segment<'a, 'b, 'c>(
    iter: &mut Peekable<impl Iterator<Item = Token<'a>>>,
) -> Result<PathSegment<'a>, Expected> {
    let mut base = vec![];
    let mut tabular = None;

    if let Some(next) = iter.peek() {
        if matches!(next, Token::Comment(_) | Token::Newline) {
            return Ok(PathSegment { base, tabular });
        }
    }

    loop {
        match iter.peek() {
            None => break,
            Some(Token::Identifier(_)) => match iter.next().unwrap() {
                Token::Identifier(str) => base.push(BasicPathElement::Plain(str)),
                _ => unreachable!(),
            },
            Some(Token::Brac) => {
                assert_eq!(iter.next().unwrap(), Token::Brac);
                match iter.peek().ok_or(Expected::Unspecific)? {
                    Token::Identifier(_) => match iter.next().unwrap() {
                        Token::Identifier(str) => base.push(BasicPathElement::List(str)),
                        _ => unreachable!(),
                    },
                    Token::Brac => {
                        tabular = Some(parse_tabular_path_segment(iter)?);
                    }
                    _ => return Err(Expected::Unspecific),
                }
                match iter.peek() {
                    Some(Token::Ket) => assert_eq!(iter.next().unwrap(), Token::Ket),
                    _ => return Err(Expected::Unspecific),
                }
            }
            _ => return Err(Expected::Unspecific),
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

fn parse_tabular_path_segments<'a>(
    iter: &mut Peekable<impl Iterator<Item = Token<'a>>>,
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

fn parse_tabular_path_segment<'a>(
    iter: &mut Peekable<impl Iterator<Item = Token<'a>>>,
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
            Some(Token::Identifier(_)) => match iter.next().unwrap() {
                Token::Identifier(str) => base.push(BasicPathElement::Plain(str)),
                _ => unreachable!(),
            },
            Some(Token::Brac) => {
                assert_eq!(iter.next().unwrap(), Token::Brac);
                match iter.peek() {
                    Some(Token::Identifier(_)) => match iter.next().unwrap() {
                        Token::Identifier(str) => base.push(BasicPathElement::List(str)),
                        _ => unreachable!(),
                    },
                    _ => return Err(Expected::Unspecific),
                }
                match iter.peek() {
                    Some(Token::Ket) => assert_eq!(iter.next().unwrap(), Token::Ket),
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

//TODO: Get rid of this enum entirely.
enum Selection<'a, 'b> {
    Map(&'a mut HashMap<Woc<'b, String, str>, Taml<'b>>),
}

impl<'a, 'b> Selection<'a, 'b> {
    fn get_last_mut<'c>(
        self,
        path: impl IntoIterator<Item = &'c PathSegment<'c>>,
    ) -> Result<Option<Selection<'a, 'b>>, ()> {
        let mut selected: Selection<'a, 'b> = self;
        for segment in path.into_iter() {
            match segment {
                PathSegment {
                    tabular: Some(_), ..
                } => return Err(()),
                PathSegment {
                    base,
                    tabular: None,
                } => {
                    for path_element in base {
                        let Selection::Map(map) = selected;
                        let value = match path_element {
                            BasicPathElement::Plain(key) => map.get_mut(key.as_ref()),

                            BasicPathElement::List(key) => match map.get_mut(key.as_ref()) {
                                Some(Taml::List(selected)) => selected.last_mut(),
                                Some(_) => return Err(()),
                                None => return Ok(None),
                            },
                        };

                        selected = match value {
                            Some(Taml::Map(map)) => Selection::Map(map),
                            Some(_) => return Err(()),
                            None => return Ok(None),
                        };
                    }
                }
            }
        }

        Ok(Some(selected))
    }

    fn instantiate<'c>(
        self,
        path: impl IntoIterator<Item = BasicPathElement<'b>>,
    ) -> Result<Selection<'c, 'b>, ()>
    where
        'a: 'c,
    {
        let mut selection = match self {
            Selection::Map(map) => Selection::Map(map),
        };

        for path_element in path {
            let Selection::Map(map) = selection;
            let value = match path_element {
                BasicPathElement::Plain(key) => map
                    .entry(key.clone())
                    .or_insert_with(|| Taml::Map(HashMap::new())),
                BasicPathElement::List(key) => {
                    let list = map.entry(key.clone()).or_insert_with(|| Taml::List(vec![]));
                    match list {
                        Taml::List(list) => {
                            list.push(Taml::Map(HashMap::new()));
                            list.last_mut().unwrap()
                        }
                        _ => unreachable!(),
                    }
                }
            };
            selection = match value {
                Taml::Map(map) => Selection::Map(map),
                _ => return Err(()),
            };
        }
        Ok(selection)
    }
}

fn parse_key_value_pair<'a>(
    iter: &mut Peekable<impl Iterator<Item = Token<'a>>>,
) -> Result<Option<(Woc<'a, String, str>, Taml<'a>)>, Expected> {
    Ok(match iter.peek().ok_or(Expected::Unspecific)? {
        Token::HeadingHashes(_)
        | Token::Paren
        | Token::String(_)
        | Token::Float(_)
        | Token::Integer(_) => None,

        Token::Identifier(_) => Some(match iter.next().unwrap() {
            Token::Identifier(key) => {
                if iter.next().ok_or(Expected::Unspecific)? != Token::Colon {
                    return Err(Expected::Unspecific);
                }
                (key, parse_value(iter)?.ok_or(Expected::Unspecific)?)
            }
            _ => unreachable!(),
        }),

        _ => return Err(Expected::Unspecific),
    })
}

fn parse_values_line<'a>(
    iter: &mut Peekable<impl Iterator<Item = Token<'a>>>,
    count: usize,
) -> Result<Vec<Taml<'a>>, Expected> {
    let mut values = vec![];
    values.push(parse_value(iter)?.ok_or(Expected::Unspecific)?);
    for _ in 1..count {
        if iter.peek() == Some(&Token::Comma) {
            assert_eq!(iter.next().unwrap(), Token::Comma);
            values.push(parse_value(iter)?.ok_or(Expected::Unspecific)?)
        } else {
            return Err(Expected::Unspecific);
        }
    }
    if iter.peek() == Some(&Token::Comma) {
        assert_eq!(iter.next().unwrap(), Token::Comma);
    }
    Ok(values)
}

fn parse_value<'a>(
    iter: &mut Peekable<impl Iterator<Item = Token<'a>>>,
) -> Result<Option<Taml<'a>>, Expected> {
    Ok(match iter.peek().ok_or(Expected::Unspecific)? {
        Token::HeadingHashes(_) => None,

        Token::Paren
        | Token::String(_)
        | Token::Float(_)
        | Token::Integer(_)
        | Token::Identifier(_) => Some(match iter.next().unwrap() {
            Token::Paren => {
                let mut items = vec![];
                while iter.peek().ok_or(Expected::Unspecific)? != &Token::Thesis {
                    items.push(parse_value(iter)?.ok_or(Expected::Unspecific)?);
                    match iter.peek() {
                        Some(Token::Comma) => assert_eq!(iter.next().unwrap(), Token::Comma),
                        _ => break,
                    }
                }
                if iter.peek() == Some(&Token::Thesis) {
                    assert_eq!(iter.next().unwrap(), Token::Thesis);
                    Taml::List(items)
                } else {
                    return Err(Expected::Unspecific);
                }
            }

            Token::String(str) => Taml::String(str),
            Token::Float(str) => Taml::Float(str),
            Token::Integer(str) => Taml::Integer(str),
            Token::Identifier(str) => {
                if str.as_ref() == "true" {
                    Taml::Boolean(true)
                } else if str.as_ref() == "false" {
                    Taml::Boolean(false)
                } else {
                    return Ok(None);
                }
            }

            _ => unreachable!(),
        }),

        _ => return Err(Expected::Unspecific),
    })
}
