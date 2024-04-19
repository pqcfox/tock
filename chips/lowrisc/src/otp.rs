// This license header is required for submitting to upstream Tock.
// It is up to ZeroRISC to decide if this header should be here or not.
//
// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

use crate::create_non_zero_usize;

use crate::registers::otp_ctrl_regs::{
    OtpCtrlRegisters, CHECK_REGWEN
};

use kernel::utilities::StaticRef;
use kernel::utilities::registers::interfaces::{Readable, ReadWriteable, Writeable};

use core::num::NonZeroUsize;

const NUMBER_ERRORS: NonZeroUsize = create_non_zero_usize!(14);
const MASK_ERRORS: NonZeroUsize = create_non_zero_usize!((1 << NUMBER_ERRORS.get()) - 1);

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

    /// Check whether an error occurred during peripheral initialization.
    ///
    /// # Return value
    ///
    /// + Ok(()): no errors
    /// + Err(()): the peripheral encountered an error
    fn check_init_errors(&self) -> Result<(), ()> {
        let status_value = self.registers.status.get();
        // CAST: usize == u32 on RV32I
        if status_value & (MASK_ERRORS.get() as u32) != 0 {
            Err(())
        } else {
            Ok(())
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
        self.check_init_errors()?;
        self.set_integrity_check_period(integrity_check_period);
        self.set_consistency_check_period(consistency_check_period);
        // Check timeout doesn't seem to work.
        if false {
            self.set_check_timeout(timeout);
        }
        self.lock_check_registers();

        Ok(())
    }
}
