// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.
//
// Copyright zeroRISC Inc.
// Confidential information of zeroRISC Inc. All rights reserved.

use kernel::utilities::StaticRef;
use lowrisc::registers::rv_core_ibex_regs::RvCoreIbexRegisters;
pub use lowrisc::rv_core_ibex::{RvCoreIbex, IBEX_EXTERNAL_NMI_MCAUSE};

use crate::registers::top_earlgrey::RV_CORE_IBEX_CFG_BASE_ADDR;

pub const RV_CORE_IBEX_BASE: StaticRef<RvCoreIbexRegisters> =
    unsafe { StaticRef::new(RV_CORE_IBEX_CFG_BASE_ADDR as *const RvCoreIbexRegisters) };

pub static mut RV_CORE_IBEX: RvCoreIbex = RvCoreIbex::new(RV_CORE_IBEX_CFG_BASE_ADDR);
