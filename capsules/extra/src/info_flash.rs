// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.
//
// This license header is required for submitting to upstream Tock.
// It is up to ZeroRISC to decide if this header should be here or not.

//! This capsule provides userspace access to info partitions present on some boards such as
//! OpenTitan.
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
//! Read an info page.
//!
//! Arguments:
//!
//! 1. A value with two fields:
//!     + bits 0..15: the type of info page to be read
//!     + bits 16..31: the bank to read from a page
//! 2. The index of the page to be read relative to the bank
//!
//! Return value:
//!
//! + CommandReturn::failure(ErrorCode::INVAL): either the type of the info partition, bank or page
//! index is invalid.
//! + CommandReturn::failure(ErrorCode::BUSY): either the capsule or the peripheral is busy
//! + CommandReturn::success(): the read operation has been initiated
//!
//!
//! ### Command number 2
//!
//! Write an info page.
//!
//! Arguments:
//!
//! 1. A value with two fields:
//!     + bits 0..15: the type of info page to be written
//!     + bits 16..31: the bank to write a page to
//! 2. The index of the page to be written relative to the bank
//!
//! Return value:
//!
//! + CommandReturn::failure(ErrorCode::INVAL): either the type of the info partition, bank or page
//! index is invalid.
//! + CommandReturn::failure(ErrorCode::SIZE): the allowed buffer's length is not equal to the size
//! of a page
//! + CommandReturn::failure(ErrorCode::BUSY): either the capsule or the peripheral is busy
//! + CommandReturn::success(): the write operation has been initiated
//!
//!
//! ### Command number 3
//!
//! Erase an info page.
//!
//! Arguments:
//!
//! 1. A value with two fields:
//!     + bits 0..15: the type of the info page to be erased
//!     + bits 16..31: the bank to erase a page from
//! 2. The index of the page to be erased relative to the bank
//!
//! Return value:
//!
//! + CommandReturn::failure(ErrorCode::INVAL): either the type of the info partition, bank or page
//! index is invalid.
//! + CommandReturn::failure(ErrorCode::BUSY): either the capsule or the peripheral is busy
//! + CommandReturn::success(): the erase operation has been initiated
//!
//!
//! Subscribe interface
//! -------------------
//!
//! ### Subscribe 0
//!
//! Register a read callback.
//!
//! Callback arguments:
//!
//! 1. Read result:
//!     + ErrorCode::FAIL: read failed. The read-write buffer is left untouched.
//!     + ErrorCode::INVAL: the allowed read-write buffer's length differs from the length of an
//!     info page. The read-write buffer is left untouched.
//!     + OK: The allowed read-write contains the content of the page.
//! 2. error code (relevant only if `Read result` is ErrorCode::FAIL): an error describing the
//!    reason the flash operation failed:
//!     + kernel::hil::flash::Error::FlashError: internal error. This probably means a bug in
//!     the peripheral.
//!     + kernel::hil::flash::Error::FlashMemoryProtectionError: the process does not have read
//!     access to the desired page.
//! 3. Always 0
//!
//! ### Subscribe 1
//!
//! Register a write callback.
//!
//! Callback arguments:
//!
//! 1. Write result:
//!     + ErrorCode::FAIL: write failed.
//!     + OK: the page has been written successfully.
//! 2. error code (relevant only if `Write result` is ErrorCode::FAIL): an error describing the
//!    reason the flash operation failed:
//!     + kernel::hil::flash::Error::FlashError: internal error. This probably means a bug in the
//!     peripheral.
//!     + kernel::hil::flash::Error::FlashMemoryProtectionError: the process does not have write
//!     access to the desired page.
//! 3. Always 0
//!
//! ### Subscribe 2
//!
//! Register an erase callback.
//!
//! Callback arguments:
//!
//! 1. Erase result:
//!     + ErrorCode::FAIL: erase failed.
//!     + OK: the page has been erased successfully.
//! 2. error code (relevant only if `Erase result` is ErrorCode::FAIL): an error describing the
//!    erason the flash operation failed:
//!     + kernel::hil::flash::Error::FlashError: internal error. This probably means a bug in the
//!     peripheral.
//!     + kernel::hil::flash::Error::FlashMemoryProtectionError: the process does not have erase
//!     access to the desired page.
//! 3. Always 0

use kernel::grant::{AllowRoCount, AllowRwCount, Grant, UpcallCount};
use kernel::hil::flash::Error;
use kernel::hil::flash::InfoClient as InfoClientTrait;
use kernel::hil::flash::InfoFlash as InfoFlashTrait;
use kernel::processbuffer::{ReadableProcessBuffer, WriteableProcessBuffer};
use kernel::syscall::{CommandReturn, SyscallDriver};
use kernel::utilities::cells::{OptionalCell, TakeCell};
use kernel::{ErrorCode, ProcessId};

/// Driver number
pub const DRIVER_NUMBER: usize = capsules_core::driver::NUM::InfoFlash as usize;

#[derive(Default)]
pub struct AppData;

/// An identifier for read-write buffers
#[repr(usize)]
enum RwAllowId {
    Read = 0,
}

impl RwAllowId {
    /// Convert the ID to usize
    const fn to_usize(self) -> usize {
        // CAST: the cast is safe since the enum is marked as usize
        self as usize
    }
}

/// An identifier for read-only buffers
#[repr(usize)]
enum RoAllowId {
    Write = 0,
}

impl RoAllowId {
    /// Convert the ID to usize
    const fn to_usize(self) -> usize {
        // CAST: the case is safe since the enum is marked as usize
        self as usize
    }
}

/// An identifier for upcalls
#[repr(usize)]
enum UpcallId {
    ReadDone = 0,
    WriteDone = 1,
    EraseDone = 2,
}

impl UpcallId {
    /// Convert the ID to usize
    const fn to_usize(self) -> usize {
        // CAST: the cast is safe since the enum is marked as usize
        self as usize
    }
}

/// Command list
enum Command {
    DriverExistence,
    ReadPage,
    WritePage,
    ErasePage,
}

impl TryFrom<usize> for Command {
    type Error = ();

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Command::DriverExistence),
            1 => Ok(Command::ReadPage),
            2 => Ok(Command::WritePage),
            3 => Ok(Command::ErasePage),
            _ => Err(()),
        }
    }
}

/// Number of upcalls used by the capsule
const UPCALL_ID_COUNT: u8 = 3;
/// Number of read-only allows used by the capsule
const RO_ALLOW_COUNT: u8 = 1;
/// Number of read-write allows used by the capsule
const RW_ALLOW_COUNT: u8 = 1;

/// Capsule providing userspace access to info partitions
pub struct InfoFlash<'a, Flash: InfoFlashTrait> {
    flash: &'a Flash,
    // Per-app state
    grant: Grant<
        AppData,
        UpcallCount<UPCALL_ID_COUNT>,
        AllowRoCount<RO_ALLOW_COUNT>,
        AllowRwCount<RW_ALLOW_COUNT>,
    >,
    buffer: TakeCell<'a, Flash::Page>,
    current_process: OptionalCell<ProcessId>,
}

impl<'a, Flash: InfoFlashTrait> InfoFlash<'a, Flash> {
    /// [InfoFlash] constructor
    ///
    /// # Parameters
    ///
    /// + `flash`: the underlying flash peripheral
    /// + `grant`: grant used to store process data
    /// + `buffer`: a buffer used internally to store a read page before copying its content
    /// to the userpace
    pub fn new(
        flash: &'a Flash,
        grant: Grant<
            AppData,
            UpcallCount<UPCALL_ID_COUNT>,
            AllowRoCount<RO_ALLOW_COUNT>,
            AllowRwCount<RW_ALLOW_COUNT>,
        >,
        buffer: &'a mut Flash::Page,
    ) -> Self {
        Self {
            flash,
            grant,
            buffer: TakeCell::new(buffer),
            current_process: OptionalCell::empty(),
        }
    }

    /// Set the buffer
    ///
    /// # Parameters
    ///
    /// + `buffer`: the buffer to be used internally for future operations by the capsule
    fn set_buffer(&self, buffer: &'a mut Flash::Page) {
        self.buffer.put(Some(buffer))
    }

    /// Take the current buffer
    ///
    /// # Return value
    ///
    /// + Some(buffer): the current available buffer
    /// + None: no available buffer
    fn take_buffer(&self) -> Option<&'a mut Flash::Page> {
        self.buffer.take()
    }

    /// Convert the first argument of a command syscall to the raw info partition type
    ///
    /// # Parameters
    ///
    /// + `argument1`: the first argument of the command syscall
    ///
    /// # Return value
    ///
    /// The passed raw info partition type.
    fn convert_command_argument_to_raw_info_partition_type(argument1: usize) -> usize {
        // The info partition type is represented by the first 16 bits of the argument
        (argument1 as u16) as usize
    }

    /// Convert the first argument of a command syscall to the raw bank
    ///
    /// # Parameters
    ///
    /// + `argument1`: the first argument of the command syscall
    ///
    /// # Return value
    ///
    /// The passed raw bank.
    fn convert_command_argument_to_raw_bank(argument1: usize) -> usize {
        // The info partition type is represented by the last 16 bits of the argument
        argument1 >> 16
    }
}

impl<Flash: InfoFlashTrait> InfoFlash<'static, Flash> {
    /// Try to read an info page.
    ///
    /// # Parameters
    ///
    /// + `raw_info_partition_type`: an integral value representing the type of the info partition
    /// to be read
    /// + `raw_bank`: an integral value representing the bank to read from
    /// + `raw_page_number`: the index of the page to be read relative to the bank
    /// + `process_id`: the ID of the process that's trying to read a page
    ///
    /// # Return value
    ///
    /// + CommandReturn::failure(ErrorCode::INVAL): `raw_info_partition_type`, `raw_bank` or
    /// `raw_page_number` are invalid.
    /// + CommandReturn::failure(ErrorCode::BUSY): the capsule or the peripheral are busy
    /// + CommandReturn::success(): the reading operation has successfully started
    fn raw_read_info_page(
        &self,
        raw_info_partition_type: usize,
        raw_bank: usize,
        raw_page_number: usize,
        process_id: ProcessId,
    ) -> CommandReturn {
        let info_partition_type: Flash::InfoType = match raw_info_partition_type.try_into() {
            Err(()) => return CommandReturn::failure(ErrorCode::INVAL),
            Ok(info_partition_type) => info_partition_type,
        };

        let bank: Flash::BankType = match raw_bank.try_into() {
            Err(()) => return CommandReturn::failure(ErrorCode::INVAL),
            Ok(bank) => bank,
        };

        let buffer = match self.take_buffer() {
            None => return CommandReturn::failure(ErrorCode::BUSY),
            Some(buffer) => buffer,
        };

        match self
            .flash
            .read_info_page(info_partition_type, bank, raw_page_number, buffer)
        {
            Err((error_code, buffer)) => {
                self.set_buffer(buffer);
                CommandReturn::failure(error_code)
            }
            Ok(()) => {
                self.current_process.set(process_id);
                CommandReturn::success()
            }
        }
    }

    /// Try to write an info page.
    ///
    /// # Parameters
    ///
    /// + `raw_info_partition_type`: an integral value representing the type of the info partition
    /// to be written
    /// + `raw_bank`: an integral value representing the bank to write to
    /// + `raw_page_number`: the index of the page to be written relative to the bank
    /// + `process_id`: the ID of the process that's trying to read a page
    ///
    /// # Return value
    ///
    /// + CommandReturn::failure(ErrorCode::INVAL): `raw_info_partition_type`, `raw_bank` or
    /// `raw_page_number` are invalid.
    /// + CommandReturn::failure(ErrorCode::BUSY): the capsule or the peripheral are busy
    /// + Commandreturn::failure(ErrorCode::SIZE): the allowed read-only buffer's length differs
    /// from the length of a page
    /// + CommandReturn::success(): the write operation has successfully started
    fn raw_write_info_page(
        &self,
        raw_info_partition_type: usize,
        raw_bank: usize,
        raw_page_number: usize,
        process_id: ProcessId,
    ) -> CommandReturn {
        let info_partition_type: Flash::InfoType = match raw_info_partition_type.try_into() {
            Err(()) => return CommandReturn::failure(ErrorCode::INVAL),
            Ok(info_partition_type) => info_partition_type,
        };

        let bank: Flash::BankType = match raw_bank.try_into() {
            Err(()) => return CommandReturn::failure(ErrorCode::INVAL),
            Ok(bank) => bank,
        };

        let buffer = match self.take_buffer() {
            None => return CommandReturn::failure(ErrorCode::BUSY),
            Some(buffer) => buffer,
        };

        let copy_result = self
            .grant
            .enter(process_id, |_, kernel_data| {
                kernel_data
                    .get_readonly_processbuffer(RoAllowId::Write.to_usize())
                    .and_then(|allowed_buffer| {
                        allowed_buffer.enter(|data| {
                            let write_slice = buffer.as_mut();

                            data.copy_to_slice_or_err(write_slice)
                        })
                    })
            })
            .ok();

        if let Some(status) = copy_result {
            if status.is_err() {
                return CommandReturn::failure(ErrorCode::SIZE);
            }
        }

        match self
            .flash
            .write_info_page(info_partition_type, bank, raw_page_number, buffer)
        {
            Err((error_code, buffer)) => {
                self.set_buffer(buffer);
                CommandReturn::failure(error_code)
            }
            Ok(()) => {
                self.current_process.set(process_id);
                CommandReturn::success()
            }
        }
    }

    /// Try to erase an info page.
    ///
    /// # Parameters
    ///
    /// + `raw_info_partition_type`: an integral value representing the type of the info partition
    /// to be erased
    /// + `raw_bank`: an integral value representing the bank to erase from
    /// + `raw_page_number`: the index of the page to be erased
    /// + `process_id`: the ID ofo the process that's trying to erase a page
    ///
    /// # Return value
    ///
    /// + CommandReturn::failure(ErrorCode::INVAL): `raw_info_partition_type`, `raw_bank` or
    /// `raw_page_number` are invalid.
    /// + CommandReturn::failure(ErrorCode::BUSY): the capsule or the peripheral are busy
    /// + CommandReturn::success(): the erase operation has successfully started
    fn raw_erase_info_page(
        &self,
        raw_info_partition_type: usize,
        raw_bank: usize,
        raw_page_number: usize,
        process_id: ProcessId,
    ) -> CommandReturn {
        let info_partition_type: Flash::InfoType = match raw_info_partition_type.try_into() {
            Err(()) => return CommandReturn::failure(ErrorCode::INVAL),
            Ok(info_partition_type) => info_partition_type,
        };

        let bank: Flash::BankType = match raw_bank.try_into() {
            Err(()) => return CommandReturn::failure(ErrorCode::INVAL),
            Ok(bank) => bank,
        };

        self.current_process.set(process_id);

        match self
            .flash
            .erase_info_page(info_partition_type, bank, raw_page_number)
        {
            Err(error_code) => CommandReturn::failure(error_code),
            Ok(()) => CommandReturn::success(),
        }
    }
}

impl<'a, Flash: InfoFlashTrait> InfoClientTrait<Flash> for InfoFlash<'a, Flash> {
    fn info_read_complete(&self, read_buffer: &'static mut Flash::Page, result: Result<(), Error>) {
        self.current_process.take().map(|process_id| {
            self.grant.enter(process_id, |_, kernel_data| {
                if let Err(error) = result {
                    // Ignore the schedule result. There is not much that can be done about that.
                    let _ = kernel_data.schedule_upcall(
                        UpcallId::ReadDone.to_usize(),
                        (
                            kernel::errorcode::into_statuscode(Err(ErrorCode::FAIL)),
                            error as usize,
                            0,
                        ),
                    );

                    return;
                }

                let copy_result = kernel_data
                    .get_readwrite_processbuffer(RwAllowId::Read.to_usize())
                    .and_then(|allowed_buffer| {
                        allowed_buffer.mut_enter(|data| {
                            let read_slice = read_buffer.as_mut();

                            // The allowed buffer must be the same length as a page
                            if data.len() != read_slice.len() {
                                return Err(());
                            }

                            // Copy the read data to the allowed buffer
                            data.copy_from_slice(read_slice);

                            Ok(())
                        })
                    })
                    .ok();

                if let Some(result) = copy_result {
                    let status_code = match result {
                        Err(()) => kernel::errorcode::into_statuscode(Err(ErrorCode::INVAL)),
                        Ok(()) => kernel::errorcode::into_statuscode(Ok(())),
                    };

                    let _ = kernel_data.schedule_upcall(
                        // Ignore the schedule result. There is not much that can be done about that.
                        UpcallId::ReadDone.to_usize(),
                        (status_code, 0, 0),
                    );
                }
            })
        });

        self.set_buffer(read_buffer);
    }

    fn info_write_complete(
        &self,
        write_buffer: &'static mut Flash::Page,
        result: Result<(), Error>,
    ) {
        self.current_process.take().map(|process_id| {
            self.grant.enter(process_id, |_, kernel_data| {
                if let Err(error) = result {
                    // Ignore the schedule result. There is not much that can be done about that.
                    let _ = kernel_data.schedule_upcall(
                        UpcallId::WriteDone.to_usize(),
                        (
                            kernel::errorcode::into_statuscode(Err(ErrorCode::FAIL)),
                            error as usize,
                            0,
                        ),
                    );
                } else {
                    // Ignore the schedule result. There is not much that can be done about that.
                    let _ = kernel_data.schedule_upcall(
                        UpcallId::WriteDone.to_usize(),
                        (kernel::errorcode::into_statuscode(Ok(())), 0, 0),
                    );
                }
            })
        });

        self.set_buffer(write_buffer);
    }

    fn info_erase_complete(&self, result: Result<(), Error>) {
        self.current_process.take().map(|process_id| {
            self.grant.enter(process_id, |_, kernel_data| {
                match result {
                    Err(error) => {
                        // Ignore the schedule result. There is not much that can be done about
                        // that.
                        let _ = kernel_data.schedule_upcall(
                            UpcallId::EraseDone.to_usize(),
                            (
                                kernel::errorcode::into_statuscode(Err(ErrorCode::FAIL)),
                                error as usize,
                                0,
                            ),
                        );
                    }
                    Ok(()) => {
                        // Ignore the schedule result. There is not much that can be done about
                        // that.
                        let _ = kernel_data.schedule_upcall(
                            UpcallId::EraseDone.to_usize(),
                            (kernel::errorcode::into_statuscode(Ok(())), 0, 0),
                        );
                    }
                }
            })
        });
    }
}

/// Provide an interface for userland.
impl<Flash: InfoFlashTrait> SyscallDriver for InfoFlash<'static, Flash> {
    fn command(
        &self,
        command_number: usize,
        argument1: usize,
        argument2: usize,
        processid: ProcessId,
    ) -> CommandReturn {
        let command = match Command::try_from(command_number) {
            Ok(command) => command,
            Err(()) => return CommandReturn::failure(ErrorCode::NOSUPPORT),
        };

        match command {
            Command::DriverExistence => CommandReturn::success(),
            Command::ReadPage => {
                let raw_info_partition_type =
                    Self::convert_command_argument_to_raw_info_partition_type(argument1);
                let raw_bank = Self::convert_command_argument_to_raw_bank(argument1);
                let raw_page_number = argument2;
                self.raw_read_info_page(
                    raw_info_partition_type,
                    raw_bank,
                    raw_page_number,
                    processid,
                )
            }
            Command::WritePage => {
                let raw_info_partition_type =
                    Self::convert_command_argument_to_raw_info_partition_type(argument1);
                let raw_bank = Self::convert_command_argument_to_raw_bank(argument1);
                let raw_page_number = argument2;
                self.raw_write_info_page(
                    raw_info_partition_type,
                    raw_bank,
                    raw_page_number,
                    processid,
                )
            }
            Command::ErasePage => {
                let raw_info_partition_type =
                    Self::convert_command_argument_to_raw_info_partition_type(argument1);
                let raw_bank = Self::convert_command_argument_to_raw_bank(argument1);
                let raw_page_number = argument2;
                self.raw_erase_info_page(
                    raw_info_partition_type,
                    raw_bank,
                    raw_page_number,
                    processid,
                )
            }
        }
    }

    fn allocate_grant(&self, process_id: ProcessId) -> Result<(), kernel::process::Error> {
        self.grant.enter(process_id, |_, _| {})
    }
}
