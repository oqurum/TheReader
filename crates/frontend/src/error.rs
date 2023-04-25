use std::fmt::Debug;

use common::api::ApiErrorResponse;
use wasm_bindgen::JsValue;
use thiserror::Error as ThisError;


pub type Result<V> = std::result::Result<V, Error>;

#[derive(Debug, ThisError)]
pub enum Error {
    #[error("JsValue Error: {0:?}")]
    JsValue(#[from] InnerValue),

    #[error("Json Error: {0}")]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    ApiResponse(#[from] ApiErrorResponse),
}

impl From<JsValue> for Error {
    fn from(value: JsValue) -> Self {
        Self::JsValue(InnerValue(value))
    }
}

impl From<Error> for ApiErrorResponse {
    fn from(val: Error) -> Self {
        match val {
            Error::JsValue(e) => ApiErrorResponse::new(e.to_string()),
            Error::Json(e) => ApiErrorResponse::new(e.to_string()),
            Error::ApiResponse(v) => v,
        }
    }
}


#[derive(Debug)]
pub struct InnerValue(JsValue);

impl std::fmt::Display for InnerValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::error::Error for InnerValue {}