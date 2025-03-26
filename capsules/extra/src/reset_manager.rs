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

pub const DRIVER_NUM: usize = capsules_core::driver::NUM::ResetManager as usize;
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

// This test code works only on OpenTitan as it needs a `RetentionRAM`.
// This test can't be done in `chips/earlgrey/src/rstmgr.rs` because that driver doesn't know the actual ResetReason, as it is obtained by this capsule from RetentionRAM, not from RstMgr registers.
#[cfg(feature = "test_resetmanager")]
pub mod test {
    use core::panic::Location;

    use kernel::hil::{
        reset_managment::{ResetManagment, ResetReason},
        retention_ram::OwnerRetentionRam,
    };

    /// test state saved in Retention RAM between resets
    #[repr(u32)]
    #[derive(Clone, Copy, Debug)]
    enum TestState {
        Started = 0xAA998877,
        SoftwareResetCheck = 0xAA665544,
        Finished = 0xAA332211,
    }

    impl TryFrom<u32> for TestState {
        type Error = u32;
        fn try_from(value: u32) -> Result<Self, Self::Error> {
            match value {
                0xAA998877 => Ok(Self::Started),
                0xAA665544 => Ok(Self::SoftwareResetCheck),
                0xAA332211 => Ok(Self::Finished),
                _ => Err(value),
            }
        }
    }

    // alias for access to Owner section of RetentionRam
    type TestStateInterface = dyn OwnerRetentionRam<Data = u32, ID = usize>;

    // this function will panic if it can't read RetentionRAM or the RetentionRAM data is invalid
    #[track_caller]
    fn get_state(retention: &TestStateInterface) -> TestState {
        let value = TestState::try_from(
            retention
                .read(59)
                .expect("Retention RAM failed, could not get state"),
        );
        match value {
            Ok(state) => state,
            Err(raw_value) => {
                panic!(
                    "Invalid value {:x} found in Retention RAM when called from {:?}",
                    raw_value,
                    Location::caller()
                )
            }
        }
    }

    // this function will panic if it can't write to RetentionRAM
    fn save_state(retention: &TestStateInterface, test_state: TestState) {
        retention
            .write(59, test_state as u32)
            .expect("Retention RAM failed, could not save state")
    }

    /// try to reset the processor and check that the reset info dump contains a valid program counter
    /// RetentionRam is used for storing the state of the test as code execution is stopped and restarted after the reset
    /// ```ignore
    ///  # State machine
    ///
    /// | test state         |   next test state   | expected reset reaason |  actions    | checks        |
    /// ---------------------------------------------------------------------------------------------------
    /// | Started            | SoftwareResetCheck  |    PowerOnReset        | reset mcu   |               |
    /// | SoftwareResetCheck | Finished            |    SoftwareRequested   | test passed | cpu dump info |
    /// | Finished           | Finished            |          ------        | nothing     |               |
    ///
    /// ```
    pub fn test_software_reset<T: ResetManagment>(
        retention: &TestStateInterface,
        reset_manager: &T,
        rom_start: usize,
        rom_end: usize,
    ) {
        //if this is a fresh start, save (in RRAM) that the test started
        //if this function fails (returns None) than the test is compromised
        let reset_reason = reset_manager
            .reset_reason()
            .expect("could not retrieve reset reason");

        // retrieve the test state
        let mut test_state = if reset_reason == ResetReason::PowerOnReset {
            // if it's a fresh start, write `Started` into RetentionRAM
            save_state(retention, TestState::Started);
            TestState::Started
        } else {
            // if it's not a fresh start it means that the state is in RetentionRAM, read it
            get_state(retention)
        };

        match test_state {
            // 1: it's a fresh start, trigger a reset and in the next cycle check that the correct reset reason was detected
            TestState::Started => {
                assert_eq!(
                    reset_reason,
                    ResetReason::PowerOnReset,
                    "this state is valid only if this is a fresh start"
                );
                // store next state
                test_state = TestState::SoftwareResetCheck;
                save_state(retention, test_state);

                reset_manager.trigger_system_reset();
            }

            //2: Step 1 triggered a software reset, check that it was detected
            TestState::SoftwareResetCheck => {
                assert_eq!(
                    reset_reason,
                    ResetReason::SoftwareRequest,
                    "should have arrived in this state only if a software reset was triggered"
                );

                // check that the reset info dump contains the address of the instruction that generated the system reset (check that in it somewhere in ROM)
                let binding = reset_manager.get_reset_info_dump().unwrap();
                let reset_info_dump = binding.as_ref();

                // reset info contains 2 sections:
                // * cpu_info [1word for 'length' + multiple words of cpu reset info]
                // * alert_info [1 word for 'length + multiple words of alert info]

                // cpu_info will contain in the third word the address of the program counter that generated a reset
                let cpu_info_length = reset_info_dump[0];
                assert!(cpu_info_length > 3, "not enough cpu crash info");
                let reset_genereating_address = reset_info_dump[2] as usize;
                assert!(
                    (rom_start..rom_end).contains(&reset_genereating_address),
                    "the instruction that genereated the reset did not come from FLASH/ROM",
                );

                test_state = TestState::Finished;
                save_state(retention, test_state);

                kernel::debug!("TEST ResetManager test_software_reset PASS");
            }
            TestState::Finished => {}
        }
    }
}
