#![allow(unused_variables)] //TODO

use {
    argh::FromArgs,
    logos::Logos as _,
    std::{
        ffi::OsStr,
        fs,
        io::Write as _,
        path::{Path, PathBuf},
    },
    taml::token::Token,
};

#[derive(Debug, FromArgs)]
/// Tamme's Amazing Markup Language
struct Arghs {
    #[argh(subcommand)]
    subcommand: SubCommand,
}

#[derive(Debug, FromArgs)]
#[argh(subcommand)]
enum SubCommand {
    Fmt(Fmt),
}

#[derive(Debug, FromArgs)]
/// Format TAML files.
#[argh(subcommand, name = "fmt")]
struct Fmt {
    #[argh(positional)]
    /// A file or folder to format.
    /// Defaults to `.`.
    path: Option<PathBuf>,
}

fn main() {
    let arghs: Arghs = argh::from_env();

    match arghs.subcommand {
        SubCommand::Fmt(Fmt { path }) => {
            let path = path.unwrap_or_else(|| ".".into());
            format_path(&path);

            fn format_path(path: impl AsRef<Path>) {
                let meta = fs::metadata(path.as_ref()).unwrap();
                if meta.is_dir() {
                    format_dir(path)
                } else {
                    format_file(path, false)
                }
            }

            fn format_dir(path: impl AsRef<Path>) {
                for entry in fs::read_dir(path).unwrap() {
                    let entry = entry.unwrap();
                    let meta = entry.metadata().unwrap();
                    if meta.is_dir() {
                        format_dir(entry.path())
                    } else if meta.is_file() {
                        format_file(entry.path(), true)
                    }
                }
            }

            fn format_file(path: impl AsRef<Path>, check_extension: bool) {
                if check_extension {
                    if let Some(extension) = path.as_ref().extension().and_then(OsStr::to_str) {
                        if extension.to_ascii_lowercase() != "taml" {
                            return;
                        }
                    } else {
                        return;
                    }
                }

                let text = fs::read_to_string(path.as_ref()).unwrap();
                let mut tokens = vec![];
                let lexer = Token::lexer(&text);
                for (token, span) in lexer.spanned() {
                    assert_ne!(
                        token,
                        Token::Error,
                        "Error in {} at {:?}: found {}",
                        path.as_ref().display(),
                        span,
                        &text[span.clone()]
                    );
                    tokens.push(token)
                }

                while tokens.last() == Some(&Token::Newline) {
                    tokens.pop();
                }

                let mut file = fs::OpenOptions::new()
                    .write(true)
                    .truncate(true)
                    .open(path)
                    .unwrap();

                let mut newline_count = usize::MAX;
                let mut hashed = false;
                let mut identified = false;

                for token in tokens {
                    match &token {
                        Token::Newline if newline_count >= 2 => Ok(()),
                        Token::Newline | Token::HeadingHash => {
                            newline_count += 1;
                            write!(&mut file, "{}", token)
                        }
                        _ if hashed => write!(&mut file, " {}", token),
                        Token::Identifier(_) if identified => write!(&mut file, " {}", token),
                        Token::Comma | Token::Colon => write!(&mut file, "{} ", token),
                        _ => write!(&mut file, "{}", token),
                    }
                    .unwrap();

                    hashed = token == Token::HeadingHash;
                    identified = matches!(token, Token::Identifier(_));
                    if token != Token::Newline {
                        newline_count = 0
                    }
                }

                write!(&mut file, "{}", Token::Newline).unwrap();
            }
        }
    }
}
