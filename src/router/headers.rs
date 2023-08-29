use std::{net::IpAddr, str::FromStr};

use axum::{
    headers::{Error, Header},
    http::HeaderName,
};
use axum::headers::HeaderValue;

static X_FORWARDED_FOR: HeaderName = HeaderName::from_static("x-forwarded-for");
static REFERER: HeaderName = HeaderName::from_static("referer");

pub struct XForwardedFor(pub Vec<IpAddr>);

impl Header for XForwardedFor {
    fn name() -> &'static HeaderName {
        &X_FORWARDED_FOR
    }

    fn decode<'i, I>(values: &mut I) -> Result<Self, Error>
    where
        Self: Sized,
        I: Iterator<Item = &'i HeaderValue>,
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

    fn encode<E: Extend<HeaderValue>>(&self, _values: &mut E) {
        unimplemented!()
    }
}

pub struct Referer(pub String);

impl Header for Referer {
    fn name() -> &'static HeaderName {
        &REFERER
    }

    fn decode<'i, I>(values: &mut I) -> Result<Self, Error> where Self: Sized, I: Iterator<Item=&'i HeaderValue> {
        let uri = values.next().unwrap().to_str().map_err(|_| Error::invalid())?;
        Ok(Self(uri.to_string()))
    }

    fn encode<E: Extend<HeaderValue>>(&self, _values: &mut E) {
        unimplemented!()
    }
}
