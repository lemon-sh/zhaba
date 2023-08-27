use std::{net::IpAddr, str::FromStr};

use axum::{
    headers::{Error, Header},
    http::HeaderName,
};

static X_FORWARDED_FOR: HeaderName = HeaderName::from_static("x-forwarded-for");

pub struct XForwardedFor(pub Vec<IpAddr>);

impl Header for XForwardedFor {
    fn name() -> &'static HeaderName {
        &X_FORWARDED_FOR
    }

    fn decode<'i, I>(values: &mut I) -> Result<Self, Error>
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
                chain.push(IpAddr::from_str(ip).map_err(|_| Error::invalid())?);
            }
        }
        Ok(Self(chain))
    }

    fn encode<E: Extend<axum::http::HeaderValue>>(&self, _values: &mut E) {
        unimplemented!()
    }
}
