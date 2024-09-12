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
//! Instructs the flash controller to read a boot certificate from the
//! flash info partition. The *entire page* is written to read-write
//! Allow 1, which must have size equal to a flash page. The caller is
//! responsible for determining the certificate length (e.g. by
//! running an x509 parser).
//!
//! Arguments:
//! 1. Which certificate to read
//!   + 0: Creator identity (UDS)
//!   + 1: Owner intermediate (CDI_0)
//!   + 2: Owner identity (CDI_1)
//! 2. Ignored
//!
//! Return value:
//! + CommandReturn::failure(ErrorCode::INVAL): if the certificate type is invalid
//! + CommandReturn::failure(ErrorCode::BUSY): if the flash controller reported a busy status.
//! + CommandReturn::failure(ErrorCode::FAIL): if the flash read operation failed for any other reason.
//! + CommandReturn::success(): the operation has been initiated.
//!
//! Subscribe interface
//! -------------------
//!
//! ### Subscribe 0
//!
//! Register a callback indicating a boot certificate read from the
//! flash info partition is available.
//!
//! Callback arguments:
//!
//! 1. Operation result:
//!     + ErrorCode::FAIL: Operation failed.
//!     + OK: Operation completed successfully.
//! 2. error code (relevant only if `Operation result` is ErrorCode::FAIL): an error
//! describing the failure
//!     + 1: Flash controller error
//!     + 2: Flash memory protection error
//! 3. Always 0
//!

use earlgrey::flash_ctrl::{Bank, FlashCtrl, RawFlashCtrlPage};
use kernel::grant::{AllowRoCount, AllowRwCount, Grant, UpcallCount};
use kernel::hil::flash::{
    Error as FlashError, InfoClient as InfoClientTrait,
    InfoFlash as InfoFlashTrait,
};
use kernel::processbuffer::WriteableProcessBuffer;
use kernel::syscall::{CommandReturn, SyscallDriver};
use kernel::utilities::cells::{OptionalCell, TakeCell};
use kernel::{ErrorCode, ProcessId};

pub const DRIVER_NUM: usize = capsules_core::driver::NUM::OpenTitanAttestation as usize;

/// OpenTitan Attestation
pub struct Attestation<'a> {
    info_flash: &'a FlashCtrl<'a>,
    flash_buf: TakeCell<'static, RawFlashCtrlPage>,
    grant: Grant<(), UpcallCount<{ upcall::COUNT }>, AllowRoCount<{ RO_ALLOW_COUNT }>, AllowRwCount<{ RW_ALLOW_COUNT }>>,
    owning_process: OptionalCell<ProcessId>,
}

impl<'a> Attestation<'a>
{
    /// Creates a new Attestation capsule
    pub fn new(
        info_flash: &'a FlashCtrl<'a>,
        flash_buf: &'static mut RawFlashCtrlPage,
        grant: Grant<(), UpcallCount<{ upcall::COUNT }>, AllowRoCount<{ RO_ALLOW_COUNT }>, AllowRwCount<{ RW_ALLOW_COUNT }>>,
    ) -> Self {
        Self {
            info_flash,
            flash_buf: TakeCell::new(flash_buf),
            grant,
            owning_process: OptionalCell::empty(),
        }
    }

    /// Invoked when a certificate has been read from the flash info
    /// partition (command #1)
    fn schedule_cert_available_upcall(
        &self,
        process_id: ProcessId,
        read_buffer: &'static mut RawFlashCtrlPage,
        status: Result<(), FlashError>,
    ) {
        // Grant errors are ignored, since there is no reasonable way
        // to handle them.
        let _ = self.grant.enter(process_id, |_, kernel_data| {
            // Scheduling errors are ignored, since there is no
            // reasonable way to handle them.
            let args = match status {
                Ok(()) => {
                    // Store the certificate in the allow buffer
                    let mut err = Ok(());
                    let _ = kernel_data
                        .get_readwrite_processbuffer(RwAllowId::BootCertificate.to_usize())
                        .and_then(|allowed_buffer| {
                            allowed_buffer.mut_enter(|data| {
                                if let Err(e) = data.copy_from_slice_or_err(read_buffer.as_mut()) {
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
                    // Return the flash read buffer so we can perform more flash
                    // operations
                    self.flash_buf.put(Some(read_buffer));
                    rtn
                }
                // CAST: `ErrorCode` defines the value conversion
                // explicitly
                Err(FlashError::FlashError) => (ErrorCode::FAIL as usize, 1, 0),
                Err(FlashError::FlashMemoryProtectionError) => (ErrorCode::FAIL as usize, 2, 0),
            };
            let _ =
                kernel_data.schedule_upcall(upcall::UpcallId::BootCertAvailable.to_usize(), args);
        });
    }

    // Flash helper
    fn read_flash_info_page(
        &self,
        partition_type: usize,
        bank: usize,
        page: usize,
    ) -> CommandReturn {
        let partition_type = match partition_type.try_into() {
            Err(()) => return CommandReturn::failure(ErrorCode::FAIL),
            Ok(info_partition_type) => info_partition_type,
        };
        let bank: Bank = match bank.try_into() {
            Err(()) => return CommandReturn::failure(ErrorCode::INVAL),
            Ok(bank) => bank,
        };
        let buffer = match self.flash_buf.take() {
            None => return CommandReturn::failure(ErrorCode::BUSY),
            Some(buffer) => buffer,
        };
        let result = self
            .info_flash
            .read_info_page(partition_type, bank, page, buffer);
        match result {
            Err((err, buf)) => {
                self.flash_buf.put(Some(buf));
                CommandReturn::failure(err)
            }
            Ok(()) => CommandReturn::success(),
        }
    }
}

impl<'a> Attestation<'a>
{
    // Command handlers

    /// Instructs the flash controller to read a boot certificate from
    /// the flash info partition.
    pub fn info_flash_read_boot_cert(
        &self,
        _calling_process: ProcessId,
        cert_type: BootCert,
    ) -> CommandReturn {
        // TODO: the location of the entropy data in flash is
        // top-specific (the values here are for earlgrey). This
        // should be refactored so that the top-specific portion of
        // the flash control driver includes an API to get these
        // values, so this capsule can be top-independent.
        const RAW_PARTITION_TYPE: usize = 0;
        const RAW_BANK: usize = 1;
        let raw_page = match cert_type {
            BootCert::CreatorIdentity => 7,
            BootCert::OwnerIntermediate => 8,
            BootCert::OwnerIdentity => 9,
        };
        self.read_flash_info_page(
            RAW_PARTITION_TYPE,
            RAW_BANK,
            raw_page,
        )
    }
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

/// Read-only allow count
const RO_ALLOW_COUNT: u8 = 0;
/// Read-write allow count
const RW_ALLOW_COUNT: u8 = 1;

/// Read-write buffer identifier
#[repr(usize)]
enum RwAllowId {
    /// A boot stage certificate read from the flash info partition
    BootCertificate = 0,
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
        BootCertAvailable = 0,
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
    InfoFlashReadBootCert,
}

impl TryFrom<usize> for Command {
    type Error = ();

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Command::DriverExistence),
            1 => Ok(Command::InfoFlashReadBootCert),
            _ => Err(()),
        }
    }
}

impl<
        'a,
    > SyscallDriver for Attestation<'a>
{
    fn command(
        &self,
        command_num: usize,
        data1: usize,
        _data2: usize,
        calling_process: ProcessId,
    ) -> CommandReturn {
        let cmd = Command::try_from(command_num);        
        match cmd {
            Ok(Command::DriverExistence) => CommandReturn::success(),
            Ok(Command::InfoFlashReadBootCert) => {
                let cert = match data1.try_into() {
                    Ok(cert) => cert,
                    Err(e) => return CommandReturn::failure(e),
                };
                self.info_flash_read_boot_cert(calling_process, cert)
            }
            Err(()) => CommandReturn::failure(ErrorCode::NOSUPPORT),
        }
    }

    fn allocate_grant(&self, processid: ProcessId) -> Result<(), kernel::process::Error> {
        self.grant.enter(processid, |_, _| {})
    }
}

// Event handling

impl<'a> InfoClientTrait<FlashCtrl<'a>> for Attestation<'a>
{
    fn info_read_complete(
        &self,
        read_buffer: &'static mut RawFlashCtrlPage,
        status: Result<(), FlashError>,
    ) {
        self.owning_process.map(|owner_id| {
            self.schedule_cert_available_upcall(owner_id, read_buffer, status)
        });
    }

    fn info_write_complete(
        &self,
        _write_buffer: &'static mut RawFlashCtrlPage,
        _result: Result<(), FlashError>,
    ) {
        // Should never happen
    }

    fn info_erase_complete(&self, _result: Result<(), FlashError>) {
        // Should never happen
    }
}
