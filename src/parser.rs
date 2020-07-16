use {
    crate::token::Token,
    smartstring::alias::String,
    std::{
        collections::HashMap,
        iter::{FromIterator, Peekable},
    },
    woc::Woc,
};

#[non_exhaustive]
pub enum Expected {
    Unspecific,
}

pub enum Taml<'a> {
    String(Woc<'a, String, str>),
    Integer(&'a str),
    Float(&'a str),
    List(Vec<Taml<'a>>),
    Map(HashMap<Woc<'a, String, str>, Taml<'a>>),
}

struct PathSegment<'a> {
    base: Vec<BasicPathElement<'a>>,
    tabular: Option<TabularPathSegment<'a>>,
}

enum BasicPathElement<'a> {
    Plain(Woc<'a, String, str>),
    List(Woc<'a, String, str>),
}

struct TabularPathSegment<'a> {
    base: Vec<BasicPathElement<'a>>,
    multi: Option<Vec<TabularPathSegment<'a>>>,
}

impl<'a> TabularPathSegment<'a> {
    fn arity(&self) -> usize {
        match &self.multi {
            None => 1,
            Some(multi) => multi.iter().map(Self::arity).sum(),
        }
    }
}

impl<'a> FromIterator<Token<'a>> for Result<HashMap<Woc<'a, String, str>, Taml<'a>>, Expected> {
    fn from_iter<T: IntoIterator<Item = Token<'a>>>(iter: T) -> Self {
        let mut iter = iter.into_iter().peekable();

        let mut taml = HashMap::new();

        parse_section(&mut iter, &mut Selection::Map(&mut taml), 0)?;

        Ok(taml)
    }
}

fn parse_section<'a, 'b>(
    iter: &mut Peekable<impl Iterator<Item = Token<'b>>>,
    selection: &'a mut Selection<'a, 'b>,
    depth: usize,
) -> Result<(), Expected> {
    while let Some(next) = iter.peek() {
        match next {
            Token::Comment(_) => assert!(matches!(iter.next().unwrap(), Token::Comment(_))),
            Token::Newline => assert_eq!(iter.next().unwrap(), Token::Newline),
            Token::HeadingHashes(count) if *count == depth + 1 => {
                let mut selection = parse_heading(iter, selection, depth)?.unwrap();

                parse_section(iter, &mut selection, depth + 1)?
            }
            Token::HeadingHashes(count) if *count <= depth => return Ok(()),
            Token::HeadingHashes(_) => return Err(Expected::Unspecific),
            _ => todo!("Parse lines"),
        }
    }

    Ok(())
}

//TODO: Fix lifetimes.
fn parse_heading<'a, 'b, 'c>(
    iter: &mut Peekable<impl Iterator<Item = Token<'b>>>,
    selection: &'c mut Selection<'a, 'b>,
    depth: usize,
) -> Result<Option<Selection<'c, 'b>>, Expected> {
    match iter.peek() {
        Some(&Token::HeadingHashes(count)) if count == depth + 1 => {
            assert_eq!(iter.next().unwrap(), Token::HeadingHashes(count))
        }
        _ => return Ok(None),
    }

    let mut base = vec![];
    let mut tabular = None;
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

    //TODO: This is a REALLY ugly hack.
    let selection =
        unsafe { std::mem::transmute_copy::<Selection<'_, 'b>, Selection<'c, 'b>>(selection) }
            .instantiate(base.iter().map(|s| match s {
                BasicPathElement::Plain(str) => {
                    BasicPathElement::Plain(Woc::Owned(str.clone().into_owned()))
                }
                BasicPathElement::List(str) => {
                    BasicPathElement::List(Woc::Owned(str.to_owned().clone().into_owned()))
                }
            }))
            .map_err(|()| Expected::Unspecific)?;

    Ok(Some(unsafe { std::mem::transmute(selection) }))
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

enum Selection<'a, 'b> {
    Map(&'a mut HashMap<Woc<'b, String, str>, Taml<'b>>),
    List(&'a mut Vec<Taml<'b>>),
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
                        let map = match selected {
                            Selection::Map(map) => map,
                            _ => return Err(()),
                        };
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
                            Some(Taml::List(list)) => Selection::List(list),
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
            Selection::List(list) => Selection::List(list),
        };

        for path_element in path {
            let map = match selection {
                Selection::Map(map) => map,
                _ => return Err(()),
            };
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
                Taml::List(list) => Selection::List(list),
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
        Token::HeadingHashes(_) | Token::Identifier(_) => None,

        Token::Paren | Token::String(_) | Token::Float(_) | Token::Integer(_) => {
            Some(match iter.next().unwrap() {
                Token::Paren => {
                    let mut items = vec![];
                    while iter.peek().ok_or(Expected::Unspecific)? != &Token::Thesis {
                        items.push(parse_value(iter)?.ok_or(Expected::Unspecific)?)
                    }
                    assert_eq!(iter.next().unwrap(), Token::Thesis);
                    Taml::List(items)
                }

                Token::String(str) => Taml::String(str),
                Token::Float(str) => Taml::Float(str),
                Token::Integer(str) => Taml::Integer(str),

                _ => unreachable!(),
            })
        }

        _ => return Err(Expected::Unspecific),
    })
}
