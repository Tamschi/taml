use codemap::CodeMap;
use codemap_diagnostic::{ColorConfig, Diagnostic, Emitter, Level, SpanLabel, SpanStyle};
use taml::diagnostics::{DiagnosticLabel, DiagnosticLabelPriority, DiagnosticType};
use {cast::u64, serde::Deserialize, taml::deserializer::from_str};

#[derive(Debug, PartialEq, Deserialize)]
struct NoFields {}

#[allow(non_camel_case_types)]
type tamlDiagnostic = taml::diagnostics::Diagnostic<usize>;

#[test]
fn no_fields() {
    let text = "key: \"value\"\n";
    let mut diagnostics = vec![];
    from_str::<NoFields, _>(text, &mut diagnostics).unwrap_err();
    assert_eq!(
        diagnostics.as_slice(),
        &[tamlDiagnostic {
            r#type: DiagnosticType::UnknownField,
            labels: vec![DiagnosticLabel::new(
                "Expected no fields.",
                0..3,
                DiagnosticLabelPriority::Primary,
            )]
        }]
    );
    report(text, diagnostics)
}

#[test]
fn no_fields_multi() {
    let text = "key: \"value\"\n\
    another: \"value\"\n";
    let mut diagnostics = vec![];
    from_str::<NoFields, _>(text, &mut diagnostics).unwrap_err();
    assert_eq!(
        diagnostics.as_slice(),
        &[
            tamlDiagnostic {
                r#type: DiagnosticType::UnknownField,
                labels: vec![DiagnosticLabel::new(
                    "Expected no fields.",
                    0..3,
                    DiagnosticLabelPriority::Primary,
                )]
            },
            tamlDiagnostic {
                r#type: DiagnosticType::UnknownField,
                labels: vec![DiagnosticLabel::new(
                    "Expected no fields.",
                    13..20,
                    DiagnosticLabelPriority::Primary,
                )]
            },
        ]
    );
    report(text, diagnostics)
}

fn report(text: &str, diagnostics: Vec<tamlDiagnostic>) {
    let mut codemap = CodeMap::new();
    let file_span = codemap.add_file("TAML".to_string(), text.to_string()).span;

    let diagnostics: Vec<_> = diagnostics
        .into_iter()
        .map(|diagnostic| Diagnostic {
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
                        taml::diagnostics::DiagnosticLabelPriority::Primary => SpanStyle::Primary,
                        taml::diagnostics::DiagnosticLabelPriority::Auxiliary => {
                            SpanStyle::Secondary
                        }
                    },
                    span: match label.span {
                        Some(span) => file_span.subspan(u64(span.start), u64(span.end)),
                        None => file_span.subspan(file_span.len(), file_span.len()),
                    },
                })
                .collect(),
        })
        .collect();

    if !diagnostics.is_empty() {
        Emitter::stderr(ColorConfig::Auto, Some(&codemap)).emit(&diagnostics)
    }
}
