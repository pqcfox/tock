use super::NoSupport;

pub trait Pattgen: crate::Component + std::fmt::Debug + std::fmt::Display {}

impl Pattgen for NoSupport {}
