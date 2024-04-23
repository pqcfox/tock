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

/// Number of possible errors
const NUMBER_ERRORS: NonZeroUsize = create_non_zero_usize!(14);
/// Mask for STATUS register to determine whether an error occurred
const MASK_ERRORS: NonZeroUsize = create_non_zero_usize!((1 << NUMBER_ERRORS.get()) - 1);
/// One past maximum OTP address
const MAX_PAST_OTP_ADDRESS: NonZeroUsize = create_non_zero_usize!(2048);

/// Address of a 32-bit word
pub struct OtpAddress32(usize);

impl OtpAddress32 {
    /// Create a new OTP address for a 32-bit word.
    ///
    /// # Parameters
    ///
    /// + `raw_address`: the value that should represent an OTP address
    ///
    /// # Return value
    ///
    /// + Ok(Self): the OTP address if `raw_address` is valid
    /// + Err(()): if `raw_address` does not fit in the 10-bit address space and is not properly
    /// aligned.
    pub const fn new(raw_address: usize) -> Result<Self, ()> {
        // CAST: u32 == usize on RV32I
        if raw_address >= MAX_PAST_OTP_ADDRESS.get() || raw_address & 0b11 != 0 {
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
        // CAST: u32 == usize on RV32I
        self.0 as u32
    }
}

/// Address of a 64-bit word
pub struct OtpAddress64(usize);

impl OtpAddress64 {
    /// Create a new OTP address for a 64-bit word.
    ///
    /// # Parameters
    ///
    /// + `raw_address`: the value that should represent an OTP address
    ///
    /// # Return value
    ///
    /// + Ok(Self): the OTP address if `raw_address` is valid
    /// + Err(()): if `raw_address` does not fit in the 10-bit address space and is not properly
    /// aligned.
    pub const fn new(raw_address: usize) -> Result<Self, ()> {
        // CAST: u32 == usize on RV32I
        if raw_address >= MAX_PAST_OTP_ADDRESS.get() || raw_address & 0b111 != 0 {
            Err(())
        } else {
            Ok(Self(raw_address))
        }
    }

    /// Convert the [OtpAddress64] to a 32-bit unsigned number
    ///
    /// # Return value
    ///
    /// The underlying 32-bit unsigned number.
    pub const fn as_u32(self) -> u32 {
        // CAST: u32 == usize on RV32I
        self.0 as u32
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
        self.registers.check_regwen.modify(CHECK_REGWEN::CHECK_REGWEN::CLEAR);
    }

    /// Check if INTEGRITY_CHECK_PERIOD and CONSISTENCY_CHECK_PERIOD registers are locked.
    fn are_check_registers_locked(&self) -> bool {
        !self.registers.check_regwen.is_set(CHECK_REGWEN::CHECK_REGWEN)
    }

    /// Check if check timeout is disabled
    fn is_timeout_disabled(&self) -> bool {
        self.registers.check_timeout.get() == 0
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
        if !self.are_check_registers_locked() {
            self.set_integrity_check_period(integrity_check_period);
            self.set_consistency_check_period(consistency_check_period);
            self.lock_check_registers();
        }
        if self.is_timeout_disabled() {
            self.set_check_timeout(timeout);
        }

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

    /// Set the address for the next 32-bit operation
    fn set_address32(&self, address: OtpAddress32) {
        self.registers.direct_access_address.modify(
            DIRECT_ACCESS_ADDRESS::DIRECT_ACCESS_ADDRESS.val(address.as_u32())
        );
    }

    /// Set the address for the next 64-bit operation
    fn set_address64(&self, address: OtpAddress64) {
        self.registers.direct_access_address.modify(
            DIRECT_ACCESS_ADDRESS::DIRECT_ACCESS_ADDRESS.val(address.as_u32())
        );
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

    /// Get a 64-bit word from DIRECT_ACCESS_RDATA register
    ///
    /// # Return value
    ///
    /// The bottom half of DIRECT_ACCESS_RDATA
    fn get_word64(&self) -> u64 {
        ((self.registers.direct_access_rdata[0].get() as u64) << 32) +
            self.registers.direct_access_rdata[1].get() as u64
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

    /// Read a 64-bit word.
    ///
    /// # Parameters
    ///
    /// + `address`: the address of the word to be read
    ///
    /// # Return value
    ///
    /// + Ok(u64): the read value
    /// + Err(ErrorCode): an error occurred:
    ///     + ErrorCode::BUSY: the peripheral is busy
    ///     + ErrorCode::FAIL: the read operation failed
    pub fn read_word64(&self, address: OtpAddress64) -> Result<u64, ErrorCode> {
        if self.is_busy() {
            return Err(ErrorCode::BUSY);
        }

        self.set_address64(address);
        self.start_read();

        while self.is_busy() {}

        if self.has_errors() {
            return Err(ErrorCode::FAIL);
        }

        Ok(self.get_word64())
    }
}

pub mod tests {
    use super::*;

    /// Test that INTEGRITY_CHECK_PERIOD register is locked.
    fn test_integrity_check_period_lock(otp: &Otp) {
        kernel::debug!("Starting testing integrity check period lock.");

        let old_integrity_check_period = otp.registers.integrity_check_period.get();
        otp.set_integrity_check_period(1234);
        let new_integrity_check_period = otp.registers.integrity_check_period.get();
        assert_eq!(
            old_integrity_check_period,
            new_integrity_check_period,
            "Integrity check period must be immutable after lock"
        );

        kernel::debug!("Finished testing integrity check period lock.");
    }

    /// Test that CONSISTENCY_CHECK_PERIOD register is locked.
    fn test_consistency_check_period_lock(otp: &Otp) {
        kernel::debug!("Starting testing consistency check period lock.");

        let old_consistency_check_period = otp.registers.consistency_check_period.get();
        otp.set_consistency_check_period(1234);
        let new_consistency_check_period = otp.registers.consistency_check_period.get();
        assert_eq!(
            old_consistency_check_period,
            new_consistency_check_period,
            "Integrity check period must be immutable after lock"
        );

        kernel::debug!("Finished testing consistency check period lock.");
    }

    /// Test if check registers are locked
    fn test_check_registers_lock(otp: &Otp) {
        test_integrity_check_period_lock(otp);
        test_consistency_check_period_lock(otp);
    }

    /// Test if reading device ID works
    fn test_read_device_id(otp: &Otp) {
        kernel::debug!("Starting testing reading device ID.");

        const DEVICE_ID_SIZE: usize = 8;
        let mut device_id = [0u32; DEVICE_ID_SIZE];

        const DEVICE_ID_START_ADDRESS: usize = 0x680;
        let mut raw_address = DEVICE_ID_START_ADDRESS;

        for word in &mut device_id {
            let otp_address = OtpAddress32::new(raw_address).expect("Attempting to create invalid OTP address");
            *word = otp.read_word32(otp_address).expect("Reading device ID failed");
            raw_address += core::mem::size_of::<u32>();
        }

        const EXPECTED_DEVICE_ID: [u32; DEVICE_ID_SIZE] =
            [0xBA2A15F5, 0xC5C33741, 0xCA6A93CD, 0x0383A1EE, 0xB11B1215, 0x4DED8AEC, 0x5FE9D22C, 0x064DDF32];
        assert_eq!(
            EXPECTED_DEVICE_ID,
            device_id,
            "The read device ID does not match the expected value"
        );

        kernel::debug!("Finished testing reading device ID.");
    }

    fn test_hw_digest(otp: &Otp) {
        kernel::debug!("Starting testing creator software configure digest.");

        const CREATOR_SW_CFG_DIGEST_ADDRESS: OtpAddress64 = match OtpAddress64::new(0x6C8) {
            Ok(otp_address) => otp_address,
            Err(()) => unreachable!(),
        };

        let actual_digest = otp.read_word64(CREATOR_SW_CFG_DIGEST_ADDRESS)
            .expect("Reading creator software configure digest address failed");
        const EXPECTED_DIGEST: u64 = 0x4e723d153038967f;

        assert_eq!(
            EXPECTED_DIGEST,
            actual_digest,
            "The read creator software configure digest does not match the expected value"
        );

        kernel::debug!("Finished testing creator software configure digest.");
    }

    /// Run all OTP tests
    pub fn run_all(otp: &Otp) {
        kernel::debug!("Starting OTP tests...");

        test_check_registers_lock(otp);
        test_read_device_id(otp);
        test_hw_digest(otp);

        kernel::debug!("Finished OTP tests. Everything is alright!");
    }
}
