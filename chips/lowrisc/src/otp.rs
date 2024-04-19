// This license header is required for submitting to upstream Tock.
// It is up to ZeroRISC to decide if this header should be here or not.
//
// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

use crate::create_non_zero_usize;

use crate::registers::otp_ctrl_regs::{
    OtpCtrlRegisters, CHECK_REGWEN, DIRECT_ACCESS_ADDRESS, DIRECT_ACCESS_CMD, STATUS,
};

use kernel::ErrorCode;
use kernel::utilities::StaticRef;
use kernel::utilities::registers::interfaces::{Readable, ReadWriteable, Writeable};

use core::num::NonZeroUsize;

const NUMBER_ERRORS: NonZeroUsize = create_non_zero_usize!(14);
const MASK_ERRORS: NonZeroUsize = create_non_zero_usize!((1 << NUMBER_ERRORS.get()) - 1);
const MAX_PAST_OTP_ADDRESS: NonZeroUsize = create_non_zero_usize!(2048);

/// Address of a 32-bit word
pub struct OtpAddress32(u32);

impl OtpAddress32 {
    /// Create a new OTP address for a 32-bit word.
    ///
    /// # Parameters
    ///
    /// + `raw_address`: the value that should represent an OTP address
    ///
    /// # Return value
    ///
    /// + Ok(Self): the OTP address
    /// + Err(()): if `raw_address` does not fit in the 10-bit address space and is not properly
    /// aligned.
    pub const fn new(raw_address: u32) -> Result<Self, ()> {
        // CAST: u32 == usize on RV32I
        if raw_address >= MAX_PAST_OTP_ADDRESS.get() as u32 || raw_address & 0b11 != 0 {
            Err(())
        } else {
            Ok(Self(raw_address))
        }
    }

    /// Convert the [OtpAddress32] to a 32-bit unsigned number
    ///
    /// # Return value
    ///
    /// The underlying 32-bit unsigned number.
    pub const fn as_u32(self) -> u32 {
        self.0
    }
}

pub struct Otp {
    registers: StaticRef<OtpCtrlRegisters>
}

impl Otp {
    /// OTP constructor
    ///
    /// # Parameters:
    ///
    /// + `registers`: OTP registers
    ///
    /// # Return value
    ///
    /// A new instance of [Otp]
    pub fn new(registers: StaticRef<OtpCtrlRegisters>) -> Self {
        Self {
            registers
        }
    }

    /// Check whether an error occurred.
    ///
    /// # Return value
    ///
    /// + Ok(()): no errors
    /// + Err(()): the peripheral encountered an error
    fn has_errors(&self) -> bool {
        let status_value = self.registers.status.get();
        // CAST: usize == u32 on RV32I
        if status_value & (MASK_ERRORS.get() as u32) != 0 {
            true
        } else {
            false
        }
    }

    /// Sets the maximum period that can be generated for integrity checks.
    ///
    /// # Parameters
    ///
    /// + `period`: the upper bits of a 40-bit value. The lower 8 bits are set to 1.
    fn set_integrity_check_period(&self, period: u32) {
        self.registers.integrity_check_period.set(period);
    }

    /// Sets the maximum period that can be generated for consistency checks.
    ///
    /// # Parameters
    ///
    /// + `period`: the upper bits of a 40-bit value. The lower 8 bits are set to 1.
    fn set_consistency_check_period(&self, period: u32) {
        self.registers.consistency_check_period.set(period);
    }

    /// Sets integrity and consistency check timeout.
    ///
    /// # Parameters
    ///
    /// + `timeout`: timeout value in CPU cycles
    fn set_check_timeout(&self, timeout: u32) {
        self.registers.check_timeout.set(timeout);
    }

    /// Locks access to INTEGRITY_CHECK_PERIOD and CONSISTENCY_CHECK_PERIOD registers.
    fn lock_check_registers(&self) {
        self.registers.check_regwen.modify(CHECK_REGWEN::CHECK_REGWEN::Clear);
    }

    /// Initialize peripheral
    ///
    /// # Parameters
    ///
    /// + `integrity_check_period`: the maximum period for integrity checks. See
    /// [set_integrity_check_period] for more details.
    /// + `consistency_check_period`: the maximum period for consistency checks. See
    /// [set_consistency_check_period] for more details.
    ///
    /// # Return value
    ///
    /// + Ok(()): initialization successful
    /// + Err(()): an error occurred during initialization
    pub fn init(
        &self,
        integrity_check_period: u32,
        consistency_check_period: u32,
        timeout: u32
    ) -> Result<(), ()> {
        if self.has_errors() {
            return Err(());
        }
        self.set_integrity_check_period(integrity_check_period);
        self.set_consistency_check_period(consistency_check_period);
        // Check timeout doesn't seem to work.
        if false {
            self.set_check_timeout(timeout);
        }
        self.lock_check_registers();

        Ok(())
    }

    /// Check whether the peripheral is busy
    ///
    /// # Return value
    ///
    /// + false: the peripheral is idle
    /// + true: the peripheral is busy
    fn is_busy(&self) -> bool {
        !self.registers.status.is_set(STATUS::DAI_IDLE)
    }

    /// Set the address for the next operation
    fn set_address32(&self, address: OtpAddress32) {
        self.registers.direct_access_address.modify(DIRECT_ACCESS_ADDRESS::DIRECT_ACCESS_ADDRESS.val(address.as_u32()));
    }

    /// Start a read
    fn start_read(&self) {
        self.registers.direct_access_cmd.modify(DIRECT_ACCESS_CMD::RD::SET);
    }

    /// Get a 32-bit word from DIRECT_ACCESS_RDATA register
    ///
    /// # Return value
    ///
    /// The bottom half of DIRECT_ACCESS_RDATA
    fn get_word32(&self) -> u32 {
        self.registers.direct_access_rdata[0].get()
    }

    /// Read a 32-bit word.
    ///
    /// # Parameters
    ///
    /// + `address`: the address of the word to be read
    ///
    /// # Return value
    ///
    /// + Ok(u32): the read value
    /// + Err(ErrorCode): an error occurred:
    ///     + ErrorCode::BUSY: the peripheral is busy
    ///     + ErrorCode::FAIL: the read operation failed
    pub fn read_word32(&self, address: OtpAddress32) -> Result<u32, ErrorCode> {
        if self.is_busy() {
            return Err(ErrorCode::BUSY);
        }

        self.set_address32(address);
        self.start_read();

        while self.is_busy() {}

        if self.has_errors() {
            return Err(ErrorCode::FAIL);
        }

        Ok(self.get_word32())
    }
}
