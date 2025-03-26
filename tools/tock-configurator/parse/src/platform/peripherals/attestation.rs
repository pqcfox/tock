// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

use crate::Flash;

pub trait Attestation: crate::Component + std::fmt::Debug + std::fmt::Display {
    type CertificateReader: CertificateReader;
}

pub trait CertificateReader: crate::Component + std::fmt::Display {}

impl<F: Flash + 'static> CertificateReader for F {}

impl Attestation for super::NoSupport {
    type CertificateReader = super::NoSupport;
}
