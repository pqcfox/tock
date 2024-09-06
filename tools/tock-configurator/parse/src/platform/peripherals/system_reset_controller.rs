use super::NoSupport;

pub trait SystemResetController: crate::Component + std::fmt::Debug + std::fmt::Display {}

impl SystemResetController for NoSupport {}
