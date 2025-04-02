// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

//! Support for the AES hardware block on OpenTitan
//!
//! <https://docs.opentitan.org/hw/ip/aes/doc/>

use crate::registers::top_earlgrey::AES_BASE_ADDR;
use kernel::utilities::StaticRef;
use lowrisc::registers::aes_regs::AesRegisters;

// https://docs.opentitan.org/hw/top_earlgrey/doc/
pub const AES_BASE: StaticRef<AesRegisters> =
    unsafe { StaticRef::new(AES_BASE_ADDR as *const AesRegisters) };
