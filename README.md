# actix-xml

[![crates.io](https://img.shields.io/crates/v/actix-xml?style=flat-square)](https://crates.io/crates/actix-xml)
[![Documentation](https://img.shields.io/docsrs/actix-xml?style=flat-square)](https://docs.rs/actix-xml)

XML extractor for actix-web.

This crate provides struct `Xml` that can be used to extract typed information from request's body.

Under the hood, [quick-xml](https://github.com/tafia/quick-xml) is used to parse payloads.

## Example

```rust
use actix_web::{web, App};
use actix_xml::Xml;
use serde::Deserialize;

#[derive(Deserialize)]
struct Info {
    username: String,
}

/// deserialize `Info` from request's body
async fn index(info: Xml<Info>) -> String {
    format!("Welcome {}!", info.username)
}

fn main() {
    let app = App::new().service(
        web::resource("/index.html").route(
            web::post().to(index))
    );
}
```

## Features

- `encoding`: support non utf-8 payload
- `compress`(default): enable actix-web `compress` support

If you've removed all compress feature flag for actix-web, make sure to remove `compress` by setting `default-features=false`,
or a compile error may occur.

## Version Support

- `0.1.x` - supports `actix-web 3.3.x`
- `0.2.0-beta.0` - supports `actix-web 4.0.0.beta.8`

## License

MIT