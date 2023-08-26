use std::{net::SocketAddr, str::FromStr};

use axum::{
    headers::{Error, Header},
    http::HeaderName,
};

static X_FORWARDED_FOR: HeaderName = HeaderName::from_static("x-forwarded-for");

struct XForwardedFor(Vec<SocketAddr>);

impl Header for XForwardedFor {
    fn name() -> &'static axum::http::HeaderName {
        &X_FORWARDED_FOR
    }

    fn decode<'i, I>(values: &mut I) -> Result<Self, axum::headers::Error>
    where
        Self: Sized,
        I: Iterator<Item = &'i axum::http::HeaderValue>,
    {
        let mut chain = Vec::new();
        for value in values {
            let ips = value
                .to_str()
                .map_err(|_| Error::invalid())?
                .split(',')
                .map(str::trim);
            for ip in ips {
                chain.push(SocketAddr::from_str(ip).map_err(|_| Error::invalid())?);
            }
        }
        Ok(Self(chain))
    }

    fn encode<E: Extend<axum::http::HeaderValue>>(&self, _values: &mut E) {
        unimplemented!()
    }
}
