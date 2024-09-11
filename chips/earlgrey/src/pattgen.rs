// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.
//
// This license header is required for submitting to upstream Tock.
// It is up to ZeroRISC to decide if this header should be here or not.

use kernel::utilities::StaticRef;
pub use lowrisc::pattgen::PattGen;
use lowrisc::registers::pattgen_regs::PattgenRegisters;

use crate::registers::top_earlgrey::PATTGEN_BASE_ADDR;

pub const PATTGEN_BASE: StaticRef<PattgenRegisters> =
    unsafe { StaticRef::new(PATTGEN_BASE_ADDR as *const PattgenRegisters) };
