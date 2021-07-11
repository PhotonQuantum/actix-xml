use actix_web::error::PayloadError;
use actix_web::http::StatusCode;
use actix_web::{HttpResponse, ResponseError};
use quick_xml::DeError as XMLError;
use thiserror::Error;

/// A set of errors that can occur during parsing xml payloads
#[derive(Debug, Error)]
pub enum XMLPayloadError {
    /// Payload size is bigger than allowed. (default: 32kB)
    #[error("Xml payload size is bigger than allowed")]
    Overflow,
    /// Content type error
    #[error("Content type error")]
    ContentType,
    /// Deserialize error
    #[error("Xml deserialize error: {0}")]
    Deserialize(#[from] XMLError),
    /// Payload error
    #[error("Error that occur during reading payload: {0}")]
    Payload(#[from] PayloadError),
}

impl ResponseError for XMLPayloadError {
    fn error_response(&self) -> actix_web::HttpResponse {
        match *self {
            XMLPayloadError::Overflow => HttpResponse::new(StatusCode::PAYLOAD_TOO_LARGE),
            _ => HttpResponse::new(StatusCode::BAD_REQUEST),
        }
    }
}
