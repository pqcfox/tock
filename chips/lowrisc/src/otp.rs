// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.
//
// This license header is required for submitting to upstream Tock.
// It is up to ZeroRISC to decide if this header should be here or not.

use kernel::utilities::helpers::create_non_zero_usize;

use crate::registers::otp_ctrl_regs::{
    OtpCtrlRegisters, CHECK_REGWEN, DIRECT_ACCESS_ADDRESS, DIRECT_ACCESS_CMD,
    OTP_CTRL_PARAM_DEVICE_ID_OFFSET, OTP_CTRL_PARAM_DEVICE_ID_SIZE,
    OTP_CTRL_PARAM_EN_SRAM_IFETCH_OFFSET, OTP_CTRL_PARAM_HW_CFG0_DIGEST_OFFSET,
    OTP_CTRL_PARAM_HW_CFG1_DIGEST_OFFSET, OTP_CTRL_PARAM_MANUF_STATE_OFFSET,
    OTP_CTRL_PARAM_MANUF_STATE_SIZE, STATUS,
};

use kernel::utilities::registers::interfaces::{ReadWriteable, Readable, Writeable};
use kernel::utilities::StaticRef;
use kernel::ErrorCode;

use core::num::{NonZeroU32, NonZeroUsize};

/// Number of possible errors
const NUMBER_ERRORS: NonZeroUsize = create_non_zero_usize(14);
/// Mask for STATUS register to determine whether an error occurred
const MASK_ERRORS: NonZeroUsize = create_non_zero_usize((1 << NUMBER_ERRORS.get()) - 1);
/// One past maximum OTP address
const MAX_PAST_OTP_ADDRESS: NonZeroUsize = create_non_zero_usize(2048);
/// Size of u32
const SIZE_U32: NonZeroUsize = create_non_zero_usize(core::mem::size_of::<u32>());

/// Address of a 32-bit word
#[derive(Clone, Copy)]
pub struct OtpAddress32(usize);

/// Returned when a raw OTBN address does not fit in the 10-bite address space
/// or is not properly aligned.
#[derive(Debug)]
pub struct OtpAddressError;

/// Returned when an OTBN peripheral initialization fails.
#[derive(Debug)]
pub struct OtpInitError;

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
    /// + Err(OtpAddressError): if `raw_address` does not fit in the 10-bit
    ///   address space or is not properly aligned.
    pub const fn new(raw_address: usize) -> Result<Self, OtpAddressError> {
        // CAST: u32 == usize on RV32I
        if raw_address >= MAX_PAST_OTP_ADDRESS.get() || raw_address & 0b11 != 0 {
            Err(OtpAddressError)
        } else {
            Ok(Self(raw_address))
        }
    }

    /// Create a new OTP address for a 32-bit word.
    ///
    /// # Parameters
    ///
    /// + `raw_address`: the value that should represent an OTP address
    ///
    /// # Return value
    ///
    /// The new OTP 32-bit word address
    ///
    /// # Panic
    ///
    /// Panics if `raw_address` is invalid. See [new] for more details.
    const fn new_or_panic(raw_address: usize) -> Self {
        match Self::new(raw_address) {
            Err(_) => panic!("Attempted to create OtpAddress32 with invalid value"),
            Ok(otp_address32) => otp_address32,
        }
    }

    /// Convert the [OtpAddress32] to a 32-bit unsigned number
    ///
    /// # Return value
    ///
    /// The underlying 32-bit unsigned number.
    pub const fn into_u32(self) -> u32 {
        // CAST: u32 == usize on RV32I
        self.0 as u32
    }

    /// Returns the next [OtpAddress32]
    ///
    /// # Return value
    ///
    /// + Some(address): the next [OtpAddress32]
    /// + None: the current OTP address is the last [OtpAddress32]
    pub const fn next(self) -> Option<Self> {
        let next_raw_address = self.into_u32() as usize + SIZE_U32.get();
        match Self::new(next_raw_address) {
            Ok(next_address) => Some(next_address),
            Err(_) => None,
        }
    }
}

/// A range of [OtpAddress32]
struct OtpAddress32Range {
    current_address: OtpAddress32,
    current_index: usize,
    count: NonZeroUsize,
}

impl OtpAddress32Range {
    /// [OtpAddress32Range] constructor
    ///
    /// # Parameters
    ///
    /// + `otp_address`: starting OTP address
    /// + `count`: the number of OTP addresses the range should include
    fn new(otp_address: OtpAddress32, count: NonZeroUsize) -> Self {
        Self {
            current_address: otp_address,
            current_index: 0,
            count,
        }
    }
}

impl Iterator for OtpAddress32Range {
    type Item = OtpAddress32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_index == self.count.get() {
            None
        } else {
            let current_address = self.current_address;
            self.current_address = self.current_address.next()?;
            self.current_index += 1;
            Some(current_address)
        }
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
    const fn new(raw_address: usize) -> Result<Self, ()> {
        // CAST: u32 == usize on RV32I
        if raw_address >= MAX_PAST_OTP_ADDRESS.get() || raw_address & 0b111 != 0 {
            Err(())
        } else {
            Ok(Self(raw_address))
        }
    }

    /// Create a new OTP address for a 64-bit word.
    ///
    /// # Parameters
    ///
    /// + `raw_address`: the value that should represent an OTP address
    ///
    /// # Return value
    ///
    /// The new OTP 64-bit word address
    ///
    /// # Panic
    ///
    /// Panics if `raw_address` is invalid. See [new] for more details.
    const fn new_or_panic(raw_address: usize) -> Self {
        match Self::new(raw_address) {
            Err(()) => panic!("Attempted to create OtpAddress64 with invalid value"),
            Ok(otp_address64) => otp_address64,
        }
    }

    /// Convert the [OtpAddress64] to a 32-bit unsigned number
    ///
    /// # Return value
    ///
    /// The underlying 32-bit unsigned number.
    const fn into_u32(self) -> u32 {
        // CAST: u32 == usize on RV32I
        self.0 as u32
    }
}

/// The starting address of device ID
const DEVICE_ID_FIELD_ADDRESS: OtpAddress32 =
    OtpAddress32::new_or_panic(OTP_CTRL_PARAM_DEVICE_ID_OFFSET);
/// The size of the device ID field in bytes
const DEVICE_ID_FIELD_SIZE: NonZeroUsize =
    // CAST: u32 == usize on RV32I
    create_non_zero_usize(OTP_CTRL_PARAM_DEVICE_ID_SIZE as usize);
/// The starting address of MANUF_STATE field
const MANUF_STATE_FIELD_ADDRESS: OtpAddress32 =
    OtpAddress32::new_or_panic(OTP_CTRL_PARAM_MANUF_STATE_OFFSET);
/// The size of the MANUF_STATE field in bytes
const MANUF_STATE_FIELD_SIZE: NonZeroUsize =
    // CAST: u32 == usize on RV32I
    create_non_zero_usize(OTP_CTRL_PARAM_MANUF_STATE_SIZE as usize);
/// The starting address of EN_SRAM_IFETCH field
const EN_SRAM_IFETCH_FIELD_ADDRESS: OtpAddress32 =
    OtpAddress32::new_or_panic(OTP_CTRL_PARAM_EN_SRAM_IFETCH_OFFSET);
/// The starting address of EN_CSRNG_SW_APP_READ field
// EN_CSRNG_SW_APP_READ belongs to the same OTP word as EN_SRAM_IFETCH
const EN_CSRNG_SW_APP_READ_FIELD_ADDRESS: OtpAddress32 = EN_SRAM_IFETCH_FIELD_ADDRESS;
/// The starting address of EN_ENTROPY_SRC_FW_READ field
// EN_ENTROPY_SRC_FW_READ belongs to the same OTP word as EN_SRAM_IFETCH
const EN_ENTROPY_SRC_FW_READ_FIELD_ADDRESS: OtpAddress32 = EN_SRAM_IFETCH_FIELD_ADDRESS;
/// The starting address of EN_ENTROPY_SRC_FW_OVER
// EN_ENTROPY_SRC_FW_OVER belongs to the same OPT word as EN_SRAM_IFETCH
const EN_ENTROPY_SRC_FW_OVER_FIELD_ADDRESS: OtpAddress32 = EN_SRAM_IFETCH_FIELD_ADDRESS;
/// The starting address of HW_CFG_DIGEST field
const HW_CFG0_DIGEST_FIELD_ADDRESS: OtpAddress64 =
    OtpAddress64::new_or_panic(OTP_CTRL_PARAM_HW_CFG0_DIGEST_OFFSET);
const HW_CFG1_DIGEST_FIELD_ADDRESS: OtpAddress64 =
    OtpAddress64::new_or_panic(OTP_CTRL_PARAM_HW_CFG1_DIGEST_OFFSET);

/// OTP peripheral driver
pub struct Otp {
    registers: StaticRef<OtpCtrlRegisters>,
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
        Self { registers }
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
        status_value & (MASK_ERRORS.get() as u32) != 0
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
    fn set_check_timeout(&self, timeout: Option<NonZeroU32>) {
        match timeout {
            Some(timeout) => self.registers.check_timeout.set(timeout.get()),
            None => self.registers.check_timeout.set(0),
        }
    }

    /// Locks access to INTEGRITY_CHECK_PERIOD and CONSISTENCY_CHECK_PERIOD registers.
    fn lock_check_registers(&self) {
        self.registers
            .check_regwen
            .modify(CHECK_REGWEN::CHECK_REGWEN::CLEAR);
    }

    /// Check if INTEGRITY_CHECK_PERIOD and CONSISTENCY_CHECK_PERIOD registers are locked.
    fn are_check_registers_locked(&self) -> bool {
        !self
            .registers
            .check_regwen
            .is_set(CHECK_REGWEN::CHECK_REGWEN)
    }

    /// Check if check timeout is disabled
    fn _is_timeout_disabled(&self) -> bool {
        self.registers.check_timeout.get() == 0
    }

    /// Initialize peripheral
    ///
    /// # Parameters
    ///
    /// + `integrity_check_period`: the maximum period for integrity checks, in
    /// units of multiples of 256 clock cycles. 255 is added to this number to
    /// determine the actual maximum.
    /// + `consistency_check_period`: the maximum period for consistency checks, in
    /// units of multiples of 256 clock cycles. 255 is added to this number to
    /// determine the actual maximum.
    ///
    /// # Return value
    ///
    /// + Ok(()): initialization successful
    /// + Err(()): an error occurred during initialization
    pub fn init(
        &self,
        integrity_check_period: u32,
        consistency_check_period: u32,
        timeout: Option<NonZeroU32>,
    ) -> Result<(), OtpInitError> {
        if self.has_errors() {
            return Err(OtpInitError);
        }
        if !self.are_check_registers_locked() {
            self.set_integrity_check_period(integrity_check_period);
            self.set_consistency_check_period(consistency_check_period);
            self.lock_check_registers();
        }

        self.set_check_timeout(timeout);

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
        self.registers
            .direct_access_address
            .modify(DIRECT_ACCESS_ADDRESS::DIRECT_ACCESS_ADDRESS.val(address.into_u32()));
    }

    /// Set the address for the next 64-bit operation
    fn set_address64(&self, address: OtpAddress64) {
        self.registers
            .direct_access_address
            .modify(DIRECT_ACCESS_ADDRESS::DIRECT_ACCESS_ADDRESS.val(address.into_u32()));
    }

    /// Start a read
    fn start_read(&self) {
        self.registers
            .direct_access_cmd
            .modify(DIRECT_ACCESS_CMD::RD::SET);
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
        ((self.registers.direct_access_rdata[0].get() as u64) << 32)
            + self.registers.direct_access_rdata[1].get() as u64
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

    /// Read a field of 8 32-bit words.
    ///
    /// # Parameters
    ///
    /// + `starting_address`: the starting address of the field
    ///
    /// # Return value
    ///
    /// + Ok([u8; 32]): the field in big endian
    /// + Err(ErrorCode): an error occurred during reading
    fn read_field_32bytes(&self, starting_address: OtpAddress32) -> Result<[u8; 32], ErrorCode> {
        const FIELD_SIZE_IN_WORDS: NonZeroUsize = create_non_zero_usize(32 / SIZE_U32.get());

        let address_range = OtpAddress32Range::new(starting_address, FIELD_SIZE_IN_WORDS);

        let mut field = [0u8; 32];

        for (index, address) in address_range.into_iter().enumerate() {
            let word = self.read_word32(address)?;
            let start_byte_index = index << 2;
            let end_byte_index = start_byte_index + SIZE_U32.get();
            let bytes = word.to_ne_bytes();
            // PANIC: end_byte_index - start_byte_index == SIZE_U32 == 4 == bytes.len()
            field[start_byte_index..end_byte_index].copy_from_slice(&bytes[..]);
        }

        Ok(field)
    }

    /// Read the device ID
    ///
    /// # Return value
    ///
    /// + Ok([u8; DEVICE_ID_FIELD_SIZE.get()]): the read device ID
    /// + Err(ErrorCode): an error occurred during reading
    pub fn read_device_id(&self) -> Result<[u8; DEVICE_ID_FIELD_SIZE.get()], ErrorCode> {
        self.read_field_32bytes(DEVICE_ID_FIELD_ADDRESS)
    }

    /// Read the manufacturer state
    ///
    /// # Return value
    ///
    /// + Ok([u8; MANUF_STATE_FIELD_SIZE.get()]): the read manufacturer state
    /// + Err(ErrorCode): an error occurred during reading
    pub fn read_manuf_state(&self) -> Result<[u8; MANUF_STATE_FIELD_SIZE.get()], ErrorCode> {
        self.read_field_32bytes(MANUF_STATE_FIELD_ADDRESS)
    }

    /// Read the EN_SRAM_IFETCH field
    ///
    /// # Return value
    ///
    /// + Ok(u8): the read EN_SRAM_IFETCH field
    /// + Err(ErrorCode): an error occurred during reading
    pub fn read_en_sram_ifetch(&self) -> Result<u8, ErrorCode> {
        let word = self.read_word32(EN_SRAM_IFETCH_FIELD_ADDRESS)?;
        // The byte index within the 32-bit word representing EN_SRAM_IFETCH
        const EN_SRAM_IFETCH_BYTE_INDEX: usize = 0;
        Ok(word.to_ne_bytes()[EN_SRAM_IFETCH_BYTE_INDEX])
    }

    /// Read the EN_CSRNG_SW_APP_READ field
    ///
    /// # Return value
    ///
    /// + Ok(u8): the read EN_CSRNG_SW_APP_READ field
    /// + Err(ErrorCode): an error occurred during reading
    pub fn read_en_csrng_sw_app_read(&self) -> Result<u8, ErrorCode> {
        let word = self.read_word32(EN_CSRNG_SW_APP_READ_FIELD_ADDRESS)?;
        // The byte index within the 32-bit word representing EN_CSRNG_SW_APP_READ
        const EN_CSRNG_SW_APP_READ_BYTE_INDEX: NonZeroUsize = create_non_zero_usize(1);
        Ok(word.to_ne_bytes()[EN_CSRNG_SW_APP_READ_BYTE_INDEX.get()])
    }

    /// Read the EN_ENTROPY_SRC_FW_READ field
    ///
    /// # Return value
    ///
    /// + Ok(u8): the read EN_ENTROPY_SRC_FW_READ field
    /// + Err(ErrorCode): an error occurred during reading
    pub fn read_en_entropy_src_fw_read(&self) -> Result<u8, ErrorCode> {
        let word = self.read_word32(EN_ENTROPY_SRC_FW_READ_FIELD_ADDRESS)?;
        // The byte index within the 32-bit word representing EN_ENTROPY_SRC_FW_READ
        const EN_ENTROPY_SRC_FW_READ_BYTE_INDEX: NonZeroUsize = create_non_zero_usize(2);
        Ok(word.to_ne_bytes()[EN_ENTROPY_SRC_FW_READ_BYTE_INDEX.get()])
    }

    /// Read the EN_ENTROPY_SRC_FW_OVER field
    ///
    /// # Return value
    ///
    /// + Ok(u8): the read EN_ENTROPY_SRC_FW_OVER field
    /// + Err(ErrorCode): an error occurred during reading
    pub fn read_en_entropy_src_fw_over(&self) -> Result<u8, ErrorCode> {
        let word = self.read_word32(EN_ENTROPY_SRC_FW_OVER_FIELD_ADDRESS)?;
        // The byte index within the 32-bit word representing EN_ENTROPY_SRC_FW_OVER
        const EN_ENTROPY_SRC_FW_OVER_BYTE_INDEX: NonZeroUsize = create_non_zero_usize(3);
        Ok(word.to_ne_bytes()[EN_ENTROPY_SRC_FW_OVER_BYTE_INDEX.get()])
    }

    /// Read the HW_CFG0 partition digest
    ///
    /// # Return value
    ///
    /// + Ok(u64): the read digest
    /// + Err(ErrorCode): an error occurred during reading
    pub fn read_hw_cfg0_digest(&self) -> Result<u64, ErrorCode> {
        self.read_word64(HW_CFG0_DIGEST_FIELD_ADDRESS)
    }

    /// Read the HW_CFG1 partition digest
    ///
    /// # Return value
    ///
    /// + Ok(u64): the read digest
    /// + Err(ErrorCode): an error occurred during reading
    pub fn read_hw_cfg1_digest(&self) -> Result<u64, ErrorCode> {
        self.read_word64(HW_CFG1_DIGEST_FIELD_ADDRESS)
    }
}

/// Tests for OTP
///
/// Usage
/// -----
///
/// Inside the board main file, add the following code before loading processes:
///
/// ```rust,ignore
/// lowrisc::otp::tests::run_all(&peripherals.otp);
/// ```
///
/// In case of an error, the tests will panic and print an error message.
#[cfg(feature = "test_otp")]
pub mod tests {
    use super::*;

    /// Test that INTEGRITY_CHECK_PERIOD register is locked.
    fn test_integrity_check_period_lock(otp: &Otp) {
        kernel::debug!("Starting testing integrity check period lock.");

        let old_integrity_check_period = otp.registers.integrity_check_period.get();
        otp.set_integrity_check_period(1234);
        let new_integrity_check_period = otp.registers.integrity_check_period.get();
        assert_eq!(
            old_integrity_check_period, new_integrity_check_period,
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
            old_consistency_check_period, new_consistency_check_period,
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

        let device_id = otp.read_device_id().expect("Failed to read device ID");

        kernel::debug!("Finished testing reading device ID.");
    }

    /// Test if reading EN_SRAM_IFETCH works
    fn test_read_en_sram_ifetch(otp: &Otp) {
        kernel::debug!("Starting testing reading EN_SRAM_IFETCH.");

        let en_sram_ifetch_id = otp
            .read_en_sram_ifetch()
            .expect("Failed to read EN_SRAM_IFETCH");

        const EXPECTED_EN_SRAM_IFETCH: u8 = 150;

        assert_eq!(
            EXPECTED_EN_SRAM_IFETCH, en_sram_ifetch_id,
            "The read EN_SRAM_IFETCH does not match the expected value"
        );

        kernel::debug!("Finished testing reading EN_SRAM_IFETCH.");
    }

    /// Test if reading the hardware digest works
    fn test_hw_digest(otp: &Otp) {
        kernel::debug!("Starting testing hardware configure digest.");

        let actual_digest = otp
            .read_hw_cfg0_digest()
            .expect("Reading hardware configure digest failed");
        const EXPECTED_DIGEST: u64 = 0x4e723d153038967f;

        assert_eq!(
            EXPECTED_DIGEST, actual_digest,
            "The read hardware configure digest does not match the expected value"
        );

        kernel::debug!("Finished testing hardware configure digest.");
    }

    /// Run all OTP tests
    pub fn run_all(otp: &Otp) {
        kernel::debug!("Starting OTP tests...");

        test_check_registers_lock(otp);
        test_read_device_id(otp);
        test_read_en_sram_ifetch(otp);
        test_hw_digest(otp);

        kernel::debug!("Finished OTP tests. Everything is alright!");
    }
}
