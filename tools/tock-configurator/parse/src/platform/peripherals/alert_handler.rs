use super::NoSupport;

pub trait AlertHandler: crate::Component + std::fmt::Debug + std::fmt::Display {}

impl AlertHandler for NoSupport {}
