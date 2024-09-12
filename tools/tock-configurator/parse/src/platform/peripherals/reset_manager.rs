use super::NoSupport;

pub trait ResetManager: crate::Component + std::fmt::Debug + std::fmt::Display {}

impl ResetManager for NoSupport {}
