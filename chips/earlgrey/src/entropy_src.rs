// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

use crate::registers::entropy_src_regs::EntropySrcRegisters;
use crate::registers::top_earlgrey::ENTROPY_SRC_BASE_ADDR;
use kernel::utilities::StaticRef;

pub const ENTROPY_SRC_BASE: StaticRef<EntropySrcRegisters> =
    unsafe { StaticRef::new(ENTROPY_SRC_BASE_ADDR as *const EntropySrcRegisters) };
