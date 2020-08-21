use cast::u64;
use codemap::CodeMap;
use codemap_diagnostic::{ColorConfig, Diagnostic, Emitter, Level, SpanLabel, SpanStyle};
use educe::*;
use indexmap::{indexmap, IndexMap};
use serde::{de::IgnoredAny, Deserialize};
use std::io::stdout;
use taml::{
	deserializer::from_str,
	diagnostics::{DiagnosticLabel, DiagnosticLabelPriority, DiagnosticType},
};

#[allow(non_camel_case_types)]
type tamlDiagnostic = taml::diagnostics::Diagnostic<usize>;

#[derive(Debug, PartialEq, Deserialize)]
struct NoFields {}

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

#[derive(Debug, PartialEq, Deserialize)]
struct ThreeFields {
	#[serde(default)]
	field_1: Option<i8>,

	#[serde(default)]
	field_2: Option<String>,

	#[serde(default)]
	field_3: Option<f32>,
}

#[test]
fn expected_other_fields() {
	let text = "key: \"value\"\n";
	let mut diagnostics = vec![];
	from_str::<ThreeFields, _>(text, &mut diagnostics).unwrap_err();
	assert_eq!(
		diagnostics.as_slice(),
		&[tamlDiagnostic {
			r#type: DiagnosticType::UnknownField,
			labels: vec![DiagnosticLabel::new(
				"Expected `field_1`, `field_2` or `field_3`.",
				0..3,
				DiagnosticLabelPriority::Primary,
			)]
		}]
	);
	report(text, diagnostics)
}

#[derive(Debug, PartialEq, Deserialize)]
struct TypedFields {
	#[serde(default)]
	i8: Option<i8>,

	#[serde(default)]
	string: Option<String>,

	#[serde(default)]
	f32: Option<f32>,
}

#[test]
fn expect_i8() {
	let text = "i8: \"value\"\n";
	let mut diagnostics = vec![];
	from_str::<TypedFields, _>(text, &mut diagnostics).unwrap_err();
	assert_eq!(
		diagnostics.as_slice(),
		&[tamlDiagnostic {
			r#type: DiagnosticType::InvalidType,
			labels: vec![DiagnosticLabel::new(
				"Expected i8 here.",
				4..11,
				DiagnosticLabelPriority::Primary,
			)]
		}]
	);
	report(text, diagnostics)
}

#[test]
fn expect_string() {
	let text = "string: 0\n";
	let mut diagnostics = vec![];
	from_str::<TypedFields, _>(text, &mut diagnostics).unwrap_err();
	assert_eq!(
		diagnostics.as_slice(),
		&[tamlDiagnostic {
			r#type: DiagnosticType::InvalidType,
			labels: vec![DiagnosticLabel::new(
				"Expected a string here.",
				8..9,
				DiagnosticLabelPriority::Primary,
			)]
		}]
	);
	report(text, diagnostics)
}

#[test]
fn expect_f32() {
	let text = "f32: (1, 2, 3, 4, 5)\n";
	let mut diagnostics = vec![];
	from_str::<TypedFields, _>(text, &mut diagnostics).unwrap_err();
	assert_eq!(
		diagnostics.as_slice(),
		&[tamlDiagnostic {
			r#type: DiagnosticType::InvalidType,
			labels: vec![DiagnosticLabel::new(
				"Expected f32 here.",
				5..20,
				DiagnosticLabelPriority::Primary,
			)]
		}]
	);
	report(text, diagnostics)
}

#[derive(Debug, Deserialize, PartialEq)]
struct ExtraFields {
	known: String,

	#[serde(rename = "taml::extra_fields")]
	extra_fields: IndexMap<String, String>,
}

#[test]
fn extra_fields() {
	let text = "known: \"It is known.\"\n\
    unknown: \"It is unknowable.\"\n";
	let mut diagnostics = vec![];
	assert_eq!(
		from_str::<ExtraFields, _>(text, &mut diagnostics),
		Ok(ExtraFields {
			known: "It is known.".to_string(),
			extra_fields: indexmap! {
				"unknown".to_string() => "It is unknowable.".to_string()
			},
		}),
		"diagnostics: {:?}",
		diagnostics
	);
	assert_eq!(diagnostics.as_slice(), &[]);
	report(text, diagnostics);
}

#[derive(Debug, Deserialize, Educe)]
#[educe(PartialEq)]
struct IgnoredExtraFields {
	known: String,

	#[educe(PartialEq(ignore))]
	#[serde(rename = "taml::extra_fields")]
	extra_fields: IgnoredAny,
}

#[test]
fn ignored_extra_fields() {
	let text = "known: \"It is known.\"\n\
    unknown: \"It is unknowable.\"\n";
	let mut diagnostics = vec![];
	assert_eq!(
		from_str::<IgnoredExtraFields, _>(text, &mut diagnostics),
		Ok(IgnoredExtraFields {
			known: "It is known.".to_string(),
			extra_fields: IgnoredAny,
		}),
		"diagnostics: {:?}",
		diagnostics
	);
	assert_eq!(diagnostics.as_slice(), &[]);
	report(text, diagnostics);
}

#[derive(Debug, Deserialize, PartialEq)]
struct MissingFields {
	missing_field: i8,
}

#[test]
#[allow(clippy::reversed_empty_ranges)]
fn missing_fields() {
	let text = "\n";
	let mut diagnostics = vec![];
	from_str::<MissingFields, _>(text, &mut diagnostics).unwrap_err();
	assert_eq!(
		diagnostics.as_slice(),
		&[tamlDiagnostic {
			r#type: DiagnosticType::MissingField,
			labels: vec![DiagnosticLabel::new(
				"Missing field missing_field.",
				0..0,
				DiagnosticLabelPriority::Primary,
			)]
		}]
	);
	report(text, diagnostics);
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
		// Not great, but seems to help a bit with output weirdness.
		let stdout = stdout();
		let _stdout_lock = stdout.lock();
		Emitter::stderr(ColorConfig::Auto, Some(&codemap)).emit(&diagnostics)
	}
}
