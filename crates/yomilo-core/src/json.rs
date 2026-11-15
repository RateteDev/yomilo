use crate::{
    Cursor, DiagnosticClass, InputState, PreparationError, PreparationInputs, RelatedLocation,
    Segment, SegmentError, SourceSpan, StructureError,
};
use serde::{Deserialize, Serialize};
use std::{error::Error, fmt};

const MAX_SAFE_INTEGER: u64 = (1_u64 << 53) - 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub enum JsonApiErrorKind {
    InvalidInput,
    Structure,
    InvariantViolation,
    Preparation,
    Serialization,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub struct JsonDiagnostic {
    pub class: DiagnosticClass,
    pub code: String,
    pub message: String,
    pub span: Option<SourceSpan>,
    pub related: Vec<RelatedLocation>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub struct JsonInputLocation {
    pub line: usize,
    pub column: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub struct JsonApiError {
    pub kind: JsonApiErrorKind,
    pub message: String,
    pub diagnostics: Vec<JsonDiagnostic>,
    pub preparation: Option<Box<PreparationError>>,
    pub input_location: Option<JsonInputLocation>,
}

impl fmt::Display for JsonApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}
impl Error for JsonApiError {}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub struct CanonicalCheckResult {
    pub canonical: bool,
    pub error: Option<JsonApiError>,
}

pub fn segment_json(source: &str, cursor_json: &str, input: &str) -> Result<String, JsonApiError> {
    let cursor: Option<Cursor> = parse_input(cursor_json)?;
    let input: InputState = serde_json::from_value(serde_json::Value::String(input.to_owned()))
        .map_err(|error| {
            simple_error(
                JsonApiErrorKind::InvalidInput,
                "invalid-input-state",
                error.to_string(),
            )
        })?;
    let batch = crate::segment(source, cursor.as_ref(), input).map_err(|error| match error {
        SegmentError::InvalidCursor { message } => {
            simple_error(JsonApiErrorKind::InvalidInput, "invalid-cursor", message)
        }
        SegmentError::Structure(error) => structure_error(error),
    })?;
    serialize(&batch)
}

pub fn required_inputs_json(segment_json: &str) -> Result<String, JsonApiError> {
    let segment: Segment = parse_input(segment_json)?;
    serialize(&crate::required_inputs(&segment))
}

pub fn prepare_segment_json(segment_json: &str, inputs_json: &str) -> Result<String, JsonApiError> {
    let segment: Segment = parse_input(segment_json)?;
    let inputs: PreparationInputs = parse_input(inputs_json)?;
    let ready = crate::prepare_segment(&segment, &inputs).map_err(preparation_error)?;
    serialize(&ready)
}

fn parse_input<T: Serialize + for<'de> Deserialize<'de>>(json: &str) -> Result<T, JsonApiError> {
    let value: T = serde_json::from_str(json).map_err(|error| JsonApiError {
        input_location: Some(JsonInputLocation {
            line: error.line(),
            column: error.column(),
        }),
        ..simple_error(
            JsonApiErrorKind::InvalidInput,
            "invalid-input-json",
            error.to_string(),
        )
    })?;
    serialize(&value).map_err(|error| {
        simple_error(
            JsonApiErrorKind::InvalidInput,
            "unsafe-json-integer",
            error.message,
        )
    })?;
    Ok(value)
}

pub fn interpret_url_card_page_json(html: &str) -> Result<String, JsonApiError> {
    serialize(&crate::interpret_url_card_page(html))
}

pub fn interpret_tweet_result_json(id: &str, json: &str) -> Result<String, JsonApiError> {
    serialize(&crate::interpret_tweet_result(id, json))
}

pub fn format_source(source: &str) -> Result<String, JsonApiError> {
    crate::structure(source)
        .map(|document| crate::format(&document))
        .map_err(structure_error)
}

pub fn check_canonical_json(source: &str) -> Result<String, JsonApiError> {
    let error = crate::check_canonical(source).err().map(structure_error);
    serialize(&CanonicalCheckResult {
        canonical: error.is_none(),
        error,
    })
}

pub fn default_preparation_inputs_json() -> Result<String, JsonApiError> {
    serialize(&PreparationInputs::default())
}

fn serialize(value: &impl Serialize) -> Result<String, JsonApiError> {
    let value = serde_json::to_value(value).map_err(|error| {
        simple_error(
            JsonApiErrorKind::Serialization,
            "serialization-failed",
            error.to_string(),
        )
    })?;
    if !safe_numbers(&value) {
        return Err(simple_error(
            JsonApiErrorKind::Serialization,
            "unsafe-json-integer",
            "JSON integers must fit JavaScript's exact integer range".to_owned(),
        ));
    }
    serde_json::to_string(&value).map_err(|error| {
        simple_error(
            JsonApiErrorKind::Serialization,
            "serialization-failed",
            error.to_string(),
        )
    })
}

fn safe_numbers(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Number(number) => {
            number.as_u64().is_some_and(|v| v <= MAX_SAFE_INTEGER)
                || number
                    .as_i64()
                    .is_some_and(|v| v.unsigned_abs() <= MAX_SAFE_INTEGER)
        }
        serde_json::Value::Array(values) => values.iter().all(safe_numbers),
        serde_json::Value::Object(values) => values.values().all(safe_numbers),
        _ => true,
    }
}

fn structure_error(error: StructureError) -> JsonApiError {
    let kind = if matches!(error, StructureError::InvariantViolation(_)) {
        JsonApiErrorKind::InvariantViolation
    } else {
        JsonApiErrorKind::Structure
    };
    let diagnostics = error
        .diagnostics()
        .iter()
        .map(|diagnostic| JsonDiagnostic {
            class: diagnostic.class,
            code: serde_json::to_value(diagnostic.code)
                .expect("diagnostic code is a string")
                .as_str()
                .expect("diagnostic code is a string")
                .to_owned(),
            message: diagnostic.message.clone(),
            span: Some(diagnostic.span),
            related: diagnostic.related.clone(),
        })
        .collect();
    JsonApiError {
        kind,
        message: error.to_string(),
        diagnostics,
        preparation: None,
        input_location: None,
    }
}

fn preparation_error(error: PreparationError) -> JsonApiError {
    let (code, span) = match &error {
        PreparationError::VersionMismatch { .. } => ("version-mismatch", None),
        PreparationError::MissingMediaInput { span, .. } => ("missing-media-input", Some(*span)),
        PreparationError::InvalidInput { span, .. } => ("invalid-preparation-input", Some(*span)),
        PreparationError::InvalidDocument { span, .. } => ("invalid-document", Some(*span)),
        PreparationError::MissingSyntax { .. } => ("missing-syntax", None),
    };
    let mut result = simple_error(JsonApiErrorKind::Preparation, code, error.to_string());
    result.diagnostics[0].span = span;
    result.preparation = Some(Box::new(error));
    result
}

fn simple_error(kind: JsonApiErrorKind, code: &str, message: String) -> JsonApiError {
    JsonApiError {
        kind,
        diagnostics: vec![JsonDiagnostic {
            class: DiagnosticClass::Semantic,
            code: code.to_owned(),
            message: message.clone(),
            span: None,
            related: Vec::new(),
        }],
        message,
        preparation: None,
        input_location: None,
    }
}
