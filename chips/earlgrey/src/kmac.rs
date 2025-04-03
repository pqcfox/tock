// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

use crate::registers::kmac_regs::KmacRegisters;
use crate::registers::top_earlgrey::KMAC_BASE_ADDR;
use kernel::utilities::StaticRef;

pub const KMAC_BASE: StaticRef<KmacRegisters> =
    unsafe { StaticRef::new(KMAC_BASE_ADDR as *const KmacRegisters) };
