use super::NoSupport;

pub trait Aes: crate::Component + std::fmt::Debug + std::fmt::Display {}

impl Aes for NoSupport {}
