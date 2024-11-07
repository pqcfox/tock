// Copyright zeroRISC Inc.
// Confidential information of zeroRISC Inc. All rights reserved.

// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

//! OpenTitan attestation orchestrator. This capsule provides a thin
//! abstraction that enables userspace apps to orchestrate attestation
//! flows without gaining access to device secrets.
//!
//! Command interface
//! -----------------
//!
//! ### Command number 0
//!
//! Check the existence of the driver on the platform.
//!
//! Arguments: none
//!
//! Return value: always CommandReturn::success()
//!
//! ### Command number 1
//!
//! Read an attestation certificate from the hardware. A data chunk
//! equal to the size of an *entire flash page* is written to
//! read-write Allow 0, which must have size equal to a flash
//! page. The caller is responsible for determining the certificate
//! length (e.g. by running an x509 parser).
//!
//! Arguments:
//! 1. The certificate type to read
//!   + 0: Boot certificate
//!   + 1: Application certificate
//! 2. The certificate to read
//!   + If `argument_1 == 0`:
//!     + 0: Creator identity (UDS)
//!     + 1: Owner intermediate (CDI_0)
//!     + 2: Owner identity (CDI_1)
//!   + If `argument_1 == 1`: Process ID of the process whose certificate to read
//!
//! Return value:
//! + CommandReturn::failure(ErrorCode::INVAL): if the certificate type is invalid
//! + CommandReturn::failure(ErrorCode::BUSY): if the flash controller reported a busy status.
//! + CommandReturn::failure(ErrorCode::FAIL): if the flash read operation failed for any other reason.
//! + CommandReturn::success(0): the operation completed with a synchronous read.
//! + CommandReturn::success(1): the operation is performing an asynchronous read.
//!
//! Subscribe interface
//! -------------------
//!
//! ### Subscribe 0
//!
//! Register a callback indicating a certificate (read via command #1)
//! is available.
//!
//! Callback arguments:
//!
//! 1. Operation result:
//!     + ErrorCode::FAIL: Operation failed.
//!     + OK: Operation completed successfully.
//! 2. The driver that reported an error:
//!     + 0: No error (always 0 if argument 1 == OK)
//!     + 1: Flash controller
//! 3. Error code (always 0 if argument 1 == OK)
//!     + If argument 2 == 1:
//!         + 1: Flash controller error
//!         + 2: Flash memory protection error
//!

use kernel::grant::{AllowRoCount, AllowRwCount, Grant, UpcallCount};
use kernel::hil::flash::Error as FlashError;
use kernel::hil::opentitan_attestation::{
    Certificate, CertificateReadError, CertificateReader, CertificateReaderClient,
};
use kernel::processbuffer::WriteableProcessBuffer;
use kernel::syscall::{CommandReturn, SyscallDriver};
use kernel::{ErrorCode, ProcessId};

/// Driver number
pub const DRIVER_NUM: usize = capsules_core::driver::NUM::OpenTitanAttestation as usize;

/// Boot certificate type identifier
const CERT_TYPE_BOOT: usize = 1;

/// Application certificate type identifier
const CERT_TYPE_APP: usize = 2;

/// OpenTitan Attestation
pub struct Attestation<'a, CertReader> {
    cert_reader: &'a CertReader,
    grant: Grant<
        (),
        UpcallCount<{ upcall::COUNT }>,
        AllowRoCount<{ RO_ALLOW_COUNT }>,
        AllowRwCount<{ RW_ALLOW_COUNT }>,
    >,
}

impl<'a, CertReader> Attestation<'a, CertReader> {
    /// Creates a new Attestation capsule
    pub fn new(
        cert_reader: &'a CertReader,
        grant: Grant<
            (),
            UpcallCount<{ upcall::COUNT }>,
            AllowRoCount<{ RO_ALLOW_COUNT }>,
            AllowRwCount<{ RW_ALLOW_COUNT }>,
        >,
    ) -> Self {
        Self { cert_reader, grant }
    }

    /// Invoked when a certificate has been read from the flash info
    /// partition (command #1)
    fn schedule_cert_available_upcall(
        &self,
        process_id: ProcessId,
        result: Result<&[u8], CertificateReadError>,
    ) {
        // Grant errors are ignored, since there is no reasonable way
        // to handle them.
        let _ = self.grant.enter(process_id, |_, kernel_data| {
            // Scheduling errors are ignored, since there is no
            // reasonable way to handle them.
            let args = match result {
                Ok(read_buffer) => {
                    // Store the certificate in the allow buffer
                    let mut err = Ok(());
                    let _ = kernel_data
                        .get_readwrite_processbuffer(RwAllowId::Certificate.to_usize())
                        .and_then(|allowed_buffer| {
                            allowed_buffer.mut_enter(|data| {
                                if let Err(e) = data.copy_from_slice_or_err(read_buffer) {
                                    err = Err(e);
                                }
                            })
                        });
                    let rtn = match err {
                        Ok(()) => (0, 0, 0),
                        // CAST: `ErrorCode` defines the value conversion
                        // explicitly
                        Err(e) => (e as usize, 0, 0),
                    };
                    rtn
                }
                // CAST: `ErrorCode` defines the value conversion
                // explicitly
                Err(CertificateReadError::Flash(FlashError::FlashError)) => {
                    (ErrorCode::FAIL as usize, 1, 1)
                }
                Err(CertificateReadError::Flash(FlashError::FlashMemoryProtectionError)) => {
                    (ErrorCode::FAIL as usize, 1, 2)
                }
            };
            // Scheduling errors are ignored, since there is no
            // reasonable way to handle them.
            let _ = kernel_data.schedule_upcall(upcall::UpcallId::CertAvailable.to_usize(), args);
        });
    }
}

impl<'a, CertReader: CertificateReader<'a>> Attestation<'a, CertReader> {
    /// Read the requested certificate from hardware. Which HWIP block
    /// is invoked to perform the read is top-specific, and possibly
    /// specific to whether this is a UDS, CDI_0/1, or application
    /// certificate.
    pub fn read_certificate(
        &self,
        calling_process: ProcessId,
        cert_type: Certificate,
    ) -> Result<(), ErrorCode> {
        self.cert_reader
            .read_certificate(calling_process, cert_type)
            .map(|opt| {
                // Check if the read was synchronous (Some(..)) or asyncrhonous (None).
                if let Some(read_buffer) = opt {
                    // We got a synchronous certificate read. Store the result
                    // in the grant space.
                    let mut err = Ok(());
                    // Grant errors are ignored, since there is no reasonable way
                    // to handle them.
                    let _ = self.grant.enter(calling_process, |_, kernel_data| {
                        // Store the certificate in the allow buffer
                        let _ = kernel_data
                            .get_readwrite_processbuffer(RwAllowId::Certificate.to_usize())
                            .and_then(|allowed_buffer| {
                                allowed_buffer.mut_enter(|data| {
                                    if let Err(e) = data.copy_from_slice_or_err(read_buffer) {
                                        err = Err(e);
                                    }
                                })
                            });
                    });
                    err?
                }
                Ok(())
            })?
    }
}

/// Read-only allow count
const RO_ALLOW_COUNT: u8 = 0;
/// Read-write allow count
const RW_ALLOW_COUNT: u8 = 1;

/// Read-write buffer identifier
#[repr(usize)]
enum RwAllowId {
    /// A boot stage certificate read from the flash info partition
    Certificate = 0,
}

impl RwAllowId {
    /// Convert the ID to usize
    const fn to_usize(self) -> usize {
        // CAST: the enum defines the conversion explicitly
        self as usize
    }
}

mod upcall {
    pub const COUNT: u8 = 1;

    /// Upcall identifiers. Matches the subscribe numbers in the API
    /// definition at the top of this file.
    #[repr(usize)]
    pub enum UpcallId {
        CertAvailable = 0,
    }

    impl UpcallId {
        /// Convert the ID to usize
        pub const fn to_usize(self) -> usize {
            // CAST: the enum defines the conversion explicitly
            self as usize
        }
    }
}

/// Attestation commands, as described in the API definition at the
/// top of this file.
enum Command {
    DriverExistence,
    ReadCertificate,
}

impl TryFrom<usize> for Command {
    type Error = ();

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Command::DriverExistence),
            1 => Ok(Command::ReadCertificate),
            _ => Err(()),
        }
    }
}

impl<'a, CertReader: CertificateReader<'a>> SyscallDriver for Attestation<'a, CertReader> {
    fn command(
        &self,
        command_num: usize,
        data1: usize,
        data2: usize,
        calling_process: ProcessId,
    ) -> CommandReturn {
        let cmd = Command::try_from(command_num);
        CommandReturn::from(match cmd {
            Ok(Command::DriverExistence) => Ok(()),
            Ok(Command::ReadCertificate) => {
                let cert = match data1 {
                    CERT_TYPE_BOOT => Certificate::Boot(match data2.try_into() {
                        Ok(c) => c,
                        Err(e) => return CommandReturn::failure(e),
                    }),
                    CERT_TYPE_APP => Certificate::Application(data2),
                    _ => return CommandReturn::failure(ErrorCode::INVAL),
                };
                self.read_certificate(calling_process, cert)
            }
            Err(()) => Err(ErrorCode::NOSUPPORT),
        })
    }

    fn allocate_grant(&self, processid: ProcessId) -> Result<(), kernel::process::Error> {
        self.grant.enter(processid, |_, _| {})
    }
}

impl<'a, CertReader> CertificateReaderClient for Attestation<'a, CertReader> {
    /// Indicates a certificate is available. On success, the
    /// certificate is guaranteed to at least contain the entire
    /// certificate, but may contain additional bytes afterwards.
    fn certificate_available(
        &self,
        owning_process: ProcessId,
        result: Result<&[u8], CertificateReadError>,
    ) {
        self.schedule_cert_available_upcall(owning_process, result)
    }
}
