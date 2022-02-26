use actix_web::http::header;
use actix_web::test::TestRequest;
use actix_web::web::Bytes;
use actix_web::{web, FromRequest};
use serde::Deserialize;

use crate::error::XMLPayloadError;
use crate::{Xml, XmlBody, XmlConfig};

#[derive(Deserialize, Eq, PartialEq, Debug)]
struct MyObject {
    name: String,
}

fn xml_eq(err: XMLPayloadError, other: XMLPayloadError) -> bool {
    match err {
        XMLPayloadError::Overflow => matches!(other, XMLPayloadError::Overflow),
        XMLPayloadError::ContentType => {
            matches!(other, XMLPayloadError::ContentType)
        }
        _ => false,
    }
}

#[actix_rt::test]
async fn test_extract() {
    let (req, mut pl) = TestRequest::default()
        .insert_header((
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("application/xml"),
        ))
        .insert_header((
            header::CONTENT_LENGTH,
            header::HeaderValue::from_static("25"),
        ))
        .set_payload(Bytes::from_static(b"<MyObject name=\"test\" />"))
        .to_http_parts();

    let s = Xml::<MyObject>::from_request(&req, &mut pl).await.unwrap();
    assert_eq!(s.name, "test");
    assert_eq!(
        s.into_inner(),
        MyObject {
            name: "test".to_string()
        }
    );

    let (req, mut pl) = TestRequest::default()
        .insert_header((
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("application/xml"),
        ))
        .insert_header((
            header::CONTENT_LENGTH,
            header::HeaderValue::from_static("25"),
        ))
        .set_payload(Bytes::from_static(b"<MyObject name=\"test\" />"))
        .app_data(XmlConfig::default().limit(10))
        .to_http_parts();

    let s = Xml::<MyObject>::from_request(&req, &mut pl).await;
    assert!(format!("{}", s.err().unwrap()).contains("Xml payload size is bigger than allowed"));
}

#[actix_rt::test]
async fn test_xml_body() {
    let (req, mut pl) = TestRequest::default()
        .insert_header((
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("application/xml"),
        ))
        .insert_header((
            header::CONTENT_LENGTH,
            header::HeaderValue::from_static("10000"),
        ))
        .to_http_parts();

    let xml = XmlBody::<MyObject>::new(&req, &mut pl).limit(100).await;
    assert!(xml_eq(xml.err().unwrap(), XMLPayloadError::Overflow));

    let (req, mut pl) = TestRequest::default()
        .insert_header((
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("application/xml"),
        ))
        .insert_header((
            header::CONTENT_LENGTH,
            header::HeaderValue::from_static("25"),
        ))
        .set_payload(Bytes::from_static(b"<MyObject name=\"test\" />"))
        .to_http_parts();

    let xml = XmlBody::<MyObject>::new(&req, &mut pl).await;
    assert_eq!(
        xml.ok().unwrap(),
        MyObject {
            name: "test".to_owned()
        }
    );
}

#[actix_rt::test]
async fn test_with_xml_and_bad_content_type() {
    let (req, mut pl) = TestRequest::default()
        .insert_header((
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("text/plain"),
        ))
        .insert_header((
            header::CONTENT_LENGTH,
            header::HeaderValue::from_static("25"),
        ))
        .set_payload(Bytes::from_static(b"<MyObject name=\"test\" />"))
        .app_data(XmlConfig::default().limit(4096))
        .to_http_parts();

    let s = Xml::<MyObject>::from_request(&req, &mut pl).await;
    assert!(s.is_err())
}

#[actix_rt::test]
async fn test_with_xml_and_good_custom_content_type() {
    let (req, mut pl) = TestRequest::default()
        .insert_header((
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("text/plain"),
        ))
        .insert_header((
            header::CONTENT_LENGTH,
            header::HeaderValue::from_static("25"),
        ))
        .set_payload(Bytes::from_static(b"<MyObject name=\"test\" />"))
        .app_data(XmlConfig::default().content_type(|mime: mime::Mime| {
            mime.type_() == mime::TEXT && mime.subtype() == mime::PLAIN
        }))
        .to_http_parts();

    let s = Xml::<MyObject>::from_request(&req, &mut pl).await;
    assert!(s.is_ok())
}

#[actix_rt::test]
async fn test_with_xml_and_bad_custom_content_type() {
    let (req, mut pl) = TestRequest::default()
        .insert_header((
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("text/html"),
        ))
        .insert_header((
            header::CONTENT_LENGTH,
            header::HeaderValue::from_static("25"),
        ))
        .set_payload(Bytes::from_static(b"<MyObject name=\"test\" />"))
        .app_data(XmlConfig::default().content_type(|mime: mime::Mime| {
            mime.type_() == mime::TEXT && mime.subtype() == mime::PLAIN
        }))
        .to_http_parts();

    let s = Xml::<MyObject>::from_request(&req, &mut pl).await;
    assert!(s.is_err())
}

#[actix_rt::test]
async fn test_with_config_in_data_wrapper() {
    let (req, mut pl) = TestRequest::default()
        .insert_header((
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("application/xml"),
        ))
        .insert_header((
            header::CONTENT_LENGTH,
            header::HeaderValue::from_static("25"),
        ))
        .set_payload(Bytes::from_static(b"<MyObject name=\"test\" />"))
        .app_data(web::Data::new(XmlConfig::default().limit(10)))
        .to_http_parts();

    let s = Xml::<MyObject>::from_request(&req, &mut pl).await;
    assert!(s.is_err());

    let err_str = s.err().unwrap().to_string();
    assert!(err_str.contains("Xml payload size is bigger than allowed"));
}
