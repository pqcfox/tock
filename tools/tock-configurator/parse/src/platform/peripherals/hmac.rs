use super::NoSupport;

pub trait Hmac: crate::Component + std::fmt::Debug + std::fmt::Display {}

impl Hmac for NoSupport {}
