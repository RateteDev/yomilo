use wasm_bindgen::prelude::*;
use yomilo_core::json::{self, JsonApiError};

#[wasm_bindgen(js_name = segment)]
pub fn segment(source: &str, cursor_json: &str, input: &str) -> Result<String, JsError> {
    json::segment_json(source, cursor_json, input).map_err(js_error)
}

#[wasm_bindgen(js_name = requiredInputs)]
pub fn required_inputs(segment_json: &str) -> Result<String, JsError> {
    json::required_inputs_json(segment_json).map_err(js_error)
}

#[wasm_bindgen(js_name = prepareSegment)]
pub fn prepare_segment(segment_json: &str, inputs_json: &str) -> Result<String, JsError> {
    json::prepare_segment_json(segment_json, inputs_json).map_err(js_error)
}

#[wasm_bindgen(js_name = interpretUrlCardPage)]
pub fn interpret_url_card_page(html: &str) -> Result<String, JsError> {
    json::interpret_url_card_page_json(html).map_err(js_error)
}

#[wasm_bindgen(js_name = interpretTweetResult)]
pub fn interpret_tweet_result(id: &str, json: &str) -> Result<String, JsError> {
    json::interpret_tweet_result_json(id, json).map_err(js_error)
}

#[wasm_bindgen(js_name = formatSource)]
pub fn format_source(source: &str) -> Result<String, JsError> {
    json::format_source(source).map_err(js_error)
}

#[wasm_bindgen(js_name = checkCanonical)]
pub fn check_canonical(source: &str) -> Result<String, JsError> {
    json::check_canonical_json(source).map_err(js_error)
}

#[wasm_bindgen(js_name = defaultPreparationInputs)]
pub fn default_preparation_inputs() -> Result<String, JsError> {
    json::default_preparation_inputs_json().map_err(js_error)
}

fn js_error(error: JsonApiError) -> JsError {
    JsError::new(
        &serde_json::to_string(&error).expect("JSON API errors contain only serializable data"),
    )
}
