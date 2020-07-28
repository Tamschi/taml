#![warn(clippy::pedantic)]
#![allow(unused_variables)] //TODO
#![allow(clippy::default_trait_access)] // because of derive(FromArgs).

use {
    argh::FromArgs,
    cast::u64,
    logos::Logos as _,
    std::{
        ffi::OsStr,
        fs,
        io::Write as _,
        path::{Path, PathBuf},
    },
    taml::{
        formatting::{CanonicalFormatScanner, Recommendation},
        parser::parse,
        token::Token,
    },
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
    Check(Check),
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

#[derive(Debug, FromArgs)]
/// Validate TAML files.
/// Exit code: number of errors reported
#[argh(subcommand, name = "check")]
struct Check {
    #[argh(positional)]
    /// A file or folder to validate.
    /// Defaults to `.`.
    path: Option<PathBuf>,

    /// hide scanned files from stdout
    #[argh(switch, short = 'q')]
    quiet: bool,
}

//TODO: Atomic file replacements.
#[allow(clippy::too_many_lines)]
#[quit::main]
fn main() {
    let arghs: Arghs = argh::from_env();

    #[allow(clippy::items_after_statements)]
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

                let mut format_scanner = CanonicalFormatScanner::new();
                for token in tokens {
                    match format_scanner.next(&token) {
                        Recommendation::Recommended | Recommendation::Required => {
                            write!(&mut file, "{}", token)
                        }
                        Recommendation::PrependSpace | Recommendation::PrependSpaceRequired => {
                            write!(&mut file, " {}", token)
                        }
                        Recommendation::PrependNewline => write!(&mut file, "\n{}", token),
                        Recommendation::PrependTwoNewlines => write!(&mut file, "\n\n{}", token),
                        Recommendation::SkipToken => Ok(()),
                    }
                    .unwrap()
                }

                writeln!(&mut file).unwrap();
            }
        }

        SubCommand::Check(Check { path, quiet }) => {
            use {
                codemap::CodeMap,
                codemap_diagnostic::{
                    ColorConfig, Diagnostic, Emitter, Level, SpanLabel, SpanStyle,
                },
            };

            let path = path.unwrap_or_else(|| ".".into());

            let mut codemap = CodeMap::new();
            let mut diagnostics = vec![];
            check_path(&path, &mut codemap, &mut diagnostics, quiet);

            if !diagnostics.is_empty() {
                let mut emitter = Emitter::stderr(ColorConfig::Auto, Some(&codemap));
                emitter.emit(&diagnostics);
            }

            quit::with_code(
                cast::i32(diagnostics.len()).expect("Too many diagnostics for exit code"),
            );

            //TODO: I should refactor these into closures with fewer arguments.
            fn check_path(
                path: impl AsRef<Path>,
                codemap: &mut CodeMap,
                diagnostics: &mut Vec<Diagnostic>,
                quiet: bool,
            ) {
                let meta = fs::metadata(path.as_ref()).unwrap();
                if meta.is_dir() {
                    check_dir(path, codemap, diagnostics, quiet)
                } else {
                    check_file(path, false, codemap, diagnostics, quiet)
                }
            }

            fn check_dir(
                path: impl AsRef<Path>,
                codemap: &mut CodeMap,
                diagnostics: &mut Vec<Diagnostic>,
                quiet: bool,
            ) {
                for entry in fs::read_dir(path).unwrap() {
                    let entry = entry.unwrap();
                    let meta = entry.metadata().unwrap();
                    if meta.is_dir() {
                        check_dir(entry.path(), codemap, diagnostics, quiet)
                    } else if meta.is_file() {
                        check_file(entry.path(), true, codemap, diagnostics, quiet)
                    }
                }
            }

            fn check_file(
                path: impl AsRef<Path>,
                check_extension: bool,
                codemap: &mut CodeMap,
                diagnostics: &mut Vec<Diagnostic>,
                quiet: bool,
            ) {
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

                let lexer = Token::lexer(&text).spanned();

                let mut file_diagnostics = vec![];
                let taml = parse(lexer, &mut file_diagnostics);

                match taml {
                    Ok(taml) =>
                    {
                        #[allow(clippy::non_ascii_literal)]
                        if !quiet {
                            println!("✓ {}", path.as_ref().to_string_lossy())
                        }
                    }
                    Err(expected) =>
                    {
                        #[allow(clippy::non_ascii_literal)]
                        if !quiet {
                            println!("✕ {}", path.as_ref().to_string_lossy())
                        }
                    }
                }
                if !quiet && !file_diagnostics.is_empty() {
                    let file_span = codemap
                        .add_file(path.as_ref().to_string_lossy().to_string(), text)
                        .span;

                    diagnostics.extend(file_diagnostics.into_iter().map(|diagnostic| {
                        Diagnostic {
                            code: Some(diagnostic.code()),
                            level: match diagnostic.level() {
                                taml::diagnostics::DiagnosticLevel::Warning => Level::Warning,
                                taml::diagnostics::DiagnosticLevel::Error => Level::Error,
                            },
                            message: diagnostic.message().to_string(),
                            spans: diagnostic
                                .labels
                                .into_iter()
                                .map(|label| SpanLabel {
                                    label: label.caption.map(|c| c.to_string()),
                                    style: match label.priority {
                                        taml::diagnostics::DiagnosticLabelPriority::Primary => {
                                            SpanStyle::Primary
                                        }
                                        taml::diagnostics::DiagnosticLabelPriority::Auxiliary => {
                                            SpanStyle::Secondary
                                        }
                                    },
                                    span: match label.span {
                                        Some(span) => {
                                            file_span.subspan(u64(span.start), u64(span.end))
                                        }
                                        None => file_span.subspan(file_span.len(), file_span.len()),
                                    },
                                })
                                .collect(),
                        }
                    }))
                }
            }
        }
    }
}
