use crate::token::Token;
use std::iter::FromIterator;
use {
    smartstring::alias::String,
    std::{collections::HashMap, iter::Peekable},
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

impl<'a> FromIterator<Token<'a>> for Result<Taml<'a>, ()> {
    fn from_iter<T: IntoIterator<Item = Token<'a>>>(iter: T) -> Self {
        let iter = iter.into_iter().peekable();
    }
}

fn parse_heading<'a, 'b: 'a, 'c: 'b>(
    iter: &mut Peekable<impl Iterator<Item = Token<'a>>>,
    current_path: &mut Vec<PathSegment<'b>>,
    taml: &mut Taml<'c>,
) -> Result<Option<Selection<'c>>, Expected> {
    if iter.peek() != Some(&Token::HeadingHash) {
        return Ok(None);
    }

    let mut hash_count = 0;
    while iter.peek() == Some(&Token::HeadingHash) {
        assert_eq!(iter.next().unwrap(), Token::HeadingHash);
        hash_count += 1;
    }

    current_path.truncate(hash_count - 1);
    if current_path.len() < hash_count - 1 {
        return Err(Expected::Unspecific);
    }
    if let Some(last_segment) = current_path.last() {
        if last_segment.tabular.is_some() {
            // Can't nest further in tabular sections.
            return Err(Expected::Unspecific);
        }
    }

    let base = vec![];
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
                        tabular = Some(parse_tabular_path_segments(iter)?);
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

    let segment = PathSegment { base, tabular };
    let selected = taml
        .get_last_mut(&*current_path)
        .unwrap() // Instantiated correctly by previous headings.
        .unwrap()
        .instantiate(&segment)?;
    current_path.push(segment);

    Ok(Some(selected))
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
            _ => todo!("Try parse basic path segement"),
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

enum Selection<'a> {
    Map(&'a mut HashMap<Woc<'a, String, str>, Taml<'a>>),
    List(&'a mut Vec<Taml<'a>>),
}

impl<'a> Taml<'a> {
    fn get_last_mut<'b: 'a>(
        &'a mut self,
        path: impl IntoIterator<Item = &'b PathSegment<'b>>,
    ) -> Result<Option<Selection<'a>>, ()> {
        let path = path.into_iter();

        match path.next() {
            None => Ok(Some(match self {
                Taml::Map(map) => Selection::Map(map),
                Taml::List(list) => Selection::List(list),
                _ => return Err(()),
            })),
            Some(PathSegment {
                tabular: Some(_), ..
            }) => Err(()),
            Some(PathSegment {
                base,
                tabular: None,
            }) => {
                let mut selected = match self {
                    Taml::Map(map) => Selection::Map(map),
                    _ => return Err(()),
                };
                for path_element in base {
                    selected = match path_element {
                        BasicPathElement::Plain(key) => match selected {
                            Selection::Map(selected) => match selected.get_mut(key) {
                                Some(Taml::List(selected)) => Selection::List(&mut selected),
                                Some(Taml::Map(selected)) => Selection::Map(&mut selected),
                                Some(_) => return Err(()),
                                None => return Ok(None),
                            },
                            _ => return Err(()),
                        },
                        BasicPathElement::List(key) => match selected {
                            Selection::Map(selected) => match selected.get_mut(key) {
                                Some(Taml::List(selected)) => match selected.last_mut() {
                                    Some(Taml::Map(selected)) => Selection::Map(selected),
                                    Some(Taml::List(selected)) => Selection::List(selected),
                                    Some(_) => return Err(()),
                                    None => return Ok(None),
                                },
                                Some(_) => return Err(()),
                                None => return Ok(None),
                            },
                            _ => return Err(()),
                        },
                    }
                }
                Ok(Some(selected))
            }
        }
    }
}

fn parse_key_value_pair<'a>(
    iter: &mut Peekable<impl Iterator<Item = Token<'a>>>,
) -> Result<Option<(Woc<'a, String, str>, Taml<'a>)>, Expected> {
    Ok(match iter.peek().ok_or(Expected::Unspecific)? {
        Token::HeadingHash
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
        Token::HeadingHash | Token::Identifier(_) => None,

        Token::Paren | Token::String(_) | Token::Float(_) | Token::Integer(_) => {
            Some(match iter.next().unwrap() {
                Token::Paren => {
                    let mut items = vec![];
                    while iter.peek().ok_or(Expected::Unspecific)? != &Token::Thesis {
                        items.push(
                            parse_value(iter)
                                .map_err(|error| match error {
                                    Expected::Unspecific | Expected::Unspecific => {
                                        Expected::Unspecific
                                    }
                                })?
                                .ok_or(Expected::Unspecific)?,
                        )
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
