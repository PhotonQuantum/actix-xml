use std::sync::Arc;

use actix_web::{web, HttpMessage, HttpRequest};

use crate::error::XMLPayloadError;

/// XML extractor configuration
///
/// # Example
///
/// ```rust
/// use actix_web::{error, web, App, FromRequest, HttpResponse};
/// use actix_xml::{Xml, XmlConfig};
/// use serde::Deserialize;
///
/// #[derive(Deserialize)]
/// struct Info {
///     username: String,
/// }
///
/// /// deserialize `Info` from request's body, max payload size is 4kb
/// async fn index(info: Xml<Info>) -> String {
///     format!("Welcome {}!", info.username)
/// }
///
/// fn main() {
///     let app = App::new().service(
///         web::resource("/index.html")
///             .app_data(
///                 // Json extractor configuration for this resource.
///                 XmlConfig::default()
///                     .limit(4096) // Limit request payload size
///                     .content_type(|mime| {  // <- accept text/plain content type
///                         mime.type_() == mime::TEXT && mime.subtype() == mime::PLAIN
///                     })
///             )
///             .route(web::post().to(index))
///     );
/// }
/// ```
///
#[derive(Clone)]
pub struct XmlConfig {
    pub(crate) limit: usize,
    content_type: Option<Arc<dyn Fn(mime::Mime) -> bool + Send + Sync>>,
}

const DEFAULT_CONFIG: XmlConfig = XmlConfig {
    limit: 262_144,
    content_type: None,
};

impl Default for XmlConfig {
    fn default() -> Self {
        DEFAULT_CONFIG.clone()
    }
}

impl XmlConfig {
    pub fn new() -> Self {
        Default::default()
    }

    /// Change max size of payload. By default max size is 256Kb
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }

    /// Set predicate for allowed content types
    pub fn content_type<F>(mut self, predicate: F) -> Self
    where
        F: Fn(mime::Mime) -> bool + Send + Sync + 'static,
    {
        self.content_type = Some(Arc::new(predicate));
        self
    }

    pub(crate) fn check_content_type(&self, req: &HttpRequest) -> Result<(), XMLPayloadError> {
        // check content-type
        if let Ok(Some(mime)) = req.mime_type() {
            if mime == "text/xml"
                || mime == "application/xml"
                || self
                    .content_type
                    .as_ref()
                    .map_or(false, |predicate| predicate(mime))
            {
                Ok(())
            } else {
                Err(XMLPayloadError::ContentType)
            }
        } else {
            Err(XMLPayloadError::ContentType)
        }
    }

    /// Extract payload config from app data. Check both `T` and `Data<T>`, in that order, and fall
    /// back to the default payload config.
    pub(crate) fn from_req(req: &HttpRequest) -> &Self {
        req.app_data::<Self>()
            .or_else(|| req.app_data::<web::Data<Self>>().map(|d| d.as_ref()))
            .unwrap_or(&DEFAULT_CONFIG)
    }
}
