//! XML extractor for actix-web
//!
//! This crate provides struct `Xml` that can be used to extract typed information from request's body.
//!
//! Under the hood, [quick-xml](https://github.com/tafia/quick-xml) is used to parse payloads.
//!
//! *Minimum supported rust version: 1.46.0*
//!
//! ## Example
//!
//! ```rust
//! use actix_web::{web, App};
//! use actix_xml::Xml;
//! use serde::Deserialize;
//!
//! #[derive(Deserialize)]
//! struct Info {
//!     username: String,
//! }
//!
//! /// deserialize `Info` from request's body
//! async fn index(info: Xml<Info>) -> String {
//!     format!("Welcome {}!", info.username)
//! }
//!
//! fn main() {
//!     let app = App::new().service(
//!         web::resource("/index.html").route(
//!             web::post().to(index))
//!     );
//! }
//! ```
//!
//! ## Features
//!
//! - `encoding`: support non utf-8 payload
//! - `compress`(default): enable actix-web `compress` support
//!
//! If you've removed all compress feature flag for actix-web, make sure to remove `compress` by setting `default-features=false`,
//! or a compile error may occur.

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::{fmt, ops};

use actix_web::dev;
use actix_web::http::header;
use actix_web::web::BytesMut;
use actix_web::Error as ActixError;
use actix_web::{FromRequest, HttpRequest};
use futures::future::{err, Either, LocalBoxFuture, Ready};
use futures::{FutureExt, StreamExt};
use serde::de::DeserializeOwned;

pub use crate::config::XmlConfig;
pub use crate::error::XMLPayloadError;

mod config;
mod error;

#[cfg(test)]
mod tests;

/// Xml extractor
///
/// Xml can be used to extract typed information from request's body.
///
/// [**XmlConfig**](struct.XmlConfig.html) allows to configure extraction
/// process.
///
/// ## Example
///
/// ```rust
/// use actix_web::{web, App};
/// use actix_xml::Xml;
/// use serde::Deserialize;
///
/// #[derive(Deserialize)]
/// struct Info {
///     username: String,
/// }
///
/// /// deserialize `Info` from request's body
/// async fn index(info: Xml<Info>) -> String {
///     format!("Welcome {}!", info.username)
/// }
///
/// fn main() {
///     let app = App::new().service(
///        web::resource("/index.html").route(
///            web::post().to(index))
///     );
/// }
/// ```
pub struct Xml<T>(pub T);

impl<T> Xml<T> {
    /// Deconstruct to an inner value
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> ops::Deref for Xml<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T> ops::DerefMut for Xml<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

impl<T> fmt::Debug for Xml<T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "XML: {:?}", self.0)
    }
}

impl<T> fmt::Display for Xml<T>
where
    T: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl<T> FromRequest for Xml<T>
where
    T: DeserializeOwned + 'static,
{
    type Config = XmlConfig;
    type Error = ActixError;
    #[allow(clippy::type_complexity)]
    type Future =
        Either<LocalBoxFuture<'static, Result<Self, ActixError>>, Ready<Result<Self, ActixError>>>;

    fn from_request(req: &HttpRequest, payload: &mut dev::Payload) -> Self::Future {
        let path = req.path().to_string();
        let config = XmlConfig::from_req(req);

        if let Err(e) = config.check_content_type(req) {
            return Either::Right(err(e.into()));
        }

        Either::Left(
            XmlBody::new(req, payload)
                .limit(config.limit)
                .map(move |res| match res {
                    Err(e) => {
                        log::debug!(
                            "Failed to deserialize XML from payload. \
                         Request path: {}",
                            path
                        );

                        Err(e.into())
                    }
                    Ok(data) => Ok(Xml(data)),
                })
                .boxed_local(),
        )
    }
}

/// Request's payload xml parser, it resolves to a deserialized `T` value.
/// This future could be used with `ServiceRequest` and `ServiceFromRequest`.
///
/// Returns error:
///
/// * content type is not `text/xml` or `application/xml`
///   (unless specified in [`XmlConfig`](struct.XmlConfig.html))
/// * content length is greater than 256k
pub struct XmlBody<U> {
    limit: usize,
    length: Option<usize>,
    #[cfg(feature = "compress")]
    stream: Option<dev::Decompress<dev::Payload>>,
    #[cfg(not(feature = "compress"))]
    stream: Option<dev::Payload>,
    err: Option<XMLPayloadError>,
    fut: Option<LocalBoxFuture<'static, Result<U, XMLPayloadError>>>,
}

impl<U> XmlBody<U>
where
    U: DeserializeOwned + 'static,
{
    /// Create `XmlBody` for request.
    #[allow(clippy::borrow_interior_mutable_const)]
    pub fn new(req: &HttpRequest, payload: &mut dev::Payload) -> Self {
        let len = req
            .headers()
            .get(&header::CONTENT_LENGTH)
            .and_then(|l| l.to_str().ok())
            .and_then(|s| s.parse::<usize>().ok());

        #[cfg(feature = "compress")]
        let payload = dev::Decompress::from_headers(payload.take(), req.headers());
        #[cfg(not(feature = "compress"))]
        let payload = payload.take();

        XmlBody {
            limit: 262_144,
            length: len,
            stream: Some(payload),
            fut: None,
            err: None,
        }
    }

    /// Change max size of payload. By default max size is 256Kb
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }
}

impl<U> Future for XmlBody<U>
where
    U: DeserializeOwned + 'static,
{
    type Output = Result<U, XMLPayloadError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if let Some(ref mut fut) = self.fut {
            return Pin::new(fut).poll(cx);
        }

        if let Some(err) = self.err.take() {
            return Poll::Ready(Err(err));
        }

        let limit = self.limit;
        if let Some(len) = self.length.take() {
            if len > limit {
                return Poll::Ready(Err(XMLPayloadError::Overflow));
            }
        }
        let mut stream = self.stream.take().unwrap();

        self.fut = Some(
            async move {
                let mut body = BytesMut::with_capacity(8192);

                while let Some(item) = stream.next().await {
                    let chunk = item?;
                    if (body.len() + chunk.len()) > limit {
                        return Err(XMLPayloadError::Overflow);
                    } else {
                        body.extend_from_slice(&chunk);
                    }
                }
                Ok(quick_xml::de::from_reader(&*body)?)
            }
            .boxed_local(),
        );

        self.poll(cx)
    }
}
