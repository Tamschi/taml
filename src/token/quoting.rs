use std::fmt::Display;
use {
    core::{borrow::Borrow, ops::Deref},
    smartstring::alias::String,
    std::borrow::ToOwned,
};

#[derive(Debug, PartialEq)]
pub enum Woc<'a, T, R: ?Sized> {
    Owned(T),
    Borrowed(&'a R),
}

impl<'a, T: Borrow<R>, R: ToOwned<Owned = T>> Woc<'a, T, R> {
    fn into_owned(self) -> T {
        match self {
            Woc::Owned(t) => t,
            Woc::Borrowed(r) => r.to_owned(),
        }
    }
}

impl<'a, T: AsRef<R>, R: ?Sized> AsRef<R> for Woc<'a, T, R> {
    fn as_ref(&self) -> &R {
        match self {
            Woc::Owned(t) => t.as_ref(),
            Woc::Borrowed(r) => r,
        }
    }
}

impl<'a, T: Deref<Target = R>, R: ?Sized> Deref for Woc<'a, T, R> {
    type Target = R;
    fn deref(&self) -> &Self::Target {
        match self {
            Woc::Owned(t) => t,
            Woc::Borrowed(r) => r,
        }
    }
}

impl<'a, T: Borrow<R>, R: ?Sized> Borrow<R> for Woc<'a, T, R> {
    fn borrow(&self) -> &R {
        match self {
            Woc::Owned(t) => t.borrow(),
            Woc::Borrowed(r) => r,
        }
    }
}

impl<'a, T: Display, R: ?Sized + Display> Display for Woc<'a, T, R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Woc::Owned(t) => t.fmt(f),
            Woc::Borrowed(r) => r.fmt(f),
        }
    }
}

impl<'a, T: Clone, R: ?Sized> Clone for Woc<'a, T, R> {
    fn clone(&self) -> Self {
        match self {
            Woc::Owned(t) => Self::Owned(t.clone()),
            Woc::Borrowed(r) => Self::Borrowed(r),
        }
    }
}

enum TransformedPart {
    Unchanged,
    Changed(String),
}

trait Transform {
    fn transform(
        &self,
        transform_next: impl FnMut(&mut &str) -> TransformedPart,
    ) -> Woc<String, str>;
}

impl Transform for str {
    fn transform(
        &self,
        mut transform_next: impl FnMut(&mut &str) -> TransformedPart,
    ) -> Woc<String, str> {
        let mut rest = self;
        let mut copied = loop {
            if rest.is_empty() {
                return Woc::Borrowed(self);
            }
            let rest_len = rest.len();
            if let TransformedPart::Changed(transformed) = transform_next(&mut rest) {
                let mut copied = String::from(&self[..self.len() - rest_len]);
                copied.push_str(&transformed);
                break copied;
            }
        };

        while !rest.is_empty() {
            let unchanged_rest = rest;
            match transform_next(&mut rest) {
                TransformedPart::Unchanged => {
                    copied.push_str(&unchanged_rest[..unchanged_rest.len() - rest.len()]);
                }
                TransformedPart::Changed(changed) => copied.push_str(&changed),
            }
        }

        Woc::Owned(copied)
    }
}

pub fn escape_string_contents(string: &str) -> Woc<String, str> {
    string.transform(|rest| match rest.chars().next().unwrap() {
        c @ '\\' | c @ '"' => {
            *rest = &rest[c.len_utf8()..];
            let mut changed = String::from(r"\");
            changed.push(c);
            TransformedPart::Changed(changed)
        }
        c => {
            *rest = &rest[c.len_utf8()..];
            TransformedPart::Unchanged
        }
    })
}

pub fn unescape_string_contents(string: &str) -> Woc<String, str> {
    let mut escaped = false;
    string.transform(|rest| match rest.chars().next().unwrap() {
        '\\' if !escaped => {
            *rest = &rest['\\'.len_utf8()..];
            escaped = true;
            TransformedPart::Changed(String::new())
        }
        c => {
            escaped = false;
            *rest = &rest[c.len_utf8()..];
            TransformedPart::Unchanged
        }
    })
}

pub fn escape_identifier(string: &str) -> Woc<String, str> {
    let mut quote = if let Some(first) = string.chars().next() {
        first == '-' || first.is_ascii_digit()
    } else {
        true
    };
    let escaped_name = string.transform(|rest| match rest.chars().next().unwrap() {
        c @ '\\' | c @ '`' => {
            quote = true;
            *rest = &rest[c.len_utf8()..];
            let mut changed = String::from(r"\");
            changed.push(c);
            TransformedPart::Changed(changed)
        }
        c => {
            if !(('a'..='z').contains(&c)
                || ('A'..='Z').contains(&c)
                || c == '-'
                || c == '_'
                || ('0'..'9').contains(&c))
            {
                quote = true
            }
            *rest = &rest[c.len_utf8()..];
            TransformedPart::Unchanged
        }
    });
    if quote {
        let mut quoted = String::from("`");
        quoted.push_str(&escaped_name);
        quoted.push('`');
        Woc::Owned(quoted)
    } else {
        escaped_name
    }
}

pub fn unescape_quoted_identifier(string: &str) -> Woc<String, str> {
    assert!(string.starts_with('`'));
    assert!(string.ends_with('`'));
    let string = &string['`'.len_utf8()..string.len() - '`'.len_utf8()];
    let mut escaped = false;
    string.transform(|rest| match rest.chars().next().unwrap() {
        '\\' if !escaped => {
            escaped = true;
            *rest = &rest['\\'.len_utf8()..];
            TransformedPart::Changed(String::new())
        }
        c => {
            escaped = false;
            *rest = &rest[c.len_utf8()..];
            TransformedPart::Unchanged
        }
    })
}
