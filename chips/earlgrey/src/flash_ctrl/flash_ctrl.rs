// This license header is required for submitting to upstream Tock.
// It is up to ZeroRISC to decide if this header should be here or not.
//
// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

use super::bank::{BANK_SIZE, NUMBER_OF_BANKS};

use crate::registers::top_earlgrey::{FLASH_CTRL_CORE_BASE_ADDR, FLASH_CTRL_MEM_BASE_ADDR};
use crate::utils;

use core::num::NonZeroUsize;

pub(super) const FLASH_HOST_STARTING_ADDRESS_OFFSET: NonZeroUsize =
    // PANIC: 0x20000000 != 0
    utils::create_non_zero_usize(FLASH_CTRL_MEM_BASE_ADDR);
/// The size of the flash in bytes
pub(super) const FLASH_SIZE: NonZeroUsize = match BANK_SIZE.checked_mul(NUMBER_OF_BANKS) {
    Some(flash_size) => flash_size,
    // BANK_SIZE * NUMBER_OF_BANKS = 512KiB * 2 = 1MiB ==> multiplication does not overflow
    None => unreachable!(),
};
