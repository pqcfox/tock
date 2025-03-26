// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.
//
// This license header is required for submitting to upstream Tock.
// It is up to ZeroRISC to decide if this header should be here or not.

use super::flash_address::FlashAddress;

use crate::registers::flash_ctrl_regs::{
    FLASH_CTRL_PARAM_BYTES_PER_BANK, FLASH_CTRL_PARAM_REG_NUM_BANKS,
    FLASH_CTRL_PARAM_REG_PAGES_PER_BANK,
};
use crate::utils;

use core::num::NonZeroUsize;

/// Number of flash banks
pub(super) const NUMBER_OF_BANKS: NonZeroUsize =
    // CAST: u32 == usize on RISC-V 32-bit platforms.
    // PANIC: 2 != 0
    utils::create_non_zero_usize(FLASH_CTRL_PARAM_REG_NUM_BANKS as usize);
/// Number of data pages per bank
pub const DATA_PAGES_PER_BANK: NonZeroUsize =
    // CAST: u32 == usize on RISC-V 32-bit platforms.
    // PANIC: 256 != 0
    utils::create_non_zero_usize(FLASH_CTRL_PARAM_REG_PAGES_PER_BANK as usize);
/// The size of a bank
pub(super) const BANK_SIZE: NonZeroUsize =
    // CAST: u32 == usize on RISC-V 32-bit platforms.
    // PANIC: 512KiB != 0
    utils::create_non_zero_usize(FLASH_CTRL_PARAM_BYTES_PER_BANK as usize);

/// The starting address of bank 0 in the flash address space
pub(super) const BANK0_STARTING_FLASH_ADDRESS: FlashAddress =
    // SAFETY: 0x0 is a valid flash address
    unsafe { FlashAddress::new_unchecked(0) };
/// The starting address of bank 1 in the flash address space
pub(super) const BANK1_STARTING_FLASH_ADDRESS: FlashAddress =
    // Bank1 is immediatelly after Bank0
    //
    // SAFETY:
    //
    // + BANK_SIZE fits in isize
    // + BANK0_FLASH_ADDRESS_OFFSET + BANK_SIZE = 0x0 + 512KiB = 0x80000 which is a valid flash
    // address.
    unsafe { BANK0_STARTING_FLASH_ADDRESS.add_unchecked(BANK_SIZE.get()) };

/// List of all banks present on OpenTitan
#[derive(Clone, Copy, Debug)]
pub enum Bank {
    Bank0,
    Bank1,
}

impl TryFrom<usize> for Bank {
    type Error = ();

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Bank0),
            1 => Ok(Self::Bank1),
            _ => Err(()),
        }
    }
}
