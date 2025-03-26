// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

use crate::registers::sram_ctrl_regs;
use crate::registers::sram_ctrl_regs::SramCtrlRegisters;
use crate::registers::top_earlgrey::SRAM_CTRL_RET_AON_REGS_BASE_ADDR;
use core::cell::Cell;
use kernel::hil::retention_ram;
use kernel::utilities::{
    registers::interfaces::{ReadWriteable, Readable},
    StaticRef,
};
use kernel::ErrorCode;

#[cfg(feature = "test_sram_ret")]
use {
    crate::rstmgr::RstMgr, core::fmt::Write, kernel::utilities::target_test, lowrisc::uart::Uart,
};

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum DrvState {
    Uninitialized,
    InitializedScrambled,
    InitializedScrambledDefault,
    Error,
}

#[used]
#[link_section = ".rram_creator"]
static mut RET_RAM_CREATOR: [u32; 512] = [0; 512];

#[used]
#[link_section = ".rram_owner"]
static mut RET_RAM_OWNER: [u32; 512] = [0; 512];

const MULTI_BIT_BOOL_4TRUE: u32 = 0x6;
const MULTI_BIT_BOOL_4FALSE: u32 = 0x9;

pub struct SramCtrl {
    registers: StaticRef<SramCtrlRegisters>,
    cached_state: Cell<DrvState>,
}

pub const SRAM_RET_BASE: StaticRef<SramCtrlRegisters> =
    unsafe { StaticRef::new(SRAM_CTRL_RET_AON_REGS_BASE_ADDR as *const SramCtrlRegisters) };

impl SramCtrl {
    pub fn new() -> Self {
        // Initialize the SRAM controller
        let local_self = Self {
            registers: SRAM_RET_BASE,
            cached_state: Cell::new(DrvState::Uninitialized),
        };
        local_self.cached_state.set(local_self.get_state());
        local_self
    }
    /// This function _forces_ the reinitialization of the init. Normally,this should not be necessary, but
    /// in case it is, we copy the rram data, we init and then restore the rram data.
    pub fn forced_safe_init(&self) -> Result<(), ErrorCode> {
        unsafe {
            let ram_creator_backup = RET_RAM_CREATOR;
            let ram_owner_backup = RET_RAM_OWNER;
            match self.reinit_ram() {
                Ok(()) => {
                    RET_RAM_CREATOR = ram_creator_backup;
                    RET_RAM_OWNER = ram_owner_backup;
                    Ok(())
                }
                _ => Err(ErrorCode::FAIL),
            }
        }
    }

    /// This is a function used to initialize the rram
    /// This WILL delete all ret sram data.
    ///
    /// The return is a Result<(), ErrorCode> because it can fail depending on the regwen state.
    pub fn reinit_ram(&self) -> Result<(), ErrorCode> {
        if self.is_locked_ctrl() {
            return Err(ErrorCode::FAIL);
        }
        // We are aware that the manual says we should initialize the scrambling key, but the scrambling
        // circuitry seems not to be hooked to the AON domain and the scrambled key will be lost after a
        // reset. Therefore, the only way we can get consistent data is if we use the "default" OTP key.
        //
        self.registers
            .ctrl
            .modify(sram_ctrl_regs::CTRL::INIT.val(1));

        while !self
            .registers
            .status
            .is_set(sram_ctrl_regs::STATUS::INIT_DONE)
        {
            // Wait for the key to be valid before proceeding
        }
        self.cached_state.set(self.get_state());
        Ok(())
    }

    /// Interface to read rram data from the creator area. Addressed through ID's and returning u32 data.
    pub fn get_creator_rram_data(&self, id: usize) -> Result<u32, ErrorCode> {
        unsafe {
            if id <= RET_RAM_CREATOR.len() {
                // Use get_unchecked in order to prevent doing a double len check, to make it a bit faster.
                Ok(*RET_RAM_CREATOR.get_unchecked(id))
            } else {
                Err(ErrorCode::SIZE)
            }
        }
    }
    /// Interface to read rram data from the owner area. Addressed through ID's and returning u32 data.
    pub fn get_owner_rram_data(&self, id: usize) -> Result<u32, ErrorCode> {
        unsafe {
            if id <= RET_RAM_OWNER.len() {
                // Use get_unchecked in order to prevent doing a double len check, to make it a bit faster.
                Ok(*RET_RAM_OWNER.get_unchecked(id))
            } else {
                Err(ErrorCode::SIZE)
            }
        }
    }

    /// Interface to read rram data from the owner area. Addressed through ID's and returning u32 data.
    pub fn set_owner_rram_data(&self, id: usize, val: u32) -> Result<(), ErrorCode> {
        unsafe {
            if id <= RET_RAM_OWNER.len() {
                RET_RAM_OWNER[id] = val;
                Ok(())
            } else {
                Err(ErrorCode::SIZE)
            }
        }
    }

    /// Get the current state of the driver.
    pub fn get_state(&self) -> DrvState {
        let reg = self.registers.status.extract();
        if reg.is_set(sram_ctrl_regs::STATUS::BUS_INTEG_ERROR)
            || reg.is_set(sram_ctrl_regs::STATUS::ESCALATED)
            || reg.is_set(sram_ctrl_regs::STATUS::INIT_ERROR)
        {
            DrvState::Error
        } else if reg.is_set(sram_ctrl_regs::STATUS::INIT_DONE) {
            if reg.is_set(sram_ctrl_regs::STATUS::SCR_KEY_SEED_VALID)
                && reg.is_set(sram_ctrl_regs::STATUS::SCR_KEY_VALID)
            {
                DrvState::InitializedScrambled
            } else {
                DrvState::InitializedScrambledDefault
            }
        } else {
            DrvState::Uninitialized
        }
    }

    /// Set the execution rights on rram. The allow_exec represents if the execution is
    /// allowed ('true') or not ('false')
    pub fn set_execution_rights(&self, allow_exec: bool) -> Result<(), ErrorCode> {
        if self.is_locked_exec() {
            return Err(ErrorCode::FAIL);
        }

        if allow_exec {
            self.registers
                .exec
                .modify(sram_ctrl_regs::EXEC::EN.val(MULTI_BIT_BOOL_4TRUE));
        } else {
            self.registers
                .exec
                .modify(sram_ctrl_regs::EXEC::EN.val(MULTI_BIT_BOOL_4FALSE));
        };
        Ok(())
    }

    /// Locks the registers controlling the execution rights. Once set, it can not be reset only by resetting the system.
    pub fn lock_exec(&self) {
        self.registers
            .exec_regwen
            .modify(sram_ctrl_regs::EXEC_REGWEN::EXEC_REGWEN.val(0b0));
    }

    /// Checks if the registers controlling the execution rights are locked.
    pub fn is_locked_exec(&self) -> bool {
        !self
            .registers
            .exec_regwen
            .is_set(sram_ctrl_regs::EXEC_REGWEN::EXEC_REGWEN)
    }

    /// Locks the access to modify the control registers. Once set, it can not be reset only by resetting the system.
    pub fn lock_ctrl(&self) {
        self.registers
            .ctrl_regwen
            .modify(sram_ctrl_regs::CTRL_REGWEN::CTRL_REGWEN.val(0b0));
    }

    /// Checks if the control registers are locked.
    pub fn is_locked_ctrl(&self) -> bool {
        !self
            .registers
            .ctrl_regwen
            .is_set(sram_ctrl_regs::CTRL_REGWEN::CTRL_REGWEN)
    }

    /// Test function. It runs on target self-test, returns if the test suite failed or passed.
    #[cfg(feature = "test_sram_ret")]
    pub fn test(
        &self,
        // test: EarlGreyDefaultPeripherals<ChipConfig, BoardPinmuxLayout>,
        reset_manager: &RstMgr,
        uart: &'static Uart,
    ) -> bool {
        let mut test_runner = target_test::TestRunner::new();
        let binding = |foo: &str| uart.transmit_sync(foo.as_bytes());
        test_runner.set_print_func(&binding);
        test_runner
            .write_str("Starting sram_ret self-test \r\n")
            .unwrap();
        match self.get_creator_rram_data(1) {
            Ok(x) => test_runner
                .write_fmt(format_args!("Reset reason from API is {} \r\n", x))
                .unwrap(),
            _ => test_runner
                .write_str("Wrong init state, can't read reset reason yet!  \r\n")
                .unwrap(),
        }

        let test_cycle: u32;

        if (self.get_creator_rram_data(1).unwrap() == 1)
            || (self.get_owner_rram_data(5).unwrap() > 100)
        {
            let _ = self.set_owner_rram_data(5, 1);
            test_cycle = 0;
            test_runner
                .write_str("Force reset test cycles \r\n")
                .unwrap();
        } else {
            test_cycle = self.get_owner_rram_data(5).unwrap();
            test_runner
                .write_fmt(format_args!("Reset Count is {} \r\n", test_cycle))
                .unwrap();

            let _ = self.set_owner_rram_data(5, test_cycle + 1);
        }

        match test_cycle {
            0 => {
                test_runner.assert_function("Test init status!", || {
                    self.get_state() == DrvState::InitializedScrambled
                });
                test_runner.assert_function("Test no lock on execution!", || {
                    !self.is_locked_exec()
                        && (self
                            .registers
                            .exec_regwen
                            .read(sram_ctrl_regs::EXEC_REGWEN::EXEC_REGWEN)
                            == 0x1)
                });
                test_runner.assert_function("Test set execution rights ON!", || {
                    self.set_execution_rights(true) == Ok(())
                        && (self.registers.exec.read(sram_ctrl_regs::EXEC::EN)
                            == MULTI_BIT_BOOL_4TRUE)
                });
                test_runner.assert_function("Test set execution rights OFF!", || {
                    self.set_execution_rights(false) == Ok(())
                        && (self.registers.exec.read(sram_ctrl_regs::EXEC::EN)
                            == MULTI_BIT_BOOL_4FALSE)
                });
                test_runner.assert_function("Test lock execution!", || {
                    self.lock_exec();
                    self.is_locked_exec()
                        && (self
                            .registers
                            .exec_regwen
                            .read(sram_ctrl_regs::EXEC_REGWEN::EXEC_REGWEN)
                            == 0x0)
                });
                test_runner.assert_function("Test fail set execution rights!", || {
                    self.set_execution_rights(true) == Err(ErrorCode::FAIL)
                });
                test_runner.assert_function("Test no lock on Ctrl!", || {
                    !self.is_locked_ctrl()
                        && (self
                            .registers
                            .ctrl_regwen
                            .read(sram_ctrl_regs::CTRL_REGWEN::CTRL_REGWEN)
                            == 0x1)
                });
                test_runner.assert_function("Test setter for rram id's!", || {
                    self.set_owner_rram_data(10, 0xFF) == Ok(())
                        && self.get_owner_rram_data(10).unwrap() == 0xFF
                });
                test_runner.assert_function("Test 2nd setter for rram id's!", || {
                    self.set_owner_rram_data(10, 0x5A) == Ok(())
                        && self.get_owner_rram_data(10).unwrap() == 0x5A
                });
                test_runner.assert(
                    "Test forced Safe Init!",
                    self.forced_safe_init() == Ok(())
                        && (self.get_state() == DrvState::InitializedScrambledDefault)
                        && (self.get_owner_rram_data(10).unwrap() == 0x5A),
                );
                test_runner.assert_function("Test rram data survived!", || {
                    self.get_owner_rram_data(10).unwrap() == 0x5A
                });
                test_runner.assert_function("Test lock execution!", || {
                    self.lock_ctrl();
                    self.is_locked_ctrl()
                        && (self
                            .registers
                            .ctrl_regwen
                            .read(sram_ctrl_regs::CTRL_REGWEN::CTRL_REGWEN)
                            == 0x0)
                });

                test_runner.assert("Check if is Locked Ctrl.", self.is_locked_ctrl() == true);
                test_runner.assert(
                    "Test fail Safe Init because we're locked",
                    self.forced_safe_init() == Err(ErrorCode::FAIL),
                );
                test_runner.assert_function("Test rram data setters ID 10 before reset!", || {
                    self.set_owner_rram_data(10, 0x5A) == Ok(())
                        && self.get_owner_rram_data(10).unwrap() == 0x5A
                });
                test_runner.assert_function("Test rram data setters ID 11 before reset!", || {
                    self.set_owner_rram_data(11, 0xFEEDBEEF) == Ok(())
                        && self.get_owner_rram_data(11).unwrap() == 0xFEEDBEEF
                });
                test_runner.assert_function("Test rram data setters ID 12 before reset!", || {
                    self.set_owner_rram_data(12, 0x5A5A5A5A) == Ok(())
                        && self.get_owner_rram_data(12).unwrap() == 0x5A5A5A5A
                });
                test_runner.write_str("Performing reset!  \r\n").unwrap();

                reset_manager.do_software_reset();
            }
            1 => {
                test_runner.assert_function("Test rram data ID 10 survived!", || {
                    self.get_owner_rram_data(10).unwrap() == 0x5A
                });
                test_runner.assert_function("Test rram data ID 11 survived!", || {
                    self.get_owner_rram_data(11).unwrap() == 0xFEEDBEEF
                });
                test_runner.assert_function("Test rram data ID 12 survived!", || {
                    self.get_owner_rram_data(12).unwrap() == 0x5A5A5A5A
                });
            }
            _ => {}
        }

        if test_runner.is_test_failed {
            test_runner.write_str("sram_ret self-test FAILED").unwrap();
        } else {
            test_runner.write_str("sram_ret self-test PASSED").unwrap();
        }
        test_runner.is_test_failed
    }
}

impl retention_ram::OwnerRetentionRam for SramCtrl {
    type Data = u32;
    type ID = usize;

    fn read(&self, id: Self::ID) -> Result<Self::Data, ErrorCode> {
        self.get_owner_rram_data(id)
    }

    fn write(&self, id: Self::ID, data: Self::Data) -> Result<(), ErrorCode> {
        self.set_owner_rram_data(id, data)
    }
}

impl retention_ram::CreatorRetentionRam for SramCtrl {
    type Data = u32;
    type ID = usize;

    fn read(&self, id: Self::ID) -> Result<Self::Data, ErrorCode> {
        self.get_creator_rram_data(id)
    }
}
