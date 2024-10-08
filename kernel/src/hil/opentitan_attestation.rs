// Copyright zeroRISC Inc.
// Confidential information of zeroRISC Inc. All rights reserved.

// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

use crate::hil::flash::Error as FlashError;
use crate::{ErrorCode, ProcessId};

/// Implemented by drivers that can read an attestation certificate
pub trait CertificateReader<'a> {
    /// Read a certificate stored in hardware. Which driver is used to
    /// retrieve the certificate is implementation-dependent.
    ///
    /// # Returns
    ///
    /// + `Ok(Some(..))` if the certificate could be read synchronously, and an upcall will
    ///   be invoked on the client when the read is complete.
    /// + `Ok(None)` if the certificate is being read asynchronously, and no upcall will
    ///   occur.
    /// + `Err(..)` if an error occurred.
    fn read_certificate(
        &self,
        calling_process: ProcessId,
        certificate: Certificate,
    ) -> Result<Option<&[u8]>, ErrorCode>;

    /// Set the client to handle events that a certificate was read.
    fn set_client(&self, client: &'a dyn CertificateReaderClient);
}

/// Implemented by types that handle events fired by the a `CertificateReader`
pub trait CertificateReaderClient {
    /// Indicates a certificate is available. On success, the
    /// certificate is guaranteed to at least contain the entire
    /// certificate, but may contain additional bytes afterwards.
    fn certificate_available(
        &self,
        owning_process: ProcessId,
        result: Result<&[u8], CertificateReadError>,
    );
}

/// Attestation certificate identifier
pub enum Certificate {
    /// Boot certificate
    Boot(BootCert),
    /// Application certificate, identified by the process ID (`ProcessId::identifier`)
    Application(usize),
}

/// A boot certificate type
#[repr(usize)]
pub enum BootCert {
    /// The creator identity certificate (UDS)
    CreatorIdentity = 0,
    /// The owner intermediate certificate (CDI_0)
    OwnerIntermediate = 1,
    /// The owner identity certificate (CDI_1)
    OwnerIdentity = 2,
}

impl TryFrom<usize> for BootCert {
    type Error = ErrorCode;
    fn try_from(n: usize) -> Result<BootCert, ErrorCode> {
        match n {
            0 => Ok(BootCert::CreatorIdentity),
            1 => Ok(BootCert::OwnerIntermediate),
            2 => Ok(BootCert::OwnerIdentity),
            _ => Err(ErrorCode::INVAL),
        }
    }
}

pub enum CertificateReadError {
    Flash(FlashError),
}
