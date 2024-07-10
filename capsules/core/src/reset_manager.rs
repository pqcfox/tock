// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

//! Provides userspace access to determine reset reason, further reset information and trigger a MCU reset
//!
//! Syscall Interface
//! -----------------
//!
//! - Stability: 0
//!
//! ### Command
//!
//! Read reset reason, reset information or trigger a MCU reset.
//!
//! #### `command_num`
//!
//! - `0`: Driver existence check
//! - `1`: Return reason for MCU reset
//! - `2`: Trigger immediate MCU reset
//! - `3`: Get further crash info from previous crash

use kernel::grant::{AllowRoCount, AllowRwCount, Grant, UpcallCount};
use kernel::hil::reset_managment::{ResetManagment, ResetReason};
use kernel::processbuffer::ReadableProcessBuffer;
use kernel::processbuffer::WriteableProcessBuffer;
use kernel::syscall::{CommandReturn, SyscallDriver};
use kernel::utilities::cells::OptionalCell;
use kernel::{ErrorCode, ProcessId};

/// Ids for read-write allow buffers
mod rw_allow {
    pub const RESET_INFO_DUMP: usize = 0;
    /// The number of allow buffers the kernel stores for this grant
    pub const COUNT: u8 = 1;
}

pub const DRIVER_NUM: usize = crate::driver::NUM::ResetManager as usize;
/// Manages MCU level resets (reset reason, trigger reset)
pub struct ResetManager<'a, M: ResetManagment> {
    hw: &'a M,
    reset_reason: OptionalCell<ResetReason>,
    reset_info_dump: OptionalCell<M::ResetInfo>,
    grants: Grant<(), UpcallCount<0>, AllowRoCount<0>, AllowRwCount<{ rw_allow::COUNT }>>,
}

impl<'a, M: ResetManagment> ResetManager<'a, M> {
    pub fn new(
        hw: &'a M,
        grants: Grant<(), UpcallCount<0>, AllowRoCount<0>, AllowRwCount<{ rw_allow::COUNT }>>,
    ) -> Self {
        Self {
            hw,
            reset_reason: OptionalCell::empty(),
            reset_info_dump: OptionalCell::empty(),
            grants,
        }
    }

    /// change the reset reason if `reset_reason` is not set or if the `reset_reason` is ResetReason::Unknown(_)
    pub fn populate_reset_reason(&self, reason: Option<ResetReason>) {
        if let Some(extracted_new_reason) = reason {
            match self.reset_reason.get() {
                Some(ResetReason::Unknown(_)) | None => self.reset_reason.set(extracted_new_reason),
                _ => {}
            };
        }
    }

    /// save reset info from HW registers
    pub fn startup(&self) {
        // try to read the reset reason from HW
        self.reset_reason.insert(self.hw.reset_reason());
        // try to read the reset info dump
        self.reset_info_dump.insert(self.hw.get_reset_info_dump());
    }
}

impl<'a, M: ResetManagment> SyscallDriver for ResetManager<'a, M> {
    /// Read reset reason, reset information or trigger a MCU reset.
    ///
    ///
    /// ### `command_num`
    ///
    /// - `0`: Driver existence check
    /// - `1`: Return reason for MCU reset
    ///         - on success it returns success_u32_u32 according to `serialize_resetreason`
    /// - `2`: Trigger immediate MCU reset
    /// - `3`: Get further crash info from previous crash
    ///
    fn command(
        &self,
        command_num: usize,
        _: usize,
        _: usize,
        processid: ProcessId,
    ) -> CommandReturn {
        match command_num {
            0 => CommandReturn::success(),
            // get reset reason, on success return success_u32_u32 according to `fn serialize_resetreason`
            1 => match self.reset_reason.get() {
                Some(reset_reason) => {
                    let a = serialize_reset_reason(reset_reason);
                    CommandReturn::success_u32_u32(u32::from(a.0), u32::from(a.1))
                }
                None => CommandReturn::failure(ErrorCode::NOACK),
            },

            // trigger system reset
            2 => {
                self.hw.trigger_system_reset();
                // if the system executes this line it means the reset didn't work so we return FAIL
                CommandReturn::failure(ErrorCode::FAIL)
            }
            // get further crash info from previous crash
            3 => {
                if let Some(dump_info) = self.reset_info_dump.get() {
                    let a = self.grants.enter(processid, |_, kernel_data| {
                        kernel_data
                            .get_readwrite_processbuffer(rw_allow::RESET_INFO_DUMP)
                            .and_then(|buffer| {
                                // compare the length (in bytes) of the buffer to the length (in bytes) of the reset info dump
                                let buffer_length = buffer.len();
                                let dump_length = dump_info.as_ref().len() * 4;
                                if buffer_length < dump_length {
                                    return Err(kernel::process::Error::OutOfMemory);
                                }

                                buffer.mut_enter(|buffer_data| {
                                    for (i, _) in dump_info.as_ref().iter().enumerate() {
                                        // WritableProcessSlice can't be trusted at compile time to have enough space to contain `dump_info`. Manually copy each byte in Little Endian Order (ARM native, RISC-V native)
                                        let value = dump_info.as_ref()[i].to_le_bytes();

                                        buffer_data[i * 4 + 0].set(value[0]);
                                        buffer_data[i * 4 + 1].set(value[1]);
                                        buffer_data[i * 4 + 2].set(value[2]);
                                        buffer_data[i * 4 + 3].set(value[3]);
                                    }
                                    // fill with 0s the rest of the buffer
                                    for index in dump_length..buffer_length {
                                        buffer_data[index].set(0);
                                    }
                                })
                            })
                    });

                    // the above procedure returns a Result<Result, k::p::Error>,k::p::Error>. The following line flattens the Result and maps the k::p::Error into ErrorCode that can be passes to userspace
                    let c = a.unwrap_or_else(|b| Err(b)).map_err(|error| error.into());
                    CommandReturn::from(c)
                } else {
                    CommandReturn::failure(ErrorCode::NOACK)
                }
            }
            _ => CommandReturn::failure(ErrorCode::ALREADY),
        }
    }

    fn allocate_grant(&self, processid: ProcessId) -> Result<(), kernel::process::Error> {
        self.grants.enter(processid, |_, _| {})
    }
}

/// pass-through implementation for `ResetManagment` trait. Used by capsules to access reset information.
impl<'a, M: ResetManagment> ResetManagment for ResetManager<'a, M> {
    type ResetInfo = M::ResetInfo;

    fn reset_reason(&self) -> Option<ResetReason> {
        self.reset_reason.get()
    }

    fn get_reset_info_dump(&self) -> Option<Self::ResetInfo> {
        self.reset_info_dump.get()
    }

    fn trigger_system_reset(&self) {
        //TODO: should other capsules be able to trigger a system reset?
        self.hw.trigger_system_reset();
    }
}

fn serialize_reset_reason(reason: ResetReason) -> (u16, u16) {
    match reason {
        ResetReason::PowerOnReset => (0, 0),
        ResetReason::LowPowerExit => (1, 0),
        ResetReason::SoftwareRequest => (2, 0),
        ResetReason::SoftwareFault => (3, 0),
        ResetReason::Debug => (4, 0),
        ResetReason::HardwareLine => (5, 0),
        ResetReason::Watchdog => (6, 0),
        ResetReason::VoltageFault => (7, 0),
        ResetReason::PeripheralFault(extra) => (8, extra),
        ResetReason::PeripheralRequest(extra) => (9, extra),
        ResetReason::Unknown(extra) => (10, extra),
    }
}
